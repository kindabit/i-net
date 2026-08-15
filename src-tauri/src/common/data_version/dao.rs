use rusqlite::Connection;

use super::entity::DataVersion;
use crate::error_code::ErrorCode;

/// 新建 data_version 表。
///
/// # 参数
/// - `connection`: 数据库连接。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn create_table(connection: &Connection) -> Result<(), ErrorCode> {
    connection
        .execute(
            "CREATE TABLE data_version (
                major INTEGER NOT NULL,
                minor INTEGER NOT NULL,
                patch INTEGER NOT NULL
            ) STRICT",
            [],
        )
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    Ok(())
}

/// 判断 data_version 表是否存在。
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
            WHERE type = 'table' AND name = 'data_version'",
            [],
            |row| row.get(0),
        )
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    Ok(count > 0)
}

/// 向 data_version 表插入一行数据版本。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `data_version`: 要插入的数据版本。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn insert(connection: &Connection, data_version: &DataVersion) -> Result<(), ErrorCode> {
    connection
        .execute(
            "INSERT INTO data_version (major, minor, patch)
            VALUES (:major, :minor, :patch)",
            rusqlite::named_params! {
                ":major": data_version.major,
                ":minor": data_version.minor,
                ":patch": data_version.patch,
            },
        )
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    Ok(())
}

/// 查询 data_version 表中的全部数据版本。
///
/// # 参数
/// - `connection`: 数据库连接。
///
/// # 返回值
/// 返回表中全部数据版本；若发生错误则返回对应的 `ErrorCode`。
pub fn select(connection: &Connection) -> Result<Vec<DataVersion>, ErrorCode> {
    let mut statement = connection
        .prepare(
            "SELECT major, minor, patch
            FROM data_version",
        )
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    let rows = statement
        .query_map([], |row| {
            Ok(DataVersion {
                major: row.get(0)?,
                minor: row.get(1)?,
                patch: row.get(2)?,
            })
        })
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 覆盖 data_version dao 模块所有 dao 函数的成功与失败路径。
    #[test]
    fn test_data_version_dao_all_functions() {
        let connection = Connection::open_in_memory().unwrap();

        // exist_table 成功路径：表不存在时返回 false。
        assert!(!exist_table(&connection).unwrap());

        // select 失败路径：表不存在时报 DatabaseError。
        assert!(matches!(
            select(&connection),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // insert 失败路径：表不存在时报 DatabaseError。
        assert!(matches!(
            insert(
                &connection,
                &DataVersion {
                    major: 0,
                    minor: 0,
                    patch: 0
                }
            ),
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

        // insert 成功路径：插入后 select 能读到相同的数据版本。
        let version = DataVersion {
            major: 1,
            minor: 2,
            patch: 3,
        };
        insert(&connection, &version).unwrap();
        assert_eq!(select(&connection).unwrap(), vec![version]);

        // insert 属性：data_version 表允许插入多行（多行情况由 service 层校验）。
        insert(&connection, &version).unwrap();
        assert_eq!(select(&connection).unwrap().len(), 2);
    }
}
