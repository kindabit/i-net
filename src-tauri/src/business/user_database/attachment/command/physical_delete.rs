use crate::business::user_database::attachment::service;
use crate::error_code::ErrorCode;
use crate::util::preprocess_util;

/// 物理删除附件：删除附件文件并删除附件元数据，不可恢复。
///
/// # 参数
/// - `id`: 附件 id。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_attachment_physical_delete(id: String) -> Result<(), ErrorCode> {
    preprocess(id)
}

/// `user_database_attachment_physical_delete` 的 preprocess 函数：校验 id 后接入 service 层的 physical_delete 函数。
pub fn preprocess(id: String) -> Result<(), ErrorCode> {
    let id = preprocess_util::preprocess_attachment_id(id)?;
    service::physical_delete(&id)
}
