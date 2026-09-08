use crate::business::user_database::edge::service;
use crate::business::user_database::entity::Edge;
use crate::error_code::ErrorCode;
use crate::util::preprocess_util;

/// 返回指定边。
///
/// # 参数
/// - `id`: 边 id。
///
/// # 返回值
/// 返回该边；若发生错误则返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_edge_get(id: String) -> Result<Edge, ErrorCode> {
    preprocess(id)
}

/// `user_database_edge_get` 的 preprocess 函数：校验参数后接入 service 层的 get 函数。
pub fn preprocess(id: String) -> Result<Edge, ErrorCode> {
    let id = preprocess_util::preprocess_edge_id(id)?;
    service::get(&id)
}
