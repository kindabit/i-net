use serde::Serialize;

use crate::business::user_database::log::response::log_list_response::LogListResponse;

/// 日志分页列表的响应结构：包含当前页的日志列表和日志总条数。
#[derive(Debug, Clone, Serialize)]
pub struct LogPageResponse {
    /// 当前页的日志列表，按时间倒序排序（时间相同的按 id 倒序）。
    pub items: Vec<LogListResponse>,
    /// 日志总条数。
    pub total: i64,
}
