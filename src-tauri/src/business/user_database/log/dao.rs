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

/// 分页查询日志，按时间从大到小排序，时间相同的按 id 从大到小排序。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `offset`: 跳过的日志条数。
/// - `limit`: 最多返回的日志条数。
///
/// # 返回值
/// 返回查询到的日志列表；若发生错误则返回对应的 `ErrorCode`。
pub fn select_paged(
    connection: &Connection,
    offset: i64,
    limit: i64,
) -> Result<Vec<Log>, ErrorCode> {
    let mut statement = connection
        .prepare(
            "SELECT id, object_id, action, time, detail
            FROM log
            ORDER BY time DESC, id DESC
            LIMIT :limit OFFSET :offset",
        )
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    let rows = statement
        .query_map(
            rusqlite::named_params! {
                ":limit": limit,
                ":offset": offset,
            },
            map_row,
        )
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })
}

/// 查询日志总条数。
///
/// # 参数
/// - `connection`: 数据库连接。
///
/// # 返回值
/// 返回日志总条数；若发生错误则返回对应的 `ErrorCode`。
pub fn select_count(connection: &Connection) -> Result<i64, ErrorCode> {
    let count: i64 = connection
        .query_row("SELECT COUNT(*) FROM log", [], |row| row.get(0))
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
            select_paged(&connection, 0, 100),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // select_count 失败路径：表不存在时报 DatabaseError。
        assert!(matches!(
            select_count(&connection),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // create_table 成功路径。
        create_table(&connection).unwrap();

        // select_count 成功路径：建表后、未插入任何数据前计数为 0。
        assert_eq!(select_count(&connection).unwrap(), 0);

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
            let selected = select_paged(&connection, 0, 100).unwrap();
            let selected = selected.iter().find(|log| log.id == id).unwrap();
            assert_eq!(selected.action, action);
            assert_eq!(selected.detail, format!("detail-{id}").into_bytes());
        }

        // select_count 成功路径：插入 5 条日志后计数为 5。
        assert_eq!(select_count(&connection).unwrap(), 5);

        // insert 失败路径：id 重复时报 DatabaseError（主键约束）。
        assert!(matches!(
            insert(&connection, &log("id-action-0", "CanvasCreate", 100)),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // select_paged 成功路径：按时间从大到小排序，时间相同的按 id 从大到小排序。
        insert(&connection, &log("id-early", "CanvasCreate", 50)).unwrap();
        insert(&connection, &log("id-late", "CanvasCreate", 200)).unwrap();

        // select_count 成功路径：再插入 2 条后总数为 7。
        assert_eq!(select_count(&connection).unwrap(), 7);
        let all = select_paged(&connection, 0, 100).unwrap();
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
        let page = select_paged(&connection, 1, 2).unwrap();
        assert_eq!(page.len(), 2);
        assert_eq!(page[0].id, "id-action-4");
        assert_eq!(page[1].id, "id-action-3");
        assert!(select_paged(&connection, 100, 100).unwrap().is_empty());
    }
}
