use crate::business::user_database::edge::service;
use crate::error_code::ErrorCode;
use crate::util::preprocess_util;

/// 物理删除指定边。
///
/// # 参数
/// - `id`: 边 id。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_edge_delete(id: String) -> Result<(), ErrorCode> {
    preprocess(id)
}

/// `user_database_edge_delete` 的 preprocess 函数：校验参数后接入 service 层的 delete 函数。
pub fn preprocess(id: String) -> Result<(), ErrorCode> {
    let id = preprocess_util::preprocess_edge_id(id)?;
    service::delete(&id)
}
