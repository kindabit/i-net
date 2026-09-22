use crate::business::user_database::node_tag::service;
use crate::business::user_database::node_tag::vo::NodeTagVO;
use crate::error_code::ErrorCode;

/// 列出全部标签及其关联的未删除数据节点数量。
///
/// # 返回值
/// 返回标签值对象列表；若发生错误则返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_node_tag_list() -> Result<Vec<NodeTagVO>, ErrorCode> {
    preprocess()
}

/// `user_database_node_tag_list` 的 preprocess 函数：无参，直接接入 service 层的 list 函数。
pub fn preprocess() -> Result<Vec<NodeTagVO>, ErrorCode> {
    service::list()
}
