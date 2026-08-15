use crate::business::preference::service;
use crate::error_code::ErrorCode;
use crate::util::preprocess_util;

/// 插入或更新偏好项。
///
/// # 参数
/// - `name`: 偏好项名称。
/// - `value`: 偏好项值。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
#[tauri::command]
pub fn preference_set(name: String, value: String) -> Result<(), ErrorCode> {
    preprocess(name, value)
}

/// `preference_set` 的 preprocess 函数：校验参数后接入 service 层的 set 函数。
pub fn preprocess(name: String, value: String) -> Result<(), ErrorCode> {
    let name = preprocess_util::preprocess_preference_name(name)?;
    service::set(&name, &value)
}
