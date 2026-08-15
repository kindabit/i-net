use crate::business::user_database::entity::Action;
use crate::business::user_database::log::dao;
use crate::business::user_database::log::response::log_list_response::LogListResponse;
use crate::business::user_database::log::response::log_page_response::LogPageResponse;
use crate::business::user_database::state;
use crate::error_code::ErrorCode;
use crate::security::aes;

/// 分页查询日志：按时间倒序排序（时间相同的按 id 倒序），
/// 每条日志的行为数据解密后与 action 列的 variant 名重组并反序列化为行为。不产生日志。
///
/// # 参数
/// - `offset`: 跳过的日志条数。
/// - `limit`: 最多返回的日志条数。
///
/// # 返回值
/// 返回重组后的日志分页列表（含总数）；行为数据无法解密时返回 `ErrorCode::FailToDecrypt`，
/// 反序列化失败时返回 `ErrorCode::FailToDeserializeAction`，
/// 发生其他错误时返回对应的 `ErrorCode`。
pub fn list(offset: i64, limit: i64) -> Result<LogPageResponse, ErrorCode> {
    let connection = state::lock_connection();
    let total = dao::select_count(&connection)?;
    let logs = dao::select_paged(&connection, offset, limit)?;
    let key = state::key();
    let items: Vec<LogListResponse> = logs
        .into_iter()
        .map(|log| {
            let data = aes::decrypt(log.detail, key)?;
            let data = String::from_utf8(data).map_err(|e| ErrorCode::FailToDecrypt {
                detail: e.to_string(),
            })?;
            let data = serde_json::from_str::<serde_json::Value>(&data)
                .map_err(|_| ErrorCode::FailToDeserializeAction)?;
            // 数据为 Null 时按无载荷的单元变体重组（防御，目前所有变体都有载荷）。
            let value = if data.is_null() {
                serde_json::json!({ "variant": log.action })
            } else {
                serde_json::json!({ "variant": log.action, "data": data })
            };
            let action = serde_json::from_value::<Action>(value)
                .map_err(|_| ErrorCode::FailToDeserializeAction)?;
            Ok(LogListResponse {
                id: log.id,
                object_id: log.object_id,
                action,
                time: log.time,
            })
        })
        .collect::<Result<Vec<_>, ErrorCode>>()?;
    Ok(LogPageResponse { items, total })
}
