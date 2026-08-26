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
/// 影子节点连线约束：影子节点之间不能互相连接；入向影子只能作为源（只有出度），
/// 出向影子只能作为目标（只有入度）。
///
/// 画布节点连线约束：画布节点之间不能互相连接（两端都是画布节点时拒绝建边）。
/// 这等价于"只有普通节点才能产生影子节点"——影子的原始节点必须是普通节点。
/// 画布节点与普通节点的连线仍按既有联动规则建影子。
///
/// 影子节点联动：target 是画布节点时，原始节点 source 是其引用画布的父，
/// 在引用画布内创建 source 的入向影子；source 是画布节点时，原始节点 target 是其引用画布的子，
/// 在引用画布内创建 target 的出向影子。
///
/// 重建语义分两段：
/// - 同向已有边（同 (source_id, target_id) 命中旧边）：仅更新旧边的连接桩后直接返回，
///   不删边、不动影子、不做成环检查、不做断连确认、不记任何日志；连接桩完全相同的重复拖线幂等成功。
/// - 反向已有边（同 (target_id, source_id) 命中旧边）：执行"删旧建新"——从边集中排除该旧边后
///   做成环检查；通过后收集旧边关联影子的断连影响，未确认时返回
///   `ErrorCode::EdgeDeleteDisconnectsNodes`，由前端确认后以 `confirmed = true` 重调；
///   确认后删除旧边、删除旧边关联的影子，插入新边（继承旧边的 title 和 description），
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
/// 画布节点之间建边时返回 `ErrorCode::CanvasToCanvasEdge`，
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
    // 画布节点之间不能互相连接：只有普通节点才能产生影子节点（影子的原始节点必须是普通节点）。
    // 此拦截覆盖新建、同向更新、换向替换三条路径，全部在 validate_shadow_endpoints 之前生效。
    if source.canvas_ref_id.is_some() && target.canvas_ref_id.is_some() {
        return Err(ErrorCode::CanvasToCanvasEdge);
    }
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
    // 替换路径：断连收集与确认拦截 → 删旧边 → 删影子。
    let mut old_titles: Option<(String, String)> = None;
    let mut inherited: Option<(String, String)> = None;
    if let Some(old) = &old_edge {
        // 旧边两端节点必然存在（节点物理删除会连带删边）；查不到属于数据污染或程序缺陷，
        // 返回 DataCorruptionEdgeEndpointMissing 由前端受控崩溃；该路径构造脏数据需绕过外键约束，
        // 按设计不单元测试，由代码审查保证。
        let old_source = node::dao::select_by_id(&connection, &old.source_id)?.ok_or_else(
            || ErrorCode::DataCorruptionEdgeEndpointMissing {
                edge_id: old.id.clone(),
                node_id: old.source_id.clone(),
            },
        )?;
        let old_target = node::dao::select_by_id(&connection, &old.target_id)?.ok_or_else(
            || ErrorCode::DataCorruptionEdgeEndpointMissing {
                edge_id: old.id.clone(),
                node_id: old.target_id.clone(),
            },
        )?;
        let shadows = super::shadows_of_edge(&connection, &old_source, &old_target)?;
        // 收集受影响节点（须在删除旧边之前完成，影子方向推导依赖旧边）。
        let mut affected: Vec<String> = Vec::new();
        for shadow in &shadows {
            affected.extend(node::service::collect_shadow_disconnected(
                &connection,
                shadow,
            )?);
        }
        if !affected.is_empty() && !confirmed {
            return Err(ErrorCode::EdgeDeleteDisconnectsNodes { nodes: affected });
        }
        dao::delete_by_id(&connection, &old.id)?;
        // 物理删除影子节点：影子自身相连的边由 edge 外键随节点行的删除级联删除。
        for shadow in &shadows {
            node::dao::delete_by_id(&connection, &shadow.id)?;
        }
        old_titles = Some((old_source.title, old_target.title));
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
    if let Some(ref_canvas_id) = &target.canvas_ref_id {
        create_shadow(&connection, ref_canvas_id, &source, ShadowDirection::Inflow)?;
    }
    if let Some(ref_canvas_id) = &source.canvas_ref_id {
        create_shadow(&connection, ref_canvas_id, &target, ShadowDirection::Outflow)?;
    }
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
/// 入向影子只能作为源（只有出度），出向影子只能作为目标（只有入度）。
/// 影子方向必然可推导，推导不出时 shadow_direction 返回 DataCorruption* 错误。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `source`: 源节点。
/// - `target`: 目标节点。
///
/// # 返回值
/// 连线合法时返回 `Ok(())`；两端皆影子节点时返回 `ErrorCode::ShadowToShadowEdge`，
/// 影子节点连线方向不合法时返回 `ErrorCode::InvalidShadowEdge`，
/// 影子方向推导失败时返回对应的 `DataCorruption*` 错误，
/// 发生数据库错误时返回对应的 `ErrorCode`。
fn validate_shadow_endpoints(
    connection: &Connection,
    source: &Node,
    target: &Node,
) -> Result<(), ErrorCode> {
    // 影子节点之间不能互相连接（先于方向约束判断：无论方向如何都不允许）。
    if source.shadow_id.is_some() && target.shadow_id.is_some() {
        return Err(ErrorCode::ShadowToShadowEdge);
    }
    if source.shadow_id.is_some()
        && node::service::shadow_direction(connection, source)? == ShadowDirection::Outflow
    {
        return Err(ErrorCode::InvalidShadowEdge);
    }
    if target.shadow_id.is_some()
        && node::service::shadow_direction(connection, target)? == ShadowDirection::Inflow
    {
        return Err(ErrorCode::InvalidShadowEdge);
    }
    Ok(())
}

/// 在指定画布内创建指向原始节点的影子节点：只有位置与 shadow_id 是影子自己的数据，
/// title / sub_title / color 以满足非空约束的空串落库。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `canvas_id`: 影子所在画布（被画布节点引用的子画布）的 id。
/// - `origin`: 原始节点。
/// - `direction`: 影子方向（入向偏左车道，出向偏右车道）。
///
/// # 返回值
/// 成功时返回 `Ok(())`；发生错误时返回对应的 `ErrorCode`。
fn create_shadow(
    connection: &Connection,
    canvas_id: &str,
    origin: &Node,
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
        shadow_id: Some(origin.id.clone()),
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
