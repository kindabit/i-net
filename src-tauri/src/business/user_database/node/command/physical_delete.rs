use crate::business::user_database::node::service;
use crate::error_code::ErrorCode;
use crate::util::preprocess_util;

/// 物理删除指定节点和与它相连的全部边；若节点在其它画布的影子子树连有关联节点，
/// 未确认时返回 `ErrorCode::NodeDeleteDisconnectsNodes`，前端向用户确认后以
/// `confirmed = true` 重调。
///
/// # 参数
/// - `id`: 节点 id。
/// - `confirmed`: 用户已确认影子子树删除带来的跨画布连接断开影响。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_node_physical_delete(id: String, confirmed: bool) -> Result<(), ErrorCode> {
    preprocess(id, confirmed)
}

/// `user_database_node_physical_delete` 的 preprocess 函数：校验参数后接入 service 层的 physical_delete 函数。
///
/// # 参数
/// - `id`: 节点 id。
/// - `confirmed`: 用户已确认影子子树删除带来的跨画布连接断开影响。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn preprocess(id: String, confirmed: bool) -> Result<(), ErrorCode> {
    let id = preprocess_util::preprocess_node_id(id)?;
    service::physical_delete(&id, confirmed)
}
