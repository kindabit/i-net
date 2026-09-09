use rusqlite::{Connection, Row};

use crate::business::user_database::entity::Log;
use crate::error_code::ErrorCode;

/// 从查询结果行构造 Log。
fn map_row(row: &Row) -> rusqlite::Result<Log> {
    Ok(Log {
        id: row.get(0)?,
        object_id: row.get(1)?,
        action: row.get(2)?,
        time: row.get(3)?,
        detail: row.get(4)?,
    })
}

/// 新建 log 表。
///
/// # 参数
/// - `connection`: 数据库连接。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn create_table(connection: &Connection) -> Result<(), ErrorCode> {
    connection
        .execute(
            "CREATE TABLE log (
                id TEXT PRIMARY KEY,
                object_id TEXT NOT NULL,
                action TEXT NOT NULL,
                time INTEGER NOT NULL,
                detail BLOB NOT NULL
            ) STRICT",
            [],
        )
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    Ok(())
}

/// 向 log 表插入一条日志。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `log`: 要插入的日志。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn insert(connection: &Connection, log: &Log) -> Result<(), ErrorCode> {
    connection
        .execute(
            "INSERT INTO log (id, object_id, action, time, detail)
            VALUES (:id, :object_id, :action, :time, :detail)",
            rusqlite::named_params! {
                ":id": log.id,
                ":object_id": log.object_id,
                ":action": &log.action,
                ":time": log.time,
                ":detail": log.detail,
            },
        )
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    Ok(())
}

/// 日志查询的 SQL 层过滤条件。所有条件均缺省时表示不过滤。
#[derive(Debug, Clone, Default)]
pub struct LogQueryFilter {
    /// 起始时间（毫秒时间戳，闭区间），None 表示不限。
    pub start_time: Option<i64>,
    /// 结束时间（毫秒时间戳，闭区间），None 表示不限。
    pub end_time: Option<i64>,
    /// 行为类型过滤（Action 的 variant 名列表），None 或空列表表示不限。
    pub actions: Option<Vec<String>>,
}

/// 构造过滤条件的 WHERE 子句与行为过滤的动态参数名列表。
///
/// # 参数
/// - `filter`: 过滤条件。
///
/// # 返回值
/// 返回 WHERE 子句（含开头的 WHERE 关键字）与 `:action{i}` 参数名列表；
/// 行为过滤缺省时参数名列表为空，否则与 WHERE 子句中的 IN 占位符一一对应。
fn build_where_clause(filter: &LogQueryFilter) -> (String, Vec<String>) {
    let mut sql = String::from(
        "WHERE (:start IS NULL OR time >= :start)
        AND (:end IS NULL OR time <= :end)",
    );
    let actions: &[String] = filter.actions.as_deref().unwrap_or(&[]);
    let keys: Vec<String> = (0..actions.len()).map(|i| format!(":action{i}")).collect();
    if !keys.is_empty() {
        sql.push_str(&format!(" AND action IN ({})", keys.join(", ")));
    }
    (sql, keys)
}

/// 组装过滤条件的查询参数（:start/:end 加上各行为参数值）。
///
/// # 参数
/// - `filter`: 过滤条件。
/// - `keys`: [`build_where_clause`] 返回的行为参数名列表（借此保证参数字符串存活足够久）。
///
/// # 返回值
/// 返回可直接用于查询的命名参数数组。
fn bind_filter_params<'a>(
    filter: &'a LogQueryFilter,
    keys: &'a [String],
) -> Vec<(&'a str, &'a dyn rusqlite::ToSql)> {
    let actions: &[String] = filter.actions.as_deref().unwrap_or(&[]);
    let mut params: Vec<(&str, &dyn rusqlite::ToSql)> = Vec::new();
    params.push((":start", &filter.start_time));
    params.push((":end", &filter.end_time));
    for (key, action) in keys.iter().zip(actions.iter()) {
        params.push((key.as_str(), action));
    }
    params
}

/// 分页查询日志，按时间从大到小排序，时间相同的按 id 从大到小排序。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `offset`: 跳过的日志条数。
/// - `limit`: 最多返回的日志条数。
/// - `filter`: 过滤条件。
///
/// # 返回值
/// 返回查询到的日志列表；若发生错误则返回对应的 `ErrorCode`。
pub fn select_paged(
    connection: &Connection,
    offset: i64,
    limit: i64,
    filter: &LogQueryFilter,
) -> Result<Vec<Log>, ErrorCode> {
    let (where_clause, keys) = build_where_clause(filter);
    let sql = format!(
        "SELECT id, object_id, action, time, detail
        FROM log
        {where_clause}
        ORDER BY time DESC, id DESC
        LIMIT :limit OFFSET :offset"
    );
    let mut statement = connection
        .prepare(&sql)
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    let mut params = bind_filter_params(filter, &keys);
    params.push((":limit", &limit));
    params.push((":offset", &offset));
    let rows = statement
        .query_map(&params[..], map_row)
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })
}

/// 流式遍历满足过滤条件的日志（按时间倒序，时间相同的按 id 倒序），逐行调用回调处理。
///
/// 行在迭代期间从数据库逐条读取，不会一次性物化到内存；
/// 回调返回错误时立即中断遍历并原样传播该错误。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `filter`: 过滤条件。
/// - `f`: 逐行回调。
///
/// # 返回值
/// 遍历完成时返回 `Ok(())`；查询失败或回调返回错误时返回对应的 `ErrorCode`。
pub fn for_each(
    connection: &Connection,
    filter: &LogQueryFilter,
    mut f: impl FnMut(Log) -> Result<(), ErrorCode>,
) -> Result<(), ErrorCode> {
    let (where_clause, keys) = build_where_clause(filter);
    let sql = format!(
        "SELECT id, object_id, action, time, detail
        FROM log
        {where_clause}
        ORDER BY time DESC, id DESC"
    );
    let mut statement = connection
        .prepare(&sql)
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    let params = bind_filter_params(filter, &keys);
    let mut rows = statement
        .query(&params[..])
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    while let Some(row) = rows.next().map_err(|e| ErrorCode::DatabaseError {
        detail: e.to_string(),
    })? {
        f(map_row(row).map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?)?;
    }
    Ok(())
}

/// 查询日志总条数，可带过滤条件。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `filter`: 过滤条件。
///
/// # 返回值
/// 返回日志总条数；若发生错误则返回对应的 `ErrorCode`。
pub fn select_count(connection: &Connection, filter: &LogQueryFilter) -> Result<i64, ErrorCode> {
    let (where_clause, keys) = build_where_clause(filter);
    let sql = format!("SELECT COUNT(*) FROM log\n{where_clause}");
    let params = bind_filter_params(filter, &keys);
    let count: i64 = connection
        .query_row(&sql, &params[..], |row| row.get(0))
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造测试用 Log，各字段可由调用方再修改。
    fn log(id: &str, action: &str, time: i64) -> Log {
        Log {
            id: id.to_string(),
            object_id: format!("object-{id}"),
            action: action.to_string(),
            time,
            detail: format!("detail-{id}").into_bytes(),
        }
    }

    /// 构造过滤条件辅助函数，便于按需指定 start/end/actions。
    fn filter(
        start_time: Option<i64>,
        end_time: Option<i64>,
        actions: Option<Vec<String>>,
    ) -> LogQueryFilter {
        LogQueryFilter {
            start_time,
            end_time,
            actions,
        }
    }

    /// 覆盖 log dao 模块所有 dao 函数的成功与失败路径。
    #[test]
    fn test_log_dao_all_functions() {
        let connection = Connection::open_in_memory().unwrap();

        // insert 失败路径：表不存在时报 DatabaseError。
        assert!(matches!(
            insert(&connection, &log("id-1", "CanvasCreate", 100)),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // select_paged 失败路径：表不存在时报 DatabaseError。
        assert!(matches!(
            select_paged(&connection, 0, 100, &LogQueryFilter::default()),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // select_count 失败路径：表不存在时报 DatabaseError。
        assert!(matches!(
            select_count(&connection, &LogQueryFilter::default()),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // for_each 失败路径：表不存在时报 DatabaseError。
        assert!(matches!(
            for_each(&connection, &LogQueryFilter::default(), |_| Ok(())),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // create_table 成功路径。
        create_table(&connection).unwrap();

        // select_count 成功路径：建表后、未插入任何数据前计数为 0。
        assert_eq!(
            select_count(&connection, &LogQueryFilter::default()).unwrap(),
            0
        );

        // create_table 失败路径：重复建表报 DatabaseError。
        assert!(matches!(
            create_table(&connection),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // insert 成功路径：action 为 Action 的 variant 名字符串，按原样存取。
        for (index, action) in [
            "CanvasCreate",
            "CanvasMove",
            "CanvasLogicalDelete",
            "CanvasRestore",
            "CanvasPhysicalDelete",
        ]
        .into_iter()
        .enumerate()
        {
            let id = format!("id-action-{index}");
            insert(&connection, &log(&id, action, 100)).unwrap();
            let selected = select_paged(&connection, 0, 100, &LogQueryFilter::default()).unwrap();
            let selected = selected.iter().find(|log| log.id == id).unwrap();
            assert_eq!(selected.action, action);
            assert_eq!(selected.detail, format!("detail-{id}").into_bytes());
        }

        // select_count 成功路径：插入 5 条日志后计数为 5。
        assert_eq!(
            select_count(&connection, &LogQueryFilter::default()).unwrap(),
            5
        );

        // insert 失败路径：id 重复时报 DatabaseError（主键约束）。
        assert!(matches!(
            insert(&connection, &log("id-action-0", "CanvasCreate", 100)),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // select_paged 成功路径：按时间从大到小排序，时间相同的按 id 从大到小排序。
        insert(&connection, &log("id-early", "CanvasCreate", 50)).unwrap();
        insert(&connection, &log("id-late", "CanvasCreate", 200)).unwrap();

        // select_count 成功路径：再插入 2 条后总数为 7。
        assert_eq!(
            select_count(&connection, &LogQueryFilter::default()).unwrap(),
            7
        );
        let all = select_paged(&connection, 0, 100, &LogQueryFilter::default()).unwrap();
        assert_eq!(all[0].id, "id-late");
        // 时间相同的五条日志按 id 从大到小排列在 id-early 之前。
        let tail: Vec<&str> = all[1..].iter().map(|log| log.id.as_str()).collect();
        assert_eq!(
            tail,
            vec![
                "id-action-4",
                "id-action-3",
                "id-action-2",
                "id-action-1",
                "id-action-0",
                "id-early"
            ]
        );

        // select_paged 成功路径：offset 和 limit 正确分页。
        let page = select_paged(&connection, 1, 2, &LogQueryFilter::default()).unwrap();
        assert_eq!(page.len(), 2);
        assert_eq!(page[0].id, "id-action-4");
        assert_eq!(page[1].id, "id-action-3");
        assert!(select_paged(&connection, 100, 100, &LogQueryFilter::default())
            .unwrap()
            .is_empty());

        // 以下为过滤场景。当前表内日志：5 条 time=100（action 依次为 CanvasCreate /
        // CanvasMove / CanvasLogicalDelete / CanvasRestore / CanvasPhysicalDelete，id 为 id-action-0..4）、
        // 1 条 time=50（id-early, CanvasCreate）、1 条 time=200（id-late, CanvasCreate）。

        // 时间范围闭区间：start 与 end 恰好等于某条记录的 time 时命中。
        let f = filter(Some(100), Some(100), None);
        let rows = select_paged(&connection, 0, 100, &f).unwrap();
        assert_eq!(rows.len(), 5);
        assert!(rows.iter().all(|log| log.time == 100));

        // 时间范围：只给 start 时命中 time >= start 的记录。
        let f = filter(Some(100), None, None);
        let rows = select_paged(&connection, 0, 100, &f).unwrap();
        assert_eq!(rows.len(), 6);
        assert!(rows.iter().all(|log| log.time >= 100));

        // 时间范围：只给 end 时命中 time <= end 的记录。
        let f = filter(None, Some(100), None);
        let rows = select_paged(&connection, 0, 100, &f).unwrap();
        assert_eq!(rows.len(), 6);
        assert!(rows.iter().all(|log| log.time <= 100));

        // 时间范围：范围外的记录被排除（start > 所有 time 时为空）。
        let f = filter(Some(300), Some(400), None);
        assert!(select_paged(&connection, 0, 100, &f).unwrap().is_empty());

        // action 过滤：单个 variant 只返回匹配的记录。
        let f = filter(None, None, Some(vec!["CanvasCreate".to_string()]));
        let rows = select_paged(&connection, 0, 100, &f).unwrap();
        assert_eq!(rows.len(), 3);
        assert!(rows.iter().all(|log| log.action == "CanvasCreate"));

        // action 过滤：多个 variant 返回任一匹配的记录。
        let f = filter(
            None,
            None,
            Some(vec!["CanvasCreate".to_string(), "CanvasMove".to_string()]),
        );
        let rows = select_paged(&connection, 0, 100, &f).unwrap();
        assert_eq!(rows.len(), 4);
        assert!(rows.iter().all(|log| {
            log.action == "CanvasCreate" || log.action == "CanvasMove"
        }));

        // action 过滤：None 表示不过滤，返回全部记录。
        let f = filter(None, None, None);
        assert_eq!(select_paged(&connection, 0, 100, &f).unwrap().len(), 7);

        // action 过滤：Some(空列表) 也表示不过滤，返回全部记录。
        let f = filter(None, None, Some(Vec::new()));
        assert_eq!(select_paged(&connection, 0, 100, &f).unwrap().len(), 7);

        // 时间范围与 action 组合过滤：time 在 [100, 200] 且 action 为 CanvasCreate。
        let f = filter(Some(100), Some(200), Some(vec!["CanvasCreate".to_string()]));
        let rows = select_paged(&connection, 0, 100, &f).unwrap();
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().all(|log| {
            log.action == "CanvasCreate" && (100..=200).contains(&log.time)
        }));

        // select_count 与 select_paged 在相同过滤条件下结果一致（取几个典型过滤条件）。
        for f in [
            filter(Some(100), Some(100), None),
            filter(None, Some(100), None),
            filter(Some(100), Some(200), Some(vec!["CanvasCreate".to_string()])),
        ] {
            let count = select_count(&connection, &f).unwrap();
            let rows = select_paged(&connection, 0, 100, &f).unwrap();
            assert_eq!(count, rows.len() as i64);
        }

        // for_each 成功路径：无过滤时遍历全部记录，且遍历顺序与 select_paged 一致。
        let mut visited: Vec<String> = Vec::new();
        for_each(&connection, &LogQueryFilter::default(), |log| {
            visited.push(log.id);
            Ok(())
        })
        .unwrap();
        let expected: Vec<String> = select_paged(&connection, 0, 100, &LogQueryFilter::default())
            .unwrap()
            .into_iter()
            .map(|log| log.id)
            .collect();
        assert_eq!(visited, expected);
        assert_eq!(visited.len(), 7);

        // for_each 成功路径：过滤条件生效，只遍历命中的记录。
        let mut visited: Vec<String> = Vec::new();
        for_each(
            &connection,
            &filter(Some(100), Some(200), Some(vec!["CanvasCreate".to_string()])),
            |log| {
                visited.push(log.id);
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(visited, vec!["id-late", "id-action-0"]);

        // for_each 失败路径：回调返回错误时立即中断遍历并原样传播该错误。
        let mut visited_count = 0;
        let result = for_each(&connection, &LogQueryFilter::default(), |_| {
            visited_count += 1;
            if visited_count == 2 {
                return Err(ErrorCode::InvalidCiphertext);
            }
            Ok(())
        });
        assert!(matches!(result, Err(ErrorCode::InvalidCiphertext)));
        assert_eq!(visited_count, 2);
    }
}
