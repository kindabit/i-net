use crate::business::user_database::attachment::service;
use crate::error_code::ErrorCode;
use crate::util::preprocess_util;

/// 覆盖保存附件内容：用新的内容整体覆盖附件文件并更新附件元数据大小。
///
/// # 参数
/// - `id`: 附件 id。
/// - `content`: 新的附件内容（明文）。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_attachment_update_file(id: String, content: Vec<u8>) -> Result<(), ErrorCode> {
    preprocess(id, content)
}

/// `user_database_attachment_update_file` 的 preprocess 函数：校验 id 后接入 service 层的 update_file 函数。
pub fn preprocess(id: String, content: Vec<u8>) -> Result<(), ErrorCode> {
    let id = preprocess_util::preprocess_attachment_id(id)?;
    service::update_file(&id, &content)
}
