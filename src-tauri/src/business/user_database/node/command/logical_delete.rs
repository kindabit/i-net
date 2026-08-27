use crate::business::user_database::entity::Node;
use crate::business::user_database::node::service;
use crate::error_code::ErrorCode;
use crate::util::preprocess_util;

/// 逻辑删除指定节点。
///
/// # 参数
/// - `id`: 节点 id。
///
/// # 返回值
/// 成功时返回逻辑删除后的节点对象；若发生错误则返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_node_logical_delete(id: String) -> Result<Node, ErrorCode> {
    preprocess(id)
}

/// `user_database_node_logical_delete` 的 preprocess 函数：校验参数后接入 service 层的 logical_delete 函数。
pub fn preprocess(id: String) -> Result<Node, ErrorCode> {
    let id = preprocess_util::preprocess_node_id(id)?;
    service::logical_delete(&id)
}
