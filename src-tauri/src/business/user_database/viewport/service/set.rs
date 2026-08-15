use crate::business::user_database::entity::{Viewport, CANVAS_UNIVERSE_VIEWPORT_ID};
use crate::business::user_database::state;
use crate::business::user_database::viewport::dao;
use crate::error_code::ErrorCode;

/// 插入或更新视口：未传入画布 id 时插入或更新画布宇宙中的视口，
/// 传入时插入或更新指定画布中的视口。不产生日志。
///
/// # 参数
/// - `canvas_id`: 可选的画布 id。
/// - `x`: 视口中心的 x 坐标。
/// - `y`: 视口中心的 y 坐标。
/// - `zoom`: 缩放比例。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn set(canvas_id: Option<String>, x: f64, y: f64, zoom: f64) -> Result<(), ErrorCode> {
    let canvas_id = canvas_id.unwrap_or_else(|| CANVAS_UNIVERSE_VIEWPORT_ID.to_string());
    let connection = state::lock_connection();
    dao::upsert(
        &connection,
        &Viewport {
            canvas_id,
            x,
            y,
            zoom,
        },
    )
}
