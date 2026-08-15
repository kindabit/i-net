use crate::business::user_database::canvas::dao;
use crate::business::user_database::entity::Action;
use crate::business::user_database::{log, state};
use crate::error_code::ErrorCode;

/// 修改指定画布的坐标。若新坐标与旧坐标相同，则视为原地移动，直接返回而不更新数据库、不产生日志。
///
/// 产生 CanvasMove 日志，载荷内记录画布名称、旧坐标和新坐标。
///
/// # 参数
/// - `id`: 画布 id。
/// - `x`: 新 x 坐标。
/// - `y`: 新 y 坐标。
///
/// # 返回值
/// 成功时返回 `Ok(())`；画布不存在时返回 `ErrorCode::NoCanvasWithSuchId`，
/// 发生其他错误时返回对应的 `ErrorCode`。
pub fn move_canvas(id: &str, x: f64, y: f64) -> Result<(), ErrorCode> {
    let connection = state::lock_connection();
    let mut canvas = dao::select_by_id(&connection, id)?
        .ok_or_else(|| ErrorCode::NoCanvasWithSuchId { id: id.to_string() })?;
    let old_x = canvas.x;
    let old_y = canvas.y;
    // 新坐标与旧坐标相同时视为原地移动，直接返回：不更新数据库，也不产生日志。
    if old_x == x && old_y == y {
        return Ok(());
    }
    canvas.x = x;
    canvas.y = y;
    dao::update(&connection, &canvas)?;
    log::service::create(
        id,
        Action::CanvasMove {
            name: canvas.name,
            old_x,
            old_y,
            new_x: x,
            new_y: y,
        },
    )?;
    Ok(())
}
