use rusqlite::Connection;

use crate::business::user_database::entity::Node;
use crate::business::user_database::shadow::service::resolve_root;
use crate::error_code::ErrorCode;

/// 取节点用于日志载荷的展示标题：影子节点的标题落库为空串（影子自有数据仅位置与产生边引用，
/// 展示数据从根本体节点拉取），故影子节点沿产生边链解析根本体并取根本体标题；
/// 非影子节点的根本体是其自身，直接取自身标题。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `node`: 待取标题的节点（可以是影子节点）。
///
/// # 返回值
/// 返回节点的展示标题；影子链解析失败（产生边悬空、端点缺失或影子链成环）时返回对应的
/// `DataCorruption*` 错误，发生数据库错误时返回对应的 `ErrorCode`。
pub fn display_title(connection: &Connection, node: &Node) -> Result<String, ErrorCode> {
    if node.shadow_id.is_none() {
        return Ok(node.title.clone());
    }
    Ok(resolve_root(connection, node)?.title)
}
