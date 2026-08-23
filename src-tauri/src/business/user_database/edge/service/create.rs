use std::collections::{HashMap, HashSet};

use rusqlite::Connection;

use crate::business::user_database::edge::dao;
use crate::business::user_database::entity::{Action, Edge, Node};
use crate::business::user_database::node::vo::ShadowDirection;
use crate::business::user_database::{log, node, state};
use crate::error_code::ErrorCode;

/// 在指定画布内新建一条边：两端节点都必须存在且属于该画布，
/// 两个节点之间不能已存在边，且新建这条边不会在画布内成环（不考虑连接桩）。
///
/// 影子节点连线约束：入向影子只能作为源（只有出度），出向影子只能作为目标（只有入度），
/// 影子节点不允许与画布节点相连（避免产生影子的影子）。
///
/// 影子节点联动：target 是画布节点时，原始节点 source 是其引用画布的父，
/// 在引用画布内创建 source 的入向影子；source 是画布节点时，原始节点 target 是其引用画布的子，
/// 在引用画布内创建 target 的出向影子；两端都是画布节点时两个影子都会创建。
///
/// 产生 EdgeCreate 日志，载荷为源节点的标题和目标节点的标题。
///
/// # 参数
/// - `canvas_id`: 画布 id。
/// - `source_id`: 源节点 id。
/// - `source_port`: 源节点连接桩。
/// - `target_id`: 目标节点 id。
/// - `target_port`: 目标节点连接桩。
///
/// # 返回值
/// 返回新建的边；任一节点不存在时返回 `ErrorCode::NoNodeWithSuchId`，
/// 两端连接桩相同时返回 `ErrorCode::EdgeSameNodePort`，
/// 两个节点之间已存在边时返回 `ErrorCode::EdgeAlreadyExists`，
/// 新建该边会成环时返回 `ErrorCode::EdgeWouldFormCycle`，
/// 影子节点连线不合法时返回 `ErrorCode::InvalidShadowEdge`，
/// 发生其他错误时返回对应的 `ErrorCode`。
pub fn create(
    canvas_id: &str,
    source_id: &str,
    source_port: String,
    target_id: &str,
    target_port: String,
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
    if dao::exists_between(&connection, source_id, target_id)? {
        return Err(ErrorCode::EdgeAlreadyExists);
    }
    let edges = dao::select_by_canvas_id(&connection, canvas_id)?;
    if would_form_cycle(&edges, source_id, target_id) {
        return Err(ErrorCode::EdgeWouldFormCycle);
    }
    let edge = Edge {
        id: uuid::Uuid::new_v4().to_string(),
        canvas_id: canvas_id.to_string(),
        source_id: source_id.to_string(),
        source_port,
        target_id: target_id.to_string(),
        target_port,
        title: String::new(),
        description: String::new(),
    };
    dao::insert(&connection, &edge)?;
    if let Some(ref_canvas_id) = &target.canvas_ref_id {
        create_shadow(&connection, ref_canvas_id, &source, ShadowDirection::Inflow)?;
    }
    if let Some(ref_canvas_id) = &source.canvas_ref_id {
        create_shadow(&connection, ref_canvas_id, &target, ShadowDirection::Outflow)?;
    }
    log::service::create(
        &edge.id,
        Action::EdgeCreate {
            source_title: source.title,
            target_title: target.title,
        },
    )?;
    Ok(edge)
}

/// 校验影子节点参与连线时的方向约束：
/// 入向影子只能作为源（只有出度），出向影子只能作为目标（只有入度），
/// 影子节点不允许与画布节点相连（避免产生影子的影子）。
/// 影子方向因数据不一致推导不出时不作限制（宁可放行也不因脏数据锁死用户操作）。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `source`: 源节点。
/// - `target`: 目标节点。
///
/// # 返回值
/// 连线合法时返回 `Ok(())`；不合法时返回 `ErrorCode::InvalidShadowEdge`，
/// 发生数据库错误时返回对应的 `ErrorCode`。
fn validate_shadow_endpoints(
    connection: &Connection,
    source: &Node,
    target: &Node,
) -> Result<(), ErrorCode> {
    if source.shadow_id.is_some() {
        if target.canvas_ref_id.is_some() {
            return Err(ErrorCode::InvalidShadowEdge);
        }
        if node::service::shadow_direction(connection, source)?
            == Some(ShadowDirection::Outflow)
        {
            return Err(ErrorCode::InvalidShadowEdge);
        }
    }
    if target.shadow_id.is_some() {
        if source.canvas_ref_id.is_some() {
            return Err(ErrorCode::InvalidShadowEdge);
        }
        if node::service::shadow_direction(connection, target)?
            == Some(ShadowDirection::Inflow)
        {
            return Err(ErrorCode::InvalidShadowEdge);
        }
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
        if node::service::shadow_direction(connection, existing)? != Some(direction) {
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
