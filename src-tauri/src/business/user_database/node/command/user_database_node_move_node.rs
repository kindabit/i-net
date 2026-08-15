use crate::business::user_database::node::service;
use crate::error_code::ErrorCode;
use crate::util::preprocess_util;

/// 修改指定节点的坐标。
///
/// # 参数
/// - `id`: 节点 id。
/// - `x`: 新 x 坐标。
/// - `y`: 新 y 坐标。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_node_move_node(id: String, x: f64, y: f64) -> Result<(), ErrorCode> {
    preprocess(id, x, y)
}

/// `user_database_node_move_node` 的 preprocess 函数：校验参数后接入 service 层的 move_node 函数。
pub fn preprocess(id: String, x: f64, y: f64) -> Result<(), ErrorCode> {
    let id = preprocess_util::preprocess_node_id(id)?;
    service::move_node(&id, x, y)
}
