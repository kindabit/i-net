use crate::business::user_database::lifecycle::service;
use crate::error_code::ErrorCode;

/// 保存当前打开的用户数据库。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_lifecycle_save() -> Result<(), ErrorCode> {
    preprocess()
}

/// `user_database_lifecycle_save` 的 preprocess 函数：接入 service 层的 save 函数。
pub fn preprocess() -> Result<(), ErrorCode> {
    service::save()
}
