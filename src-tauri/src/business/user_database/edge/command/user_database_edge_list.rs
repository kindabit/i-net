use crate::business::user_database::edge::service;
use crate::business::user_database::entity::Edge;
use crate::error_code::ErrorCode;
use crate::util::preprocess_util;

/// 返回指定画布内的所有边。
///
/// # 参数
/// - `canvas_id`: 画布 id。
///
/// # 返回值
/// 返回该画布内的边列表；若发生错误则返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_edge_list(canvas_id: String) -> Result<Vec<Edge>, ErrorCode> {
    preprocess(canvas_id)
}

/// `user_database_edge_list` 的 preprocess 函数：校验参数后接入 service 层的 list 函数。
pub fn preprocess(canvas_id: String) -> Result<Vec<Edge>, ErrorCode> {
    let canvas_id = preprocess_util::preprocess_canvas_id(canvas_id)?;
    service::list(&canvas_id)
}
