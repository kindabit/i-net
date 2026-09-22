use crate::business::user_database::entity::Node;
use crate::business::user_database::node::service;
use crate::error_code::ErrorCode;
use crate::util::preprocess_util;

/// 在指定画布内的指定位置创建指定节点的副本。
///
/// 副本继承源节点的标题、副标题、颜色和字段结构（不含字段值），不复制附件和边。
/// 影子节点与画布数据节点不允许复制。
///
/// # 参数
/// - `id`: 被复制的节点 id。
/// - `canvas_id`: 副本归属的目标画布 id。
/// - `x`: 副本节点在画布中的 x 坐标。
/// - `y`: 副本节点在画布中的 y 坐标。
///
/// # 返回值
/// 返回新建的副本节点；若发生错误则返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_node_copy(
    id: String,
    canvas_id: String,
    x: f64,
    y: f64,
) -> Result<Node, ErrorCode> {
    preprocess(id, canvas_id, x, y)
}

/// `user_database_node_copy` 的 preprocess 函数：校验参数后接入 service 层的 copy 函数。
///
/// id 经 `preprocess_node_id` 校验（trim + 标准小写连字符 uuid 格式），
/// canvas_id 经 `preprocess_canvas_id` 校验（trim + 标准小写连字符 uuid 格式）。
pub fn preprocess(id: String, canvas_id: String, x: f64, y: f64) -> Result<Node, ErrorCode> {
    let id = preprocess_util::preprocess_node_id(id)?;
    let canvas_id = preprocess_util::preprocess_canvas_id(canvas_id)?;
    service::copy(&id, &canvas_id, x, y)
}
