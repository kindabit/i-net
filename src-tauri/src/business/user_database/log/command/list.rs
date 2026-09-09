use crate::business::user_database::log::response::log_page_response::LogPageResponse;
use crate::business::user_database::log::service;
use crate::business::user_database::log::service::LogFilter;
use crate::error_code::ErrorCode;

/// 分页查询日志，按时间倒序排序，支持可选的时间范围、行为类型与内容关键词筛选。
///
/// # 参数
/// - `offset`: 跳过的日志条数。
/// - `limit`: 最多返回的日志条数。
/// - `start_time`: 起始时间（毫秒时间戳，闭区间），None 表示不限。
/// - `end_time`: 结束时间（毫秒时间戳，闭区间），None 表示不限。
/// - `actions`: 行为类型列表（Action 的 variant 名），None 或空列表表示不限。
/// - `keyword`: 内容搜索关键词，None 或纯空白表示不做内容搜索。
///
/// # 返回值
/// 返回解密后的日志分页列表（含总数，带筛选时为过滤后的匹配总数）；
/// start_time 晚于 end_time 时返回 `ErrorCode::InvalidLogTimeRange`，
/// 若发生其他错误则返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_log_list(
    offset: i64,
    limit: i64,
    start_time: Option<i64>,
    end_time: Option<i64>,
    actions: Option<Vec<String>>,
    keyword: Option<String>,
) -> Result<LogPageResponse, ErrorCode> {
    preprocess(offset, limit, start_time, end_time, actions, keyword)
}

/// `user_database_log_list` 的 preprocess 函数：预处理筛选条件后接入 service 层的 list 函数。
///
/// 预处理规则：
/// - start_time 与 end_time 均存在且 start_time > end_time 时返回 `ErrorCode::InvalidLogTimeRange`。
/// - keyword 去除首尾空白后为空字符串时视为 None；非空则转小写后传递。
/// - actions 为空数组时视为 None。
///
/// # 参数
/// - `offset`: 跳过的日志条数。
/// - `limit`: 最多返回的日志条数。
/// - `start_time`: 起始时间（毫秒时间戳，闭区间），None 表示不限。
/// - `end_time`: 结束时间（毫秒时间戳，闭区间），None 表示不限。
/// - `actions`: 行为类型列表（Action 的 variant 名），None 或空列表表示不限。
/// - `keyword`: 内容搜索关键词。
///
/// # 返回值
/// 返回解密后的日志分页列表；时间范围无效时返回 `ErrorCode::InvalidLogTimeRange`，
/// 若发生其他错误则返回对应的 `ErrorCode`。
pub fn preprocess(
    offset: i64,
    limit: i64,
    start_time: Option<i64>,
    end_time: Option<i64>,
    actions: Option<Vec<String>>,
    keyword: Option<String>,
) -> Result<LogPageResponse, ErrorCode> {
    // 时间范围校验：仅当两者均存在时比较，不消费原始 Option。
    match (start_time.as_ref(), end_time.as_ref()) {
        (Some(start), Some(end)) if *start > *end => {
            return Err(ErrorCode::InvalidLogTimeRange {
                start: *start,
                end: *end,
            });
        }
        _ => {}
    }
    // keyword：trim 后为空字符串视为 None（不传关键词）；非空则转小写。
    let keyword = keyword.map(|k| k.trim().to_string());
    let keyword = match keyword {
        Some(k) if k.is_empty() => None,
        Some(k) => Some(k.to_lowercase()),
        None => None,
    };
    // actions：空数组视为 None。
    let actions = match actions {
        Some(a) if a.is_empty() => None,
        other => other,
    };
    service::list(
        offset,
        limit,
        LogFilter {
            start_time,
            end_time,
            actions,
            keyword,
        },
    )
}