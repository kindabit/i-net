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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::business::metadata;
    use crate::business::user_database::entity;
    use crate::business::user_database::lifecycle;
    use crate::business::user_database::log;
    use crate::business::user_database::state;
    use crate::test;

    /// 日志 command 层 preprocess 的筛选预处理：时间范围校验（InvalidLogTimeRange）与
    /// keyword / actions 的归一化（纯空白关键词视为无关键词、trim+小写化、空 actions 视为不过滤）。
    #[test]
    fn test_log_list_command_filter() {
        let _guard = test::acquire_test_lock();

        // 初始化测试数据目录、metadata 数据库并打开一个全新的用户数据库。
        let path = test::create_test_path();
        crate::state::set_path(path.clone());
        metadata::service::initialize().unwrap();
        let registered =
            metadata::service::register("log-filter-cmd-test-db".to_string()).unwrap();
        lifecycle::command::initialize::preprocess(registered.id.clone(), "password".to_string())
            .unwrap();

        // 直接向 log 表插入一条标题为 "Hello World" 的日志（time 可控，detail 与落库格式一致）。
        // command::initialize 的密钥由密码 "password" 派生，加密必须使用同一密钥。
        let key = crate::util::preprocess_util::preprocess_password("password".to_string()).unwrap();
        let value = serde_json::to_value(&entity::Action::NodeCreate {
            title: "Hello World".to_string(),
            sub_title: String::new(),
        })
        .unwrap();
        let variant = value.get("variant").and_then(|v| v.as_str()).unwrap().to_string();
        let data = value.get("data").cloned().unwrap_or(serde_json::Value::Null);
        let data = serde_json::to_string(&data).unwrap();
        let detail = crate::security::aes::encrypt(data.into_bytes(), key).unwrap();
        {
            let connection = state::lock_connection();
            log::dao::insert(
                &connection,
                &entity::Log {
                    id: "cmd-log-1".to_string(),
                    action: variant,
                    time: 100,
                    detail,
                },
            )
            .unwrap();
        }

        // preprocess 失败路径：start_time > end_time 返回 InvalidLogTimeRange。
        assert!(matches!(
            log::command::list::preprocess(0, 100, Some(200), Some(100), None, None),
            Err(ErrorCode::InvalidLogTimeRange { .. })
        ));

        // preprocess 失败路径：错误载荷携带 trim 后的 start 与 end。
        match log::command::list::preprocess(0, 100, Some(300), Some(100), None, None) {
            Err(ErrorCode::InvalidLogTimeRange { start, end }) => {
                assert_eq!(start, 300);
                assert_eq!(end, 100);
            }
            other => panic!("expected InvalidLogTimeRange, got {other:?}"),
        }

        // preprocess 成功路径：时间范围相等时不报错（闭区间，start == end 合法）。
        let page = log::command::list::preprocess(0, 100, Some(100), Some(100), None, None).unwrap();
        assert_eq!(page.total, 1);

        // preprocess 预处理：keyword 为纯空白时按无关键词处理（结果与无筛选一致）。
        let all = log::command::list::preprocess(0, 100, None, None, None, None).unwrap();
        let blank =
            log::command::list::preprocess(0, 100, None, None, None, Some("   ".to_string())).unwrap();
        assert_eq!(blank.total, all.total);
        assert_eq!(blank.items.len(), all.items.len());

        // preprocess 预处理：keyword 带首尾空白且大小写混合时仍能命中（trim 与小写化生效）。
        let hit =
            log::command::list::preprocess(0, 100, None, None, None, Some("  hello  ".to_string()))
                .unwrap();
        assert_eq!(hit.total, 1);
        assert_eq!(hit.items[0].id, "cmd-log-1");

        // preprocess 预处理：actions 空数组视为 None（不过滤）。
        let empty_actions =
            log::command::list::preprocess(0, 100, None, None, Some(vec![]), None).unwrap();
        assert_eq!(empty_actions.total, all.total);

        // preprocess 成功路径：actions 过滤生效（不存在的 variant 返回空结果）。
        let no_match = log::command::list::preprocess(
            0,
            100,
            None,
            None,
            Some(vec!["CanvasMove".to_string()]),
            None,
        )
        .unwrap();
        assert_eq!(no_match.total, 0);
        assert!(no_match.items.is_empty());

        lifecycle::command::save::preprocess().unwrap();
        lifecycle::command::close::preprocess().unwrap();
        test::cleanup(&path);
    }
}