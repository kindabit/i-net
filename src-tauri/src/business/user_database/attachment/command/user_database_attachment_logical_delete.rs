use crate::business::user_database::attachment::service;
use crate::error_code::ErrorCode;
use crate::util::preprocess_util;

/// 逻辑删除附件：将附件标记为已删除（进入回收站），附件文件保留不动。
///
/// # 参数
/// - `id`: 附件 id。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_attachment_logical_delete(id: String) -> Result<(), ErrorCode> {
    preprocess(id)
}

/// `user_database_attachment_logical_delete` 的 preprocess 函数：校验 id 后接入 service 层的 logical_delete 函数。
pub fn preprocess(id: String) -> Result<(), ErrorCode> {
    let id = preprocess_util::preprocess_attachment_id(id)?;
    service::logical_delete(&id)
}
