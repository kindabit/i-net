use crate::business::user_database::attachment::service;
use crate::error_code::ErrorCode;
use crate::util::preprocess_util;

/// 重命名附件：修改指定附件的文件名（仅元数据，附件文件内容不受影响）。
///
/// # 参数
/// - `id`: 附件 id。
/// - `file_name`: 新文件名。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_attachment_rename(id: String, file_name: String) -> Result<(), ErrorCode> {
    preprocess(id, file_name)
}

/// `user_database_attachment_rename` 的 preprocess 函数：校验参数后接入 service 层的 rename 函数。
pub fn preprocess(id: String, file_name: String) -> Result<(), ErrorCode> {
    let id = preprocess_util::preprocess_attachment_id(id)?;
    let file_name = preprocess_util::preprocess_file_name(file_name)?;
    service::rename(&id, file_name)
}