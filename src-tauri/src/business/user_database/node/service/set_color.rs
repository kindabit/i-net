use crate::business::user_database::node::dao;
use crate::business::user_database::state;
use crate::error_code::ErrorCode;

/// 设置指定节点的颜色。
///
/// # 参数
/// - `id`: 节点 id。
/// - `color`: 前端序列化的自定义颜色，空串表示使用默认色。
///
/// # 返回值
/// 成功时返回 `Ok(())`；节点不存在时返回 `ErrorCode::NoNodeWithSuchId`，
/// 发生其他错误时返回对应的 `ErrorCode`。
pub fn set_color(id: &str, color: String) -> Result<(), ErrorCode> {
    let connection = state::lock_connection();
    let mut node = dao::select_by_id(&connection, id)?
        .ok_or_else(|| ErrorCode::NoNodeWithSuchId { id: id.to_string() })?;
    node.color = color;
    dao::update(&connection, &node)?;
    Ok(())
}
