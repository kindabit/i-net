use crate::business::preference::service;
use crate::error_code::ErrorCode;
use crate::util::preprocess_util;

/// 查询偏好项的值。
///
/// # 参数
/// - `name`: 偏好项名称。
///
/// # 返回值
/// 返回偏好项的值，不存在时返回 `None`；若发生错误则返回对应的 `ErrorCode`。
#[tauri::command]
pub fn preference_get(name: String) -> Result<Option<String>, ErrorCode> {
    preprocess(name)
}

/// `preference_get` 的 preprocess 函数：校验参数后接入 service 层的 get 函数。
pub fn preprocess(name: String) -> Result<Option<String>, ErrorCode> {
    let name = preprocess_util::preprocess_preference_name(name)?;
    service::get(&name)
}
