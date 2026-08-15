use crate::business::user_database::registry::service;
use crate::error_code::ErrorCode;
use crate::util::preprocess_util;

/// 按名称查询 registry 变量的值。
///
/// # 参数
/// - `name`: 变量名称。
///
/// # 返回值
/// 返回变量的值，不存在时返回 `None`；若发生错误则返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_registry_get(name: String) -> Result<Option<String>, ErrorCode> {
    preprocess(name)
}

/// `user_database_registry_get` 的 preprocess 函数：校验参数后接入 service 层的 get 函数。
pub fn preprocess(name: String) -> Result<Option<String>, ErrorCode> {
    let name = preprocess_util::preprocess_registry_name(name)?;
    service::get(&name)
}
