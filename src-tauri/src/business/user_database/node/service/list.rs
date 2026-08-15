use crate::business::user_database::entity::Node;
use crate::business::user_database::node::dao;
use crate::business::user_database::state;
use crate::error_code::ErrorCode;

/// 返回指定画布内的正常节点或者已经逻辑删除的节点。不产生日志。
///
/// # 参数
/// - `canvas_id`: 画布 id。
/// - `deleted`: 逻辑删除标志，false 返回正常节点，true 返回已逻辑删除的节点。
///
/// # 返回值
/// 返回查询到的节点列表；若发生错误则返回对应的 `ErrorCode`。
pub fn list(canvas_id: &str, deleted: bool) -> Result<Vec<Node>, ErrorCode> {
    let connection = state::lock_connection();
    dao::select_by_canvas_id_and_deleted(&connection, canvas_id, deleted)
}
