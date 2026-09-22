use crate::business::user_database::node::dao as node_dao;
use crate::business::user_database::node::response::NodeSearchResponse;
use crate::business::user_database::state;
use crate::error_code::ErrorCode;

/// 按标签查询节点（标签精确匹配），过滤逻辑删除的节点、逻辑删除画布内的节点与影子节点。
/// 结果按画布名称、节点标题排序。不产生日志。
///
/// # 参数
/// - `tag`: 标签名称。
///
/// # 返回值
/// 返回携带该标签的节点搜索结果列表；若发生错误则返回对应的 `ErrorCode`。
pub fn list_nodes(tag: &str) -> Result<Vec<NodeSearchResponse>, ErrorCode> {
    let connection = state::lock_connection();
    node_dao::select_by_tag(&connection, tag)
}
