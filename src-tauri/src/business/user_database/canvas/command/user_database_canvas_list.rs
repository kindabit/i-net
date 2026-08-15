use crate::business::user_database::canvas::service;
use crate::business::user_database::entity::Canvas;
use crate::error_code::ErrorCode;

/// 返回正常画布或者已经逻辑删除的画布。
///
/// # 参数
/// - `deleted`: 逻辑删除标志，false 返回正常画布，true 返回已逻辑删除的画布。
///
/// # 返回值
/// 返回查询到的画布列表；若发生错误则返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_canvas_list(deleted: bool) -> Result<Vec<Canvas>, ErrorCode> {
    preprocess(deleted)
}

/// `user_database_canvas_list` 的 preprocess 函数：接入 service 层的 list 函数。
pub fn preprocess(deleted: bool) -> Result<Vec<Canvas>, ErrorCode> {
    service::list(deleted)
}
