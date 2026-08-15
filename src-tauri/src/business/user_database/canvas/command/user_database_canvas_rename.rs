use crate::business::user_database::canvas::service;
use crate::error_code::ErrorCode;
use crate::util::preprocess_util;

/// 修改指定画布的名称。
///
/// # 参数
/// - `id`: 画布 id。
/// - `name`: 新名称。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_canvas_rename(id: String, name: String) -> Result<(), ErrorCode> {
    preprocess(id, name)
}

/// `user_database_canvas_rename` 的 preprocess 函数：校验参数后接入 service 层的 rename 函数。
pub fn preprocess(id: String, name: String) -> Result<(), ErrorCode> {
    let id = preprocess_util::preprocess_canvas_id(id)?;
    let name = preprocess_util::preprocess_canvas_name(name)?;
    service::rename(&id, name)
}
