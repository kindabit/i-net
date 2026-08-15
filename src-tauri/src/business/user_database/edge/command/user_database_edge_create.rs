use crate::business::user_database::edge::service;
use crate::business::user_database::entity::Edge;
use crate::error_code::ErrorCode;
use crate::util::preprocess_util;

/// 在指定画布内新建一条边。
///
/// # 参数
/// - `canvas_id`: 画布 id。
/// - `source_id`: 源节点 id。
/// - `source_port`: 源节点连接桩。
/// - `target_id`: 目标节点 id。
/// - `target_port`: 目标节点连接桩。
///
/// # 返回值
/// 返回新建的边；若发生错误则返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_edge_create(
    canvas_id: String,
    source_id: String,
    source_port: String,
    target_id: String,
    target_port: String,
) -> Result<Edge, ErrorCode> {
    preprocess(canvas_id, source_id, source_port, target_id, target_port)
}

/// `user_database_edge_create` 的 preprocess 函数：校验参数后接入 service 层的 create 函数。
pub fn preprocess(
    canvas_id: String,
    source_id: String,
    source_port: String,
    target_id: String,
    target_port: String,
) -> Result<Edge, ErrorCode> {
    let canvas_id = preprocess_util::preprocess_canvas_id(canvas_id)?;
    let source_id = preprocess_util::preprocess_node_id(source_id)?;
    let source_port = preprocess_util::preprocess_node_port(source_port)?;
    let target_id = preprocess_util::preprocess_node_id(target_id)?;
    let target_port = preprocess_util::preprocess_node_port(target_port)?;
    service::create(&canvas_id, &source_id, source_port, &target_id, target_port)
}
