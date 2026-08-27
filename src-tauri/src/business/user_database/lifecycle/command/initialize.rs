use crate::business::metadata::entity::Metadata;
use crate::business::user_database::lifecycle::service;
use crate::error_code::ErrorCode;
use crate::util::preprocess_util;

/// 初始化（打开）一个用户数据库。
///
/// # 参数
/// - `id`: 数据库 id。
/// - `password`: 数据库密码。
///
/// # 返回值
/// 返回更新过最后打开时间的元数据；若发生错误则返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_lifecycle_initialize(
    id: String,
    password: String,
) -> Result<Metadata, ErrorCode> {
    preprocess(id, password)
}

/// `user_database_lifecycle_initialize` 的 preprocess 函数：校验参数后接入 service 层的 initialize 函数。
pub fn preprocess(id: String, password: String) -> Result<Metadata, ErrorCode> {
    let id = preprocess_util::preprocess_user_database_id(id)?;
    let key = preprocess_util::preprocess_password(password)?;
    service::initialize(&id, key)
}
