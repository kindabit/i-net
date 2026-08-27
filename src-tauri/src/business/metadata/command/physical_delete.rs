use crate::business::metadata::service;
use crate::error_code::ErrorCode;
use crate::util::preprocess_util;

/// 物理删除一个用户数据库。
///
/// # 参数
/// - `id`: 数据库 id。
/// - `password`: 该数据库的密码。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
#[tauri::command]
pub fn metadata_physical_delete(id: String, password: String) -> Result<(), ErrorCode> {
    preprocess(id, password)
}

/// `metadata_physical_delete` 的 preprocess 函数：校验参数并将密码哈希为密钥后，
/// 接入 service 层的 physical_delete 函数。
pub fn preprocess(id: String, password: String) -> Result<(), ErrorCode> {
    let id = preprocess_util::preprocess_user_database_id(id)?;
    let key = preprocess_util::preprocess_password(password)?;
    service::physical_delete(&id, key)
}
