use crate::business::user_database::node::dao;
use crate::business::user_database::node::response::NodeSearchResponse;
use crate::business::user_database::state;
use crate::error_code::ErrorCode;

/// 列出所有被收藏的节点（书签列表），过滤逻辑删除的节点、逻辑删除画布内的节点与影子节点。
/// 结果按画布名称、节点标题排序。不产生日志。
///
/// # 返回值
/// 返回被收藏节点的搜索结果列表；若发生错误则返回对应的 `ErrorCode`。
pub fn list_bookmarked() -> Result<Vec<NodeSearchResponse>, ErrorCode> {
    let connection = state::lock_connection();
    dao::select_bookmarked(&connection)
}
