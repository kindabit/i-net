use crate::business::user_database::node::response::NodeSearchResponse;
use crate::business::user_database::node::service;
use crate::error_code::ErrorCode;

/// 列出所有被收藏的节点（书签列表）。
///
/// # 返回值
/// 返回被收藏节点的搜索结果列表；若发生错误则返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_node_list_bookmarked() -> Result<Vec<NodeSearchResponse>, ErrorCode> {
    preprocess()
}

/// `user_database_node_list_bookmarked` 的 preprocess 函数：无参，直接接入 service 层的 list_bookmarked 函数。
pub fn preprocess() -> Result<Vec<NodeSearchResponse>, ErrorCode> {
    service::list_bookmarked()
}
