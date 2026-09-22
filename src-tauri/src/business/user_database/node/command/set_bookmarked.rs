use crate::business::user_database::node::service;
use crate::error_code::ErrorCode;
use crate::util::preprocess_util;

/// 设置指定节点的书签状态。
///
/// # 参数
/// - `id`: 节点 id。
/// - `bookmarked`: 目标书签状态，true 表示收藏，false 表示取消收藏。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_node_set_bookmarked(id: String, bookmarked: bool) -> Result<(), ErrorCode> {
    preprocess(id, bookmarked)
}

/// `user_database_node_set_bookmarked` 的 preprocess 函数：校验 id 后接入 service 层的 set_bookmarked 函数。
pub fn preprocess(id: String, bookmarked: bool) -> Result<(), ErrorCode> {
    let id = preprocess_util::preprocess_node_id(id)?;
    service::set_bookmarked(&id, bookmarked)
}
