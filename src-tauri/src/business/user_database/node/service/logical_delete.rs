use crate::business::user_database::entity::Node;
use crate::business::user_database::entity::Action;
use crate::business::user_database::node::dao;
use crate::business::user_database::{canvas, log, state};
use crate::error_code::ErrorCode;

/// 逻辑删除指定节点：置上逻辑删除标志。如果是画布节点，同时逻辑删除其引用的子画布。
///
/// 产生 NodeLogicalDelete 日志，载荷为节点的标题。
///
/// # 参数
/// - `id`: 节点 id。
///
/// # 返回值
/// 成功时返回逻辑删除后的节点对象；节点不存在时返回 `ErrorCode::NoNodeWithSuchId`，
/// 发生其他错误时返回对应的 `ErrorCode`。
pub fn logical_delete(id: &str) -> Result<Node, ErrorCode> {
    let connection = state::lock_connection();
    let mut node = dao::select_by_id(&connection, id)?
        .ok_or_else(|| ErrorCode::NoNodeWithSuchId { id: id.to_string() })?;
    node.deleted = true;
    dao::update(&connection, &node)?;
    let canvas_ref_id = node.canvas_ref_id.clone();
    log::service::create(id, Action::NodeLogicalDelete { title: node.title.clone() })?;
    // 级联逻辑删除引用的子画布
    if let Some(ref_id) = canvas_ref_id {
        canvas::service::logical_delete(&ref_id)?;
    }
    Ok(node)
}
