use crate::business::user_database::canvas::service;
use crate::error_code::ErrorCode;
use crate::util::preprocess_util;

/// 逻辑删除指定画布以及它的全部子孙画布。
///
/// # 参数
/// - `id`: 画布 id。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_canvas_logical_delete(id: String) -> Result<(), ErrorCode> {
    preprocess(id)
}

/// `user_database_canvas_logical_delete` 的 preprocess 函数：校验参数后接入 service 层的 logical_delete 函数。
pub fn preprocess(id: String) -> Result<(), ErrorCode> {
    let id = preprocess_util::preprocess_canvas_id(id)?;
    service::logical_delete(&id)
}
