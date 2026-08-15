use crate::business::metadata::entity::Metadata;
use crate::business::metadata::service;
use crate::error_code::ErrorCode;
use crate::util::preprocess_util;

/// 注册一个用户数据库。
///
/// # 参数
/// - `name`: 数据库名称。
///
/// # 返回值
/// 返回新建记录的元数据；若发生错误则返回对应的 `ErrorCode`。
#[tauri::command]
pub fn metadata_register(name: String) -> Result<Metadata, ErrorCode> {
    preprocess(name)
}

/// `metadata_register` 的 preprocess 函数：校验参数后接入 service 层的 register 函数。
pub fn preprocess(name: String) -> Result<Metadata, ErrorCode> {
    let name = preprocess_util::preprocess_user_database_name(name)?;
    service::register(name)
}
