use crate::business::user_database::edge::service;
use crate::error_code::ErrorCode;
use crate::util::preprocess_util;

/// 物理删除指定边。边的某个端点是画布节点时，被引用子画布内另一端节点的影子节点随边一并删除；
/// 若影子节点在子画布内有关联节点，未确认时返回 `ErrorCode::EdgeDeleteDisconnectsNodes`，
/// 前端向用户确认后以 `confirmed = true` 重调。
///
/// # 参数
/// - `id`: 边 id。
/// - `confirmed`: 用户已确认影子节点删除带来的连接断开影响。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_edge_delete(id: String, confirmed: bool) -> Result<(), ErrorCode> {
    preprocess(id, confirmed)
}

/// `user_database_edge_delete` 的 preprocess 函数：校验参数后接入 service 层的 delete 函数。
///
/// # 参数
/// - `id`: 边 id。
/// - `confirmed`: 用户已确认影子节点删除带来的连接断开影响。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn preprocess(id: String, confirmed: bool) -> Result<(), ErrorCode> {
    let id = preprocess_util::preprocess_edge_id(id)?;
    service::delete(&id, confirmed)
}
