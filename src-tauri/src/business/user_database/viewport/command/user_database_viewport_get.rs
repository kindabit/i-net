use crate::business::user_database::entity::Viewport;
use crate::business::user_database::viewport::service;
use crate::error_code::ErrorCode;
use crate::util::preprocess_util;

/// 查询视口：未传入画布 id 时返回画布宇宙中的视口，
/// 传入时返回目标画布中的视口；视口不存在时返回一个默认值。
///
/// # 参数
/// - `canvas_id`: 可选的画布 id。
///
/// # 返回值
/// 返回查询到的视口；若发生错误则返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_viewport_get(canvas_id: Option<String>) -> Result<Viewport, ErrorCode> {
    preprocess(canvas_id)
}

/// `user_database_viewport_get` 的 preprocess 函数：校验参数后接入 service 层的 get 函数。
pub fn preprocess(canvas_id: Option<String>) -> Result<Viewport, ErrorCode> {
    let canvas_id = canvas_id
        .map(preprocess_util::preprocess_canvas_id)
        .transpose()?;
    service::get(canvas_id)
}
