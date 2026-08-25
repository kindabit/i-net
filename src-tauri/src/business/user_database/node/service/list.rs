use std::collections::HashSet;

use rusqlite::Connection;

use crate::business::user_database::entity::Node;
use crate::business::user_database::node::dao;
use crate::business::user_database::node::service::shadow_direction;
use crate::business::user_database::node::vo::NodeVO;
use crate::business::user_database::state;
use crate::error_code::ErrorCode;

/// 返回指定画布内的正常节点或者已经逻辑删除的节点（以 NodeVO 形式）。不产生日志。
///
/// 影子节点的展示数据合并为原始节点的值，并附带原始节点状态与影子方向。
///
/// # 参数
/// - `canvas_id`: 画布 id。
/// - `deleted`: 逻辑删除标志，false 返回正常节点，true 返回已逻辑删除的节点。
///
/// # 返回值
/// 返回节点值对象列表；若发生错误则返回对应的 `ErrorCode`。
pub fn list(canvas_id: &str, deleted: bool) -> Result<Vec<NodeVO>, ErrorCode> {
    let connection = state::lock_connection();
    let nodes = dao::select_by_canvas_id_and_deleted(&connection, canvas_id, deleted)?;
    nodes.into_iter().map(|node| to_vo(&connection, node)).collect()
}

/// 将 Node 转换为 NodeVO，影子节点合并原始节点的展示数据。
///
/// 影子链支持嵌套：影子节点的 shadow_id 指向其直接来源节点；展示数据沿影子链
/// 级联向上查到根原始节点（shadow_id 为 None 的节点），合并 title / sub_title /
/// color / canvas_ref_id；原始节点状态以根为准。影子方向只对当前节点的直接来源推导。
/// 用 visited 集合防御脏数据成环：成环时停止向上，以当前节点为准降级处理，
/// 与 shadow_direction 的"宁可放行也不因脏数据锁死用户操作"原则一致。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `node`: 待转换的节点。
///
/// # 返回值
/// 返回转换后的节点值对象；影子链上任一节点的原始节点缺失（数据不一致）时返回
/// `ErrorCode::NoNodeWithSuchId`；数据库错误返回对应的 `ErrorCode`。
fn to_vo(connection: &Connection, node: Node) -> Result<NodeVO, ErrorCode> {
    let Some(first_origin_id) = node.shadow_id.clone() else {
        return Ok(NodeVO { node, shadow_origin_deleted: None, shadow_direction: None });
    };
    let direction = shadow_direction(connection, &node)?;
    // 沿影子链向上查到根原始节点：展示数据与"原始节点已删除"状态以根为准。
    let mut visited: HashSet<String> = HashSet::new();
    let mut cursor = match dao::select_by_id(connection, &first_origin_id)? {
        Some(origin) => origin,
        None => return Err(ErrorCode::NoNodeWithSuchId { id: first_origin_id }),
    };
    while let Some(next_id) = cursor.shadow_id.clone() {
        if !visited.insert(cursor.id.clone()) {
            break;
        }
        match dao::select_by_id(connection, &next_id)? {
            Some(next) => cursor = next,
            None => return Err(ErrorCode::NoNodeWithSuchId { id: next_id }),
        }
    }
    let mut merged = node;
    merged.title = cursor.title;
    merged.sub_title = cursor.sub_title;
    merged.color = cursor.color;
    merged.canvas_ref_id = cursor.canvas_ref_id.clone();
    Ok(NodeVO { node: merged, shadow_origin_deleted: Some(cursor.deleted), shadow_direction: direction })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// to_vo 失败路径：影子节点的 shadow_id 悬空（仅可能由数据污染或外键被关闭造成）时报 NoNodeWithSuchId。
    #[test]
    fn test_to_vo_shadow_missing_origin() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch("PRAGMA foreign_keys = OFF;")
            .unwrap();
        dao::create_table(&connection).unwrap();
        let shadow = Node {
            id: "shadow-1".to_string(),
            canvas_id: "canvas-1".to_string(),
            x: 0.0,
            y: 0.0,
            title: String::new(),
            sub_title: String::new(),
            canvas_ref_id: None,
            deleted: false,
            color: String::new(),
            shadow_id: Some("missing-origin".to_string()),
        };
        dao::insert(&connection, &shadow).unwrap();
        assert!(matches!(
            to_vo(&connection, shadow),
            Err(ErrorCode::NoNodeWithSuchId { .. })
        ));
    }
}
