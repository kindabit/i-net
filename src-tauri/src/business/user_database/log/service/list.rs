use crate::business::user_database::entity::{Action, Log};
use crate::business::user_database::log::dao;
use crate::business::user_database::log::dao::LogQueryFilter;
use crate::business::user_database::log::response::log_list_response::LogListResponse;
use crate::business::user_database::log::response::log_page_response::LogPageResponse;
use crate::business::user_database::state;
use crate::error_code::ErrorCode;
use crate::security::aes;

/// 日志查询过滤器。所有条件均缺省时表示不过滤。
#[derive(Debug, Clone, Default)]
pub struct LogFilter {
    /// 起始时间（毫秒时间戳，闭区间），None 表示不限。
    pub start_time: Option<i64>,
    /// 结束时间（毫秒时间戳，闭区间），None 表示不限。
    pub end_time: Option<i64>,
    /// 行为类型过滤（Action 的 variant 名列表），None 或空列表表示不限。
    pub actions: Option<Vec<String>>,
    /// 内容搜索关键词（调用方已 trim 并转小写），None 表示不做内容搜索。
    pub keyword: Option<String>,
}

/// 分页查询日志：按时间倒序排序（时间相同的按 id 倒序），
/// 每条日志的行为数据解密后与 action 列的 variant 名重组并反序列化为行为。不产生日志。
///
/// 无关键词时与现状一致：SQL 层按时间范围与行为类型过滤后直接取当前页；
/// 有关键词时游标式逐行流式扫描全部候选记录，对解密后的数据 JSON 做
/// 大小写不敏感的子串匹配，命中记录按全局匹配序号分页；
/// 每条日志至多解密一次，匹配与重组复用同一份解密结果。
///
/// # 参数
/// - `offset`: 跳过的日志条数。
/// - `limit`: 最多返回的日志条数。
/// - `filter`: 过滤条件。
///
/// # 返回值
/// 返回重组后的日志分页列表（含总数，带筛选时为过滤后的匹配总数）；行为数据无法解密时
/// 返回 `ErrorCode::FailToDecrypt`，反序列化失败时返回 `ErrorCode::FailToDeserializeAction`，
/// 发生其他错误时返回对应的 `ErrorCode`。
pub fn list(offset: i64, limit: i64, filter: LogFilter) -> Result<LogPageResponse, ErrorCode> {
    let connection = state::lock_connection();
    let query_filter = LogQueryFilter {
        start_time: filter.start_time,
        end_time: filter.end_time,
        actions: filter.actions,
    };
    let key = state::key();
    match filter.keyword {
        // 无关键词路径：与现状一致，SQL 层过滤后直接解密重组当前页。
        None => {
            let total = dao::select_count(&connection, &query_filter)?;
            let logs = dao::select_paged(&connection, offset, limit, &query_filter)?;
            let items: Vec<LogListResponse> = logs
                .into_iter()
                .map(|log| {
                    let data = decrypt_log(&log, key)?;
                    reassemble_log(log, data)
                })
                .collect::<Result<Vec<_>, ErrorCode>>()?;
            Ok(LogPageResponse { items, total })
        }
        // 有关键词路径：游标式逐行流式扫描全部候选记录，按全局匹配序号分页。
        // 每条日志只解密一次，匹配与重组复用同一份解密结果。
        Some(keyword) => {
            let keyword = keyword.as_str();
            let mut items: Vec<LogListResponse> = Vec::new();
            let mut matched_total: i64 = 0;
            dao::for_each(&connection, &query_filter, |log| {
                let data = decrypt_log(&log, key)?;
                if value_contains_keyword(&data, keyword) {
                    if matched_total >= offset && matched_total < offset + limit {
                        items.push(reassemble_log(log, data)?);
                    }
                    matched_total += 1;
                }
                Ok(())
            })?;
            Ok(LogPageResponse { items, total: matched_total })
        }
    }
}

/// 解密日志的行为数据并解析为数据 JSON。
///
/// 解密失败或明文不是合法 UTF-8 时返回 `ErrorCode::FailToDecrypt`，
/// 数据 JSON 解析失败时返回 `ErrorCode::FailToDeserializeAction`。
///
/// # 参数
/// - `log`: 待解密的日志。
/// - `key`: 用户数据库密钥。
///
/// # 返回值
/// 返回解析出的数据 JSON；失败时返回对应的 `ErrorCode`。
fn decrypt_log(log: &Log, key: [u8; 32]) -> Result<serde_json::Value, ErrorCode> {
    let data = aes::decrypt(log.detail.clone(), key)?;
    let data = String::from_utf8(data).map_err(|e| ErrorCode::FailToDecrypt {
        detail: e.to_string(),
    })?;
    serde_json::from_str::<serde_json::Value>(&data)
        .map_err(|_| ErrorCode::FailToDeserializeAction)
}

/// 将日志与解密后的数据 JSON 重组为日志响应。
///
/// 数据为 Null 时按无载荷的单元变体重组（防御，目前所有变体都有载荷）；
/// Action 反序列化失败时返回 `ErrorCode::FailToDeserializeAction`。
///
/// # 参数
/// - `log`: 日志。
/// - `data`: 该日志解密后的数据 JSON。
///
/// # 返回值
/// 返回重组后的日志响应；失败时返回 `ErrorCode::FailToDeserializeAction`。
fn reassemble_log(log: Log, data: serde_json::Value) -> Result<LogListResponse, ErrorCode> {
    let Log { id, action, time, .. } = log;
    let value = if data.is_null() {
        serde_json::json!({ "variant": action })
    } else {
        serde_json::json!({ "variant": action, "data": data })
    };
    let action = serde_json::from_value::<Action>(value)
        .map_err(|_| ErrorCode::FailToDeserializeAction)?;
    Ok(LogListResponse { id, action, time })
}

/// 判断 JSON 值的任意字符串值是否包含关键词（关键词已转为小写，大小写不敏感匹配）。
/// 仅 String 类型的值参与匹配；Array/Object 递归；数字、布尔、null 不参与。
fn value_contains_keyword(value: &serde_json::Value, keyword: &str) -> bool {
    match value {
        serde_json::Value::String(s) => s.to_lowercase().contains(keyword),
        serde_json::Value::Array(arr) => arr.iter().any(|v| value_contains_keyword(v, keyword)),
        serde_json::Value::Object(map) => map.values().any(|v| value_contains_keyword(v, keyword)),
        _ => false,
    }
}