use crate::business::metadata::service;
use crate::error_code::ErrorCode;
use crate::util::preprocess_util;

/// 设置指定 id 的用户数据库的归档状态。
///
/// # 参数
/// - `id`: 数据库 id。
/// - `archived`: 归档状态，`true` 表示归档，`false` 表示解除归档。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
#[tauri::command]
pub fn metadata_archive(id: String, archived: bool) -> Result<(), ErrorCode> {
    preprocess(id, archived)
}

/// `metadata_archive` 的 preprocess 函数：校验参数后接入 service 层的 archive 函数。
pub fn preprocess(id: String, archived: bool) -> Result<(), ErrorCode> {
    let id = preprocess_util::preprocess_user_database_id(id)?;
    service::archive(&id, archived)
}
