use crate::business::user_database::canvas::service;
use crate::error_code::ErrorCode;
use crate::util::preprocess_util;

/// 修改指定画布的坐标。
///
/// # 参数
/// - `id`: 画布 id。
/// - `x`: 新 x 坐标。
/// - `y`: 新 y 坐标。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_canvas_move_canvas(id: String, x: f64, y: f64) -> Result<(), ErrorCode> {
    preprocess(id, x, y)
}

/// `user_database_canvas_move_canvas` 的 preprocess 函数：校验参数后接入 service 层的 move_canvas 函数。
pub fn preprocess(id: String, x: f64, y: f64) -> Result<(), ErrorCode> {
    let id = preprocess_util::preprocess_canvas_id(id)?;
    service::move_canvas(&id, x, y)
}
