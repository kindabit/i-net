use crate::business::user_database::node_field::service;
use crate::business::user_database::node_field::vo::NodeFieldVO;
use crate::error_code::ErrorCode;
use crate::util::preprocess_util;

/// 获取指定节点的全部字段。
///
/// # 参数
/// - `node_id`: 节点 id。
///
/// # 返回值
/// 返回字段值对象列表；若发生错误则返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_node_field_get(node_id: String) -> Result<Vec<NodeFieldVO>, ErrorCode> {
    preprocess(node_id)
}

/// `user_database_node_field_get` 的 preprocess 函数：校验 node_id 后接入 service 层的 get 函数。
pub fn preprocess(node_id: String) -> Result<Vec<NodeFieldVO>, ErrorCode> {
    let node_id = preprocess_util::preprocess_node_id(node_id)?;
    service::get(&node_id)
}
