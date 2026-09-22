use crate::business::user_database::node_tag::dao;
use crate::business::user_database::state;
use crate::error_code::ErrorCode;

/// 查询指定节点的全部标签，按标签名称升序。不产生日志，也不校验节点是否存在
/// （节点不存在或影子节点没有标签行时自然返回空列表）。
///
/// # 参数
/// - `node_id`: 节点 id。
///
/// # 返回值
/// 返回该节点的标签名称列表；若发生错误则返回对应的 `ErrorCode`。
pub fn list_for_node(node_id: &str) -> Result<Vec<String>, ErrorCode> {
    let connection = state::lock_connection();
    dao::select_tags_by_node_id(&connection, node_id)
}
