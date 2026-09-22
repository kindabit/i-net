use crate::business::user_database::node_tag::service;
use crate::error_code::ErrorCode;
use crate::util::preprocess_util;

/// 查询指定节点的标签列表（按标签名称升序）。
///
/// # 参数
/// - `node_id`: 节点 id。
///
/// # 返回值
/// 返回标签名称列表；若发生错误则返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_node_tag_list_for_node(node_id: String) -> Result<Vec<String>, ErrorCode> {
    preprocess(node_id)
}

/// `user_database_node_tag_list_for_node` 的 preprocess 函数：校验 node_id 后接入 service 层的 list_for_node 函数。
pub fn preprocess(node_id: String) -> Result<Vec<String>, ErrorCode> {
    let node_id = preprocess_util::preprocess_node_id(node_id)?;
    service::list_for_node(&node_id)
}
