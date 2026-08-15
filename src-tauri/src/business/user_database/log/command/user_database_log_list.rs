use crate::business::user_database::log::response::log_page_response::LogPageResponse;
use crate::business::user_database::log::service;
use crate::error_code::ErrorCode;

/// 分页查询日志，按时间倒序排序。
///
/// # 参数
/// - `offset`: 跳过的日志条数。
/// - `limit`: 最多返回的日志条数。
///
/// # 返回值
/// 返回解密后的日志分页列表（含总数）；若发生错误则返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_log_list(offset: i64, limit: i64) -> Result<LogPageResponse, ErrorCode> {
    preprocess(offset, limit)
}

/// `user_database_log_list` 的 preprocess 函数：接入 service 层的 list 函数。
pub fn preprocess(offset: i64, limit: i64) -> Result<LogPageResponse, ErrorCode> {
    service::list(offset, limit)
}
