use crate::business::user_database::node_tag::service;
use crate::error_code::ErrorCode;
use crate::util::preprocess_util;

/// 全量设置指定节点的标签集合。
///
/// # 参数
/// - `node_id`: 节点 id。
/// - `tags`: 新的标签名称列表，允许包含空白与重复项（由 service 层规整）。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_node_tag_set_for_node(node_id: String, tags: Vec<String>) -> Result<(), ErrorCode> {
    preprocess(node_id, tags)
}

/// `user_database_node_tag_set_for_node` 的 preprocess 函数：校验 node_id 后接入 service 层的 set_for_node 函数。
pub fn preprocess(node_id: String, tags: Vec<String>) -> Result<(), ErrorCode> {
    let node_id = preprocess_util::preprocess_node_id(node_id)?;
    service::set_for_node(&node_id, tags)
}
