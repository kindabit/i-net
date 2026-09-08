use crate::business::user_database::migration::service;
use crate::business::user_database::migration::vo::{ImportedEdgeVO, ImportedNodeVO};
use crate::error_code::ErrorCode;
use crate::util::preprocess_util;

/// 数据迁移聚合导入接口（KeePass 2.0）：接收前端已构造好的节点与边数据，
/// 在根画布内创建一个画布节点，其引用的新画布内批量写入全部节点、字段与父子边，
/// 所有写库操作聚合为一条日志条目。
///
/// # 参数
/// - `canvas_name`: 新画布的名称（画布节点标题与其保持一致），重名时自动追加 " 2"、" 3"…。
/// - `canvas_node_x`: 画布节点在根画布中的 x 坐标。
/// - `canvas_node_y`: 画布节点在根画布中的 y 坐标。
/// - `nodes`: 前端构造好的导入节点列表（第一个节点表示数据库本身，为树的根）。
/// - `edges`: 前端构造好的父子边列表（下标引用 `nodes`）。
///
/// # 返回值
/// 成功时返回 `Ok(新画布的 id)`（前端凭此跳转至新画布）；
/// 画布名称为空时返回 `ErrorCode::EmptyCanvasName`，
/// 发生其他错误时返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_migration_import_keepass2(
    canvas_name: String,
    canvas_node_x: f64,
    canvas_node_y: f64,
    nodes: Vec<ImportedNodeVO>,
    edges: Vec<ImportedEdgeVO>,
) -> Result<String, ErrorCode> {
    preprocess(canvas_name, canvas_node_x, canvas_node_y, nodes, edges)
}

/// `user_database_migration_import_keepass2` 的 preprocess 函数：校验画布名称非空后
/// 接入 service 层的 import_keepass2 函数。
///
/// # 参数
/// - `canvas_name`: 新画布的名称。
/// - `canvas_node_x`: 画布节点在根画布中的 x 坐标。
/// - `canvas_node_y`: 画布节点在根画布中的 y 坐标。
/// - `nodes`: 前端构造好的导入节点列表。
/// - `edges`: 前端构造好的父子边列表（下标引用 `nodes`）。
///
/// # 返回值
/// 成功时返回 `Ok(新画布的 id)`；画布名称为空时返回 `ErrorCode::EmptyCanvasName`，
/// 其他错误由 service 层返回。
pub fn preprocess(
    canvas_name: String,
    canvas_node_x: f64,
    canvas_node_y: f64,
    nodes: Vec<ImportedNodeVO>,
    edges: Vec<ImportedEdgeVO>,
) -> Result<String, ErrorCode> {
    let canvas_name = preprocess_util::preprocess_canvas_name(canvas_name)?;
    service::import_keepass2(&canvas_name, canvas_node_x, canvas_node_y, &nodes, &edges)
}
