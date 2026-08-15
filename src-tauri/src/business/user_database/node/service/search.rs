use crate::business::user_database::node::dao;
use crate::business::user_database::node::response::NodeSearchResponse;
use crate::business::user_database::state;
use crate::error_code::ErrorCode;

/// 在所有画布中按关键词搜索节点。不产生日志。
///
/// # 参数
/// - `keywords`: 预处理后的关键词列表，调用方保证非空且每个关键词非空。
///
/// # 返回值
/// 返回搜索结果列表；若发生错误则返回对应的 `ErrorCode`。
pub fn search(keywords: &[String]) -> Result<Vec<NodeSearchResponse>, ErrorCode> {
    let connection = state::lock_connection();
    dao::search_by_keywords(&connection, keywords)
}
