use crate::business::user_database::node::service;
use crate::business::user_database::node::vo::NodeVO;
use crate::error_code::ErrorCode;
use crate::util::preprocess_util;

/// 返回指定画布内的正常节点或者已经逻辑删除的节点，影子节点的展示数据合并自原始节点。
///
/// # 参数
/// - `canvas_id`: 画布 id。
/// - `deleted`: 逻辑删除标志，false 返回正常节点，true 返回已逻辑删除的节点。
///
/// # 返回值
/// 返回节点值对象列表；若发生错误则返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_node_list(canvas_id: String, deleted: bool) -> Result<Vec<NodeVO>, ErrorCode> {
    preprocess(canvas_id, deleted)
}

/// `user_database_node_list` 的 preprocess 函数：校验参数后接入 service 层的 list 函数。
///
/// # 参数
/// - `canvas_id`: 画布 id。
/// - `deleted`: 逻辑删除标志。
///
/// # 返回值
/// 返回节点值对象列表；若发生错误则返回对应的 `ErrorCode`。
pub fn preprocess(canvas_id: String, deleted: bool) -> Result<Vec<NodeVO>, ErrorCode> {
    let canvas_id = preprocess_util::preprocess_canvas_id(canvas_id)?;
    service::list(&canvas_id, deleted)
}
