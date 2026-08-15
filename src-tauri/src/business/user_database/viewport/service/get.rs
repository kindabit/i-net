use crate::business::user_database::entity::{Viewport, CANVAS_UNIVERSE_VIEWPORT_ID};
use crate::business::user_database::state;
use crate::business::user_database::viewport::dao;
use crate::error_code::ErrorCode;

/// 查询视口：未传入画布 id 时返回画布宇宙中的视口，
/// 传入时返回目标画布中的视口；视口不存在时返回一个默认值
/// （坐标 (0, 0)，缩放比例 1）。不产生日志。
///
/// # 参数
/// - `canvas_id`: 可选的画布 id。
///
/// # 返回值
/// 返回查询到的视口；若发生错误则返回对应的 `ErrorCode`。
pub fn get(canvas_id: Option<String>) -> Result<Viewport, ErrorCode> {
    let canvas_id = canvas_id.unwrap_or_else(|| CANVAS_UNIVERSE_VIEWPORT_ID.to_string());
    let connection = state::lock_connection();
    let viewport = dao::select_by_canvas_id(&connection, &canvas_id)?.unwrap_or(Viewport {
        canvas_id,
        x: 0.0,
        y: 0.0,
        zoom: 1.0,
    });
    Ok(viewport)
}
