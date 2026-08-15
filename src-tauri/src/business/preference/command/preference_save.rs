use crate::business::preference::service;
use crate::error_code::ErrorCode;

/// 保存 preference 数据库至文件。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
#[tauri::command]
pub fn preference_save() -> Result<(), ErrorCode> {
    preprocess()
}

/// `preference_save` 的 preprocess 函数：接入 service 层的 save 函数。
pub fn preprocess() -> Result<(), ErrorCode> {
    service::save()
}
