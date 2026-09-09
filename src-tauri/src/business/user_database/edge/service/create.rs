use std::collections::{HashMap, HashSet};

use rusqlite::Connection;

use crate::business::user_database::edge::dao;
use crate::business::user_database::entity::{Action, Edge, Node};
use crate::business::user_database::shadow::vo::ShadowDirection;
use crate::business::user_database::{log, node, shadow, state};
use crate::error_code::ErrorCode;

/// 在指定画布内新建一条边：两端节点都必须存在且属于该画布，
/// 且新建这条边不会在画布内成环（不考虑连接桩；两端已有边时排除旧边后检查）。
///
/// 新连接规则矩阵：
/// - 普通节点 → 普通节点：仅插边，无影子。
/// - 普通节点 / 入向影子 → 画布节点：在 target 引用的子画布内创建源端根本体的入向影子。
/// - 普通节点 → 出向影子：沿目标影子本体链解析到根本体画布节点，在其引用的子画布内创建源端根本体的入向影子。
/// - 画布节点 → 画布节点 / 出向影子：在 source 引用的子画布内创建目标端根本体的出向影子。
/// - 入向影子 → 普通节点：仅插边，无影子。
///
/// 非法连接：
/// - 画布节点 → 普通节点：`ErrorCode::CanvasToPlainNodeEdge`（避免依赖项散落各画布）。
/// - 入向影子作为目标 / 出向影子作为源：`ErrorCode::InvalidShadowEdge`。
/// - 入向影子 → 出向影子：`ErrorCode::ShadowToShadowEdge`（应在父画布直接连线）。
///
/// 影子生命周期由边控制：影子的 shadow_id 指向产生它的边，删除边时其产生的影子
/// 经 node.shadow_id 外键级联删除，影子的相连边经 edge.source_id/target_id 外键级联删除，
/// 下游嵌套影子沿外键链递归坍塌，应用层不再手动删除影子。
///
/// 重建语义分两段：
/// - 同向已有边（同 (source_id, target_id) 命中旧边）：仅更新旧边的连接桩后直接返回，
///   不删边、不动影子、不做成环检查、不做断连确认、不记任何日志；连接桩完全相同的重复拖线幂等成功。
/// - 反向已有边（同 (target_id, source_id) 命中旧边）：执行"删旧建新"——从边集中排除该旧边后
///   做成环检查；通过后收集旧边关联影子的断连影响，未确认时返回
///   `ErrorCode::EdgeDeleteDisconnectsNodes`，由前端确认后以 `confirmed = true` 重调；
///   确认后删除旧边（其产生的影子经外键级联删除），插入新边（继承旧边的 title 和 description），
///   再按新建流程建影子联动；产生 `Action::EdgeReplace` 日志。
/// - 无旧边：走全新建边流程，产生 `Action::EdgeCreate` 日志。
///
/// # 参数
/// - `canvas_id`: 画布 id。
/// - `source_id`: 源节点 id。
/// - `source_port`: 源节点连接桩。
/// - `target_id`: 目标节点 id。
/// - `target_port`: 目标节点连接桩。
/// - `confirmed`: 调用方已确认反向替换路径中影子删除带来的连接断开影响；同向重建与无旧边流程忽略。
///
/// # 返回值
/// 返回新建或更新后的边；任一节点不存在时返回 `ErrorCode::NoNodeWithSuchId`，
/// 两端连接桩相同时返回 `ErrorCode::EdgeSameNodePort`，
/// 新建该边会成环时返回 `ErrorCode::EdgeWouldFormCycle`，
/// 画布节点作为源连接普通节点时返回 `ErrorCode::CanvasToPlainNodeEdge`，
/// 影子节点互相连接时返回 `ErrorCode::ShadowToShadowEdge`，
/// 影子节点连线方向不合法时返回 `ErrorCode::InvalidShadowEdge`，
/// 反向替换路径中删除旧边会使子画布内的节点失去连接且未确认时返回
/// `ErrorCode::EdgeDeleteDisconnectsNodes`，
/// 发生其他错误时返回对应的 `ErrorCode`。
pub fn create(
    canvas_id: &str,
    source_id: &str,
    source_port: String,
    target_id: &str,
    target_port: String,
    confirmed: bool,
) -> Result<Edge, ErrorCode> {
    let connection = state::lock_connection();
    let source = node::dao::select_by_id(&connection, source_id)?
        .filter(|node| node.canvas_id == canvas_id)
        .ok_or_else(|| ErrorCode::NoNodeWithSuchId {
            id: source_id.to_string(),
        })?;
    let target = node::dao::select_by_id(&connection, target_id)?
        .filter(|node| node.canvas_id == canvas_id)
        .ok_or_else(|| ErrorCode::NoNodeWithSuchId {
            id: target_id.to_string(),
        })?;
    validate_shadow_endpoints(&connection, &source, &target)?;
    if source_port == target_port {
        return Err(ErrorCode::EdgeSameNodePort);
    }
    // 查同向旧边：命中则直接更新连接桩并返回（同向重建只是调整连接位置，
    // 不删边、不动影子、无需成环检查与断连确认、不记日志）。
    if let Some(mut old) = dao::select_between(&connection, source_id, target_id)? {
        dao::update_ports(&connection, &old.id, &source_port, &target_port)?;
        old.source_port = source_port;
        old.target_port = target_port;
        return Ok(old);
    }
    // 查反向旧边（换向替换路径）：UNIQUE(source_id, target_id) 保证至多一行。
    let old_edge = dao::select_between(&connection, target_id, source_id)?;
    // 成环检查：边集排除旧边。
    let edges = dao::select_by_canvas_id(&connection, canvas_id)?;
    let edges: Vec<Edge> = match &old_edge {
        Some(old) => edges.into_iter().filter(|e| e.id != old.id).collect(),
        None => edges,
    };
    if would_form_cycle(&edges, source_id, target_id) {
        return Err(ErrorCode::EdgeWouldFormCycle);
    }
    // 日志载荷的端点标题取展示标题：影子端点的标题落库为空串，须沿产生边链解析根本体标题；
    // 解析须在删除旧边之前完成（影子链解析依赖产生边链完好）。
    let source_title = shadow::service::display_title(&connection, &source)?;
    let target_title = shadow::service::display_title(&connection, &target)?;
    // 替换路径：断连收集与确认拦截 → 删旧边（影子由外键级联）。
    let replaced = old_edge.is_some();
    let mut inherited: Option<(String, String)> = None;
    if let Some(old) = &old_edge {
        // 断连收集须在删除旧边之前完成（收集依赖产生边链完好）。
        let affected = shadow::service::collect_edge_disconnected(&connection, old)?;
        if !affected.is_empty() && !confirmed {
            return Err(ErrorCode::EdgeDeleteDisconnectsNodes { nodes: affected });
        }
        // 删除旧边：其产生的影子经 node.shadow_id 外键级联删除，下游嵌套影子沿
        // edge.source_id/target_id 与 node.shadow_id 外键链递归级联，应用层禁止手写递归删除。
        dao::delete_by_id(&connection, &old.id)?;
        inherited = Some((old.title.clone(), old.description.clone()));
    }
    let (title, description) = inherited.unwrap_or_default();
    let edge = Edge {
        id: uuid::Uuid::new_v4().to_string(),
        canvas_id: canvas_id.to_string(),
        source_id: source_id.to_string(),
        source_port,
        target_id: target_id.to_string(),
        target_port,
        title,
        description,
    };
    dao::insert(&connection, &edge)?;
    shadow::service::create_shadow_for_edge(&connection, &edge, &source, &target)?;
    if replaced {
        // 旧边即新边的反向：旧边源端是新边目标节点，旧边目标端是新边源节点。
        log::service::create(Action::EdgeReplace {
            old_source_title: target_title.clone(),
            old_target_title: source_title.clone(),
            source_title,
            target_title,
        })?;
    } else {
        log::service::create(Action::EdgeCreate {
            source_title,
            target_title,
        })?;
    }
    Ok(edge)
}

/// 校验影子节点参与连线时的约束：影子节点之间不能互相连接；
/// 入向影子只能作为源（只有出度），出向影子只能作为目标（只有入度）；
/// 画布节点不能直接作为源连接普通节点。
///
/// 影子方向必然可推导，推导不出时 shadow_direction 返回 DataCorruption* 错误。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `source`: 源节点。
/// - `target`: 目标节点。
///
/// # 返回值
/// 连线合法时返回 `Ok(())`；画布节点连接普通节点时返回 `ErrorCode::CanvasToPlainNodeEdge`，
/// 影子节点连线方向不合法时返回 `ErrorCode::InvalidShadowEdge`，
/// 两端皆影子节点时返回 `ErrorCode::ShadowToShadowEdge`，
/// 影子方向推导失败时返回对应的 `DataCorruption*` 错误，
/// 发生数据库错误时返回对应的 `ErrorCode`。
fn validate_shadow_endpoints(
    connection: &Connection,
    source: &Node,
    target: &Node,
) -> Result<(), ErrorCode> {
    // 画布节点不能直接作为源连接普通节点（避免目标节点的依赖项散落在各画布中，
    // 应先经子画布中转）。
    if source.canvas_ref_id.is_some() && target.canvas_ref_id.is_none() && target.shadow_id.is_none()
    {
        return Err(ErrorCode::CanvasToPlainNodeEdge);
    }
    // 入向影子（普通节点的影子）只能作为源，不能作为目标。
    if target.shadow_id.is_some()
        && shadow::service::shadow_direction(connection, target)? == ShadowDirection::Inflow
    {
        return Err(ErrorCode::InvalidShadowEdge);
    }
    // 出向影子（画布节点的影子）只能作为目标，不能作为源。
    if source.shadow_id.is_some()
        && shadow::service::shadow_direction(connection, source)? == ShadowDirection::Outflow
    {
        return Err(ErrorCode::InvalidShadowEdge);
    }
    // 能走到这里的双影子组合只剩 入向影子→出向影子：两者的本体（或上级影子）都直接或间接
    // 存在于父画布中，若需要连接应直接在父画布中进行，而不是先接入子画布再连接。
    if source.shadow_id.is_some() && target.shadow_id.is_some() {
        return Err(ErrorCode::ShadowToShadowEdge);
    }
    Ok(())
}

/// 判断在现有边的基础上新建一条从源节点到目标节点的有向边是否会成环：
/// 从目标节点出发沿有向边能够到达源节点（含源节点与目标节点相同的自环情况）即成环。
///
/// # 参数
/// - `edges`: 画布内现有的全部边。
/// - `source_id`: 新边的源节点 id。
/// - `target_id`: 新边的目标节点 id。
///
/// # 返回值
/// 返回新建该边是否会成环的布尔值。
fn would_form_cycle(edges: &[Edge], source_id: &str, target_id: &str) -> bool {
    let mut adjacency: HashMap<&str, Vec<&str>> = HashMap::new();
    for edge in edges {
        adjacency
            .entry(edge.source_id.as_str())
            .or_default()
            .push(edge.target_id.as_str());
    }
    let mut stack = vec![target_id];
    let mut visited = HashSet::new();
    while let Some(current) = stack.pop() {
        if current == source_id {
            return true;
        }
        if !visited.insert(current) {
            continue;
        }
        if let Some(next) = adjacency.get(current) {
            stack.extend(next);
        }
    }
    false
}
