use crate::business::user_database::viewport::service;
use crate::error_code::ErrorCode;
use crate::util::preprocess_util;

/// 插入或更新视口：未传入画布 id 时插入或更新画布宇宙中的视口，
/// 传入时插入或更新指定画布中的视口。
///
/// # 参数
/// - `canvas_id`: 可选的画布 id。
/// - `x`: 视口中心的 x 坐标。
/// - `y`: 视口中心的 y 坐标。
/// - `zoom`: 缩放比例。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_viewport_set(
    canvas_id: Option<String>,
    x: f64,
    y: f64,
    zoom: f64,
) -> Result<(), ErrorCode> {
    preprocess(canvas_id, x, y, zoom)
}

/// `user_database_viewport_set` 的 preprocess 函数：校验参数后接入 service 层的 set 函数。
pub fn preprocess(canvas_id: Option<String>, x: f64, y: f64, zoom: f64) -> Result<(), ErrorCode> {
    let canvas_id = canvas_id
        .map(preprocess_util::preprocess_canvas_id)
        .transpose()?;
    service::set(canvas_id, x, y, zoom)
}
