use std::collections::{HashMap, HashSet};

use rusqlite::Connection;

use crate::business::user_database::edge::dao;
use crate::business::user_database::entity::{Action, Edge, Node};
use crate::business::user_database::node::vo::ShadowDirection;
use crate::business::user_database::{log, node, state};
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
    // 替换路径：断连收集与确认拦截 → 删旧边（影子由外键级联）。
    let mut old_titles: Option<(String, String)> = None;
    let mut inherited: Option<(String, String)> = None;
    if let Some(old) = &old_edge {
        // 断连收集须在删除旧边之前完成（收集依赖产生边链完好）。
        // 旧边即新边的反向：旧边两端节点与新边 source / target 一一对应，标题直接取用即可。
        let affected = node::service::collect_edge_disconnected(&connection, old)?;
        if !affected.is_empty() && !confirmed {
            return Err(ErrorCode::EdgeDeleteDisconnectsNodes { nodes: affected });
        }
        // 删除旧边：其产生的影子经 node.shadow_id 外键级联删除，下游嵌套影子沿
        // edge.source_id/target_id 与 node.shadow_id 外键链递归级联，应用层禁止手写递归删除。
        dao::delete_by_id(&connection, &old.id)?;
        old_titles = Some((target.title.clone(), source.title.clone()));
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
    create_shadow_for_edge(&connection, &edge, &source, &target)?;
    match old_titles {
        Some((old_source_title, old_target_title)) => log::service::create(
            &edge.id,
            Action::EdgeReplace {
                source_title: source.title,
                target_title: target.title,
                old_source_title,
                old_target_title,
            },
        )?,
        None => log::service::create(
            &edge.id,
            Action::EdgeCreate {
                source_title: source.title,
                target_title: target.title,
            },
        )?,
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
        && node::service::shadow_direction(connection, target)? == ShadowDirection::Inflow
    {
        return Err(ErrorCode::InvalidShadowEdge);
    }
    // 出向影子（画布节点的影子）只能作为目标，不能作为源。
    if source.shadow_id.is_some()
        && node::service::shadow_direction(connection, source)? == ShadowDirection::Outflow
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

/// 按连接规则为新建的边联动创建影子节点（不产生影子的连接直接返回）：
/// - 源端是画布节点（画布节点→画布节点 / 画布节点→出向影子）：在源画布节点引用的子画布内
///   创建目标端根本体（必为画布节点）的出向影子；
/// - 目标端是画布节点（普通节点/入向影子→画布节点）：在目标画布节点引用的子画布内创建
///   源端根本体（必为普通节点）的入向影子；
/// - 目标端是出向影子（普通节点→出向影子）：向上查找目标影子的根本体画布节点，
///   在其引用的子画布内创建源端根本体（必为普通节点）的入向影子；
/// - 其余连接（普通→普通、入向影子→普通）不产生影子。
/// 影子的 shadow_id 指向产生它的边，生命周期完全由边控制。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `edge`: 刚插入的新边。
/// - `source`: 边的源节点。
/// - `target`: 边的目标节点。
///
/// # 返回值
/// 成功时返回 `Ok(())`；根本体类型与预期矛盾时返回 `ErrorCode::DataCorruptionShadowRootTypeMismatch`；
/// 产生边链解析失败时返回对应的 `DataCorruption*` 错误；数据库错误返回对应的 `ErrorCode`。
fn create_shadow_for_edge(
    connection: &Connection,
    edge: &Edge,
    source: &Node,
    target: &Node,
) -> Result<(), ErrorCode> {
    if let Some(ref_canvas_id) = &source.canvas_ref_id {
        // 出向影子：本体链在目标端，根本体必须是画布节点。
        let root = node::service::resolve_root(connection, target)?;
        if root.canvas_ref_id.is_none() {
            return Err(ErrorCode::DataCorruptionShadowRootTypeMismatch {
                shadow_id: target.id.clone(),
                root_id: root.id.clone(),
            });
        }
        return create_shadow(connection, ref_canvas_id, &edge.id, ShadowDirection::Outflow);
    }
    if target.canvas_ref_id.is_none() && target.shadow_id.is_none() {
        // 普通→普通、入向影子→普通：不产生影子。
        return Ok(());
    }
    // 入向影子：本体链在源端，根本体必须是普通节点。
    let root = node::service::resolve_root(connection, source)?;
    if root.canvas_ref_id.is_some() {
        return Err(ErrorCode::DataCorruptionShadowRootTypeMismatch {
            shadow_id: source.id.clone(),
            root_id: root.id.clone(),
        });
    }
    // 落点画布：target 是画布节点时取其 canvas_ref_id；target 是出向影子时沿其本体链
    // 找到根本体画布节点，取其 canvas_ref_id。
    let shadow_canvas_id = match &target.canvas_ref_id {
        Some(ref_canvas_id) => ref_canvas_id.clone(),
        None => {
            let target_root = node::service::resolve_root(connection, target)?;
            target_root.canvas_ref_id.clone().ok_or_else(|| {
                ErrorCode::DataCorruptionShadowRootTypeMismatch {
                    shadow_id: target.id.clone(),
                    root_id: target_root.id.clone(),
                }
            })?
        }
    };
    create_shadow(connection, &shadow_canvas_id, &edge.id, ShadowDirection::Inflow)
}

/// 在指定画布内创建由指定产生边产生的影子节点：只有位置与 shadow_id 是影子自己的数据，
/// shadow_id 指向产生该影子的边；title / sub_title / color 以满足非空约束的空串落库。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `canvas_id`: 影子所在画布（被画布节点引用的子画布）的 id。
/// - `edge_id`: 产生该影子的边的 id，写入 shadow_id。
/// - `direction`: 影子方向（入向偏左车道，出向偏右车道）。
///
/// # 返回值
/// 成功时返回 `Ok(())`；发生错误时返回对应的 `ErrorCode`。
fn create_shadow(
    connection: &Connection,
    canvas_id: &str,
    edge_id: &str,
    direction: ShadowDirection,
) -> Result<(), ErrorCode> {
    let (x, y) = shadow_position(connection, canvas_id, direction)?;
    let shadow = Node {
        id: uuid::Uuid::new_v4().to_string(),
        canvas_id: canvas_id.to_string(),
        x,
        y,
        title: String::new(),
        sub_title: String::new(),
        canvas_ref_id: None,
        deleted: false,
        color: String::new(),
        shadow_id: Some(edge_id.to_string()),
    };
    node::dao::insert(connection, &shadow)
}

/// 计算新建影子节点的初始位置（不做网格吸附；吸附由前端在坐标写入后端前完成）：
/// 入向影子放在画布现有非影子内容左侧车道（最小 x - 400），
/// 出向影子放在右侧车道（最大 x + 400）；画布内还没有非影子节点时分别取 0 / 400。
/// 同一画布内已有同方向影子时垂直堆叠在其下方（最大 y + 120），避免影子互相重叠。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `canvas_id`: 影子所在画布的 id。
/// - `direction`: 影子方向。
///
/// # 返回值
/// 返回 (x, y) 坐标；发生数据库错误时返回对应的 `ErrorCode`。
fn shadow_position(
    connection: &Connection,
    canvas_id: &str,
    direction: ShadowDirection,
) -> Result<(f64, f64), ErrorCode> {
    let nodes = node::dao::select_by_canvas_id_and_deleted(connection, canvas_id, false)?;
    let content: Vec<&Node> = nodes.iter().filter(|n| n.shadow_id.is_none()).collect();
    let lane_x = match direction {
        ShadowDirection::Inflow => content
            .iter()
            .map(|n| n.x)
            .reduce(f64::min)
            .map(|min_x| min_x - 400.0)
            .unwrap_or(0.0),
        ShadowDirection::Outflow => content
            .iter()
            .map(|n| n.x)
            .reduce(f64::max)
            .map(|max_x| max_x + 400.0)
            .unwrap_or(400.0),
    };
    // 同方向已有影子的最大 y：逐个推导方向，与新建影子同方向的参与堆叠。
    let mut stack_y: Option<f64> = None;
    for existing in nodes.iter().filter(|n| n.shadow_id.is_some()) {
        if node::service::shadow_direction(connection, existing)? != direction {
            continue;
        }
        stack_y = Some(match stack_y {
            Some(max_y) => f64::max(max_y, existing.y),
            None => existing.y,
        });
    }
    let y = stack_y.map(|max_y| max_y + 120.0).unwrap_or(0.0);
    Ok((lane_x, y))
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
