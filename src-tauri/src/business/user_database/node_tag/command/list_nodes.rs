use crate::business::user_database::node::response::NodeSearchResponse;
use crate::business::user_database::node_tag::service;
use crate::error_code::ErrorCode;

/// 按标签查询节点（标签精确匹配）。
///
/// # 参数
/// - `tag`: 标签名称。
///
/// # 返回值
/// 返回携带该标签的节点搜索结果列表；若发生错误则返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_node_tag_list_nodes(tag: String) -> Result<Vec<NodeSearchResponse>, ErrorCode> {
    preprocess(tag)
}

/// `user_database_node_tag_list_nodes` 的 preprocess 函数：裁剪标签首尾空白后接入 service 层的 list_nodes 函数。
pub fn preprocess(tag: String) -> Result<Vec<NodeSearchResponse>, ErrorCode> {
    service::list_nodes(tag.trim())
}
