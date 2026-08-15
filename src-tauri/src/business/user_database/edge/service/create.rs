use std::collections::{HashMap, HashSet};

use crate::business::user_database::edge::dao;
use crate::business::user_database::entity::{Action, Edge};
use crate::business::user_database::{log, node, state};
use crate::error_code::ErrorCode;

/// 在指定画布内新建一条边：两端节点都必须存在且属于该画布，
/// 两个节点之间不能已存在边，且新建这条边不会在画布内成环（不考虑连接桩）。
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
/// 两个节点之间已存在边时返回 `ErrorCode::EdgeAlreadyExists`，
/// 新建该边会成环时返回 `ErrorCode::EdgeWouldFormCycle`，
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
    log::service::create(
        &edge.id,
        Action::EdgeCreate {
            source_title: source.title,
            target_title: target.title,
        },
    )?;
    Ok(edge)
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
