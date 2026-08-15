use crate::business::metadata::entity::Metadata;
use crate::business::metadata::service;
use crate::error_code::ErrorCode;

/// 按归档状态查询用户数据库列表。
///
/// # 参数
/// - `archived`: 归档状态，`false` 查询未归档的数据库，`true` 查询已归档的数据库。
///
/// # 返回值
/// 返回查询到的元数据列表；若发生错误则返回对应的 `ErrorCode`。
#[tauri::command]
pub fn metadata_list(archived: bool) -> Result<Vec<Metadata>, ErrorCode> {
    preprocess(archived)
}

/// `metadata_list` 的 preprocess 函数：接入 service 层的 list 函数。
pub fn preprocess(archived: bool) -> Result<Vec<Metadata>, ErrorCode> {
    service::list(archived)
}
