use rusqlite::{Connection, OptionalExtension};

use crate::error_code::ErrorCode;

/// 新建 variable 表。
///
/// # 参数
/// - `connection`: 数据库连接。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn create_table(connection: &Connection) -> Result<(), ErrorCode> {
    connection
        .execute(
            "CREATE TABLE variable (
                name TEXT PRIMARY KEY,
                value TEXT NOT NULL
            ) STRICT",
            [],
        )
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    Ok(())
}

/// 判断 variable 表是否存在。
///
/// # 参数
/// - `connection`: 数据库连接。
///
/// # 返回值
/// 返回表是否存在的布尔值；若发生错误则返回对应的 `ErrorCode`。
pub fn exist_table(connection: &Connection) -> Result<bool, ErrorCode> {
    let count: i64 = connection
        .query_row(
            "SELECT COUNT(*)
            FROM sqlite_master
            WHERE type = 'table' AND name = 'variable'",
            [],
            |row| row.get(0),
        )
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    Ok(count > 0)
}

/// 向 variable 表插入或更新一条变量。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `name`: 变量名称。
/// - `value`: 变量值。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn upsert(connection: &Connection, name: &str, value: &str) -> Result<(), ErrorCode> {
    connection
        .execute(
            "INSERT INTO variable (name, value)
            VALUES (:name, :value)
            ON CONFLICT(name) DO UPDATE SET value = excluded.value",
            rusqlite::named_params! {
                ":name": name,
                ":value": value,
            },
        )
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    Ok(())
}

/// 按名称查询变量。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `name`: 变量名称。
///
/// # 返回值
/// 返回变量的值，不存在时返回 `None`；若发生错误则返回对应的 `ErrorCode`。
pub fn select_by_name(
    connection: &Connection,
    name: &str,
) -> Result<Option<String>, ErrorCode> {
    connection
        .query_row(
            "SELECT value
            FROM variable
            WHERE name = :name",
            rusqlite::named_params! {":name": name},
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 覆盖 variable dao 模块所有 dao 函数的成功与失败路径。
    #[test]
    fn test_variable_dao_all_functions() {
        let connection = Connection::open_in_memory().unwrap();

        // exist_table 成功路径：表不存在时返回 false。
        assert!(!exist_table(&connection).unwrap());

        // upsert 失败路径：表不存在时报 DatabaseError。
        assert!(matches!(
            upsert(&connection, "theme", "dark"),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // select_by_name 失败路径：表不存在时报 DatabaseError。
        assert!(matches!(
            select_by_name(&connection, "theme"),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // create_table 成功路径：建表后 exist_table 返回 true。
        create_table(&connection).unwrap();
        assert!(exist_table(&connection).unwrap());

        // create_table 失败路径：重复建表报 DatabaseError。
        assert!(matches!(
            create_table(&connection),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // select_by_name 成功路径：变量不存在时返回 None。
        assert!(select_by_name(&connection, "theme").unwrap().is_none());

        // upsert 成功路径：插入变量后能查询到相同的值。
        upsert(&connection, "theme", "dark").unwrap();
        assert_eq!(
            select_by_name(&connection, "theme").unwrap(),
            Some("dark".to_string())
        );

        // upsert 成功路径：对已存在的变量执行更新。
        upsert(&connection, "theme", "light").unwrap();
        assert_eq!(
            select_by_name(&connection, "theme").unwrap(),
            Some("light".to_string())
        );
    }
}
