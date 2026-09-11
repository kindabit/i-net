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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::business::metadata;
    use crate::business::user_database::entity;
    use crate::business::user_database::lifecycle;
    use crate::business::user_database::log;
    use crate::test;

    /// 日志分页查询的筛选能力：时间范围、行为类型与内容关键词（含组合过滤与关键词分页语义）。
    #[test]
    fn test_log_list_filter() {
        let _guard = test::acquire_test_lock();

        // 初始化测试数据目录、metadata 数据库并打开一个全新的用户数据库。
        let path = test::create_test_path();
        crate::state::set_path(path.clone());
        metadata::service::initialize().unwrap();
        let registered = metadata::service::register("log-filter-test-db".to_string()).unwrap();
        lifecycle::service::initialize(&registered.id, test::test_key()).unwrap();

        // 直接向 log 表插入一条日志（time 可控，detail 与 log::service::create 的落库格式一致）。
        let insert_log = |id: &str, action: &entity::Action, time: i64| {
            let value = serde_json::to_value(action).unwrap();
            let variant = value.get("variant").and_then(|v| v.as_str()).unwrap().to_string();
            let data = value.get("data").cloned().unwrap_or(serde_json::Value::Null);
            let data = serde_json::to_string(&data).unwrap();
            let detail = crate::security::aes::encrypt(data.into_bytes(), test::test_key()).unwrap();
            let connection = state::lock_connection();
            log::dao::insert(
                &connection,
                &entity::Log {
                    id: id.to_string(),
                    action: variant,
                    time,
                    detail,
                },
            )
            .unwrap();
        };
        // 构造 LogFilter 的辅助闭包。
        let filter = |start_time: Option<i64>, end_time: Option<i64>, actions: Option<Vec<String>>, keyword: Option<String>| LogFilter {
            start_time,
            end_time,
            actions,
            keyword,
        };

        // 四条基础日志：节点创建（标题含关键词）、字段修改（changes 嵌套字段值含关键词）、
        // 画布创建（名称大写混合）、画布移动（载荷不含关键词）。
        insert_log(
            "log-1",
            &entity::Action::NodeCreate {
                title: "Alpha Project".to_string(),
                sub_title: "sub".to_string(),
            },
            100,
        );
        insert_log(
            "log-2",
            &entity::Action::NodeFieldsModify {
                node_title: "node".to_string(),
                changes: vec![entity::NodeFieldChange::Added {
                    name: "密码".to_string(),
                    field_type: "string:password".to_string(),
                    value: Some("TopSecret".to_string()),
                }],
            },
            200,
        );
        insert_log(
            "log-3",
            &entity::Action::CanvasCreate {
                name: "MIXED Case Canvas".to_string(),
            },
            300,
        );
        insert_log(
            "log-4",
            &entity::Action::CanvasMove {
                name: "canvas".to_string(),
                old_x: 0.0,
                old_y: 0.0,
                new_x: 1.0,
                new_y: 1.0,
            },
            400,
        );

        // keyword 命中日志载荷中的节点标题（NodeCreate 的 title）。
        let page = log::service::list(0, 100, filter(None, None, None, Some("alpha".to_string())))
            .unwrap();
        assert_eq!(page.total, 1);
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].id, "log-1");

        // keyword 命中 NodeFieldsModify 的 changes 嵌套字段值。
        let page = log::service::list(0, 100, filter(None, None, None, Some("topsecret".to_string())))
            .unwrap();
        assert_eq!(page.total, 1);
        assert_eq!(page.items[0].id, "log-2");

        // keyword 大小写不敏感：内容为大写、关键词为小写也命中。
        let page = log::service::list(0, 100, filter(None, None, None, Some("mixed".to_string())))
            .unwrap();
        assert_eq!(page.total, 1);
        assert_eq!(page.items[0].id, "log-3");

        // keyword 不命中：items 为空且 total 为 0。
        let page =
            log::service::list(0, 100, filter(None, None, None, Some("no-such-keyword".to_string())))
                .unwrap();
        assert!(page.items.is_empty());
        assert_eq!(page.total, 0);

        // 时间范围过滤命中：闭区间 [200, 300] 恰好覆盖 log-2 与 log-3 的 time。
        let page = log::service::list(0, 100, filter(Some(200), Some(300), None, None)).unwrap();
        assert_eq!(page.total, 2);
        assert!(page.items.iter().all(|e| e.id == "log-2" || e.id == "log-3"));

        // 时间范围过滤排除：只给 start 时排除更早的记录。
        let page = log::service::list(0, 100, filter(Some(300), None, None, None)).unwrap();
        assert_eq!(page.total, 2);
        assert!(page.items.iter().all(|e| e.id == "log-3" || e.id == "log-4"));

        // 时间范围过滤排除：只给 end 时排除更晚的记录。
        let page = log::service::list(0, 100, filter(None, Some(200), None, None)).unwrap();
        assert_eq!(page.total, 2);
        assert!(page.items.iter().all(|e| e.id == "log-1" || e.id == "log-2"));

        // action 过滤：只返回指定 variant 的日志。
        let page =
            log::service::list(0, 100, filter(None, None, Some(vec!["CanvasCreate".to_string()]), None))
                .unwrap();
        assert_eq!(page.total, 1);
        assert_eq!(page.items[0].id, "log-3");

        // action 过滤：多个 variant 任一匹配。
        let page = log::service::list(
            0,
            100,
            filter(
                None,
                None,
                Some(vec!["CanvasCreate".to_string(), "CanvasMove".to_string()]),
                None,
            ),
        )
        .unwrap();
        assert_eq!(page.total, 2);
        assert!(page.items.iter().all(|e| e.id == "log-3" || e.id == "log-4"));

        // 时间 + action + keyword 组合过滤：画布创建日志中名称含 "mixed" 且 time 在 [100, 300]。
        let page = log::service::list(
            0,
            100,
            filter(
                Some(100),
                Some(300),
                Some(vec!["CanvasCreate".to_string()]),
                Some("mixed".to_string()),
            ),
        )
        .unwrap();
        assert_eq!(page.total, 1);
        assert_eq!(page.items[0].id, "log-3");

        // keyword 搜索的分页：再插入 5 条标题含 "needle" 的日志（time 依次递增），
        // 验证 offset/limit 以匹配序号为基准翻页且 total 为匹配总数。
        for index in 0..5 {
            insert_log(
                &format!("needle-{index}"),
                &entity::Action::NodeCreate {
                    title: format!("needle title {index}"),
                    sub_title: String::new(),
                },
                500 + index,
            );
        }
        let page = log::service::list(0, 100, filter(None, None, None, Some("needle".to_string())))
            .unwrap();
        assert_eq!(page.total, 5);
        assert_eq!(page.items.len(), 5);
        // 匹配序号按时间倒序：offset=1、limit=2 取第 2、3 条命中（time 为 503 与 502）。
        let page = log::service::list(1, 2, filter(None, None, None, Some("needle".to_string())))
            .unwrap();
        assert_eq!(page.total, 5);
        assert_eq!(page.items.len(), 2);
        assert_eq!(page.items[0].time, 503);
        assert_eq!(page.items[1].time, 502);

        // 保存并关闭数据库，清理测试数据目录。
        lifecycle::service::save().unwrap();
        lifecycle::service::close().unwrap();
        test::cleanup(&path);
    }
}
