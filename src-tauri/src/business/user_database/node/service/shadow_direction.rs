use rusqlite::Connection;

use crate::business::user_database::entity::Node;
use crate::business::user_database::node::vo::ShadowDirection;
use crate::business::user_database::{edge, node::dao};
use crate::error_code::ErrorCode;

/// 推导影子节点在其所在画布内的方向：找到引用影子所在画布的画布节点 B，再判断原始节点 X 与 B 之间边的方向；
/// X→B（X 是源）为入向影子 Inflow，B→X（B 是源）为出向影子 Outflow。
/// 数据不一致（影子没有 shadow_id、找不到 B 或找不到边）时返回 Ok(None)，不报错。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `shadow`: 影子节点。
///
/// # 返回值
/// 返回推导出的方向；数据不一致时返回 None；数据库错误返回对应的 `ErrorCode`。
pub fn shadow_direction(
    connection: &Connection,
    shadow: &Node,
) -> Result<Option<ShadowDirection>, ErrorCode> {
    let Some(origin_id) = shadow.shadow_id.as_deref() else {
        return Ok(None);
    };
    let Some(canvas_node) = dao::select_by_canvas_ref_id(connection, &shadow.canvas_id)? else {
        return Ok(None);
    };
    if edge::dao::exists_between(connection, origin_id, &canvas_node.id)? {
        return Ok(Some(ShadowDirection::Inflow));
    }
    if edge::dao::exists_between(connection, &canvas_node.id, origin_id)? {
        return Ok(Some(ShadowDirection::Outflow));
    }
    Ok(None)
}
