use crate::business::user_database::entity::Action;
use crate::business::user_database::node::dao;
use crate::business::user_database::{log, state};
use crate::error_code::ErrorCode;

/// 修改指定节点的坐标。若新坐标与旧坐标相同，则视为原地移动，直接返回而不更新数据库、不产生日志。
///
/// 产生 NodeMove 日志，载荷为节点标题、旧坐标和新坐标。
///
/// # 参数
/// - `id`: 节点 id。
/// - `x`: 新 x 坐标。
/// - `y`: 新 y 坐标。
///
/// # 返回值
/// 成功时返回 `Ok(())`；节点不存在时返回 `ErrorCode::NoNodeWithSuchId`，
/// 发生其他错误时返回对应的 `ErrorCode`。
pub fn move_node(id: &str, x: f64, y: f64) -> Result<(), ErrorCode> {
    let connection = state::lock_connection();
    let mut node = dao::select_by_id(&connection, id)?
        .ok_or_else(|| ErrorCode::NoNodeWithSuchId { id: id.to_string() })?;
    let old_x = node.x;
    let old_y = node.y;
    // 新坐标与旧坐标相同时视为原地移动，直接返回：不更新数据库，也不产生日志。
    if old_x == x && old_y == y {
        return Ok(());
    }
    node.x = x;
    node.y = y;
    dao::update(&connection, &node)?;
    log::service::create(
        id,
        Action::NodeMove {
            title: node.title,
            old_x,
            old_y,
            new_x: x,
            new_y: y,
        },
    )?;
    Ok(())
}
