use rusqlite::Connection;

use super::entity::DataVersion;
use crate::error_code::ErrorCode;
use crate::util::sea_query_util::values_to_params;
use sea_query::{
    Alias, Asterisk, ColumnDef, ColumnType, Expr, ExprTrait, Func, Query, SqliteQueryBuilder,
    Table,
};

/// data_version 表的标识符集合，作为 sea-query 构建语句时使用的受控术语表。
#[derive(sea_query::Iden)]
enum DataVersionIden {
    #[iden = "data_version"]
    Table,
    Major,
    Minor,
    Patch,
}

/// 新建 data_version 表。
///
/// # 参数
/// - `connection`: 数据库连接。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn create_table(connection: &Connection) -> Result<(), ErrorCode> {
    let table = Table::create()
        .table(DataVersionIden::Table)
        .col(
            ColumnDef::new_with_type(DataVersionIden::Major, ColumnType::custom("INTEGER"))
                .not_null(),
        )
        .col(
            ColumnDef::new_with_type(DataVersionIden::Minor, ColumnType::custom("INTEGER"))
                .not_null(),
        )
        .col(
            ColumnDef::new_with_type(DataVersionIden::Patch, ColumnType::custom("INTEGER"))
                .not_null(),
        )
        .extra("STRICT")
        .take();
    let sql = table.to_string(SqliteQueryBuilder);
    connection
        .execute(&sql, [])
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
    let query = Query::select()
        .expr(Func::count(Expr::col(Asterisk)))
        .from(Alias::new("sqlite_master"))
        .and_where(Expr::col(Alias::new("type")).eq("table"))
        .and_where(Expr::col(Alias::new("name")).eq("data_version"))
        .take();
    let (sql, values) = query.build(SqliteQueryBuilder);
    let count: i64 = connection
        .query_row(
            &sql,
            rusqlite::params_from_iter(values_to_params(values)),
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
    let query = Query::insert()
        .into_table(DataVersionIden::Table)
        .columns([DataVersionIden::Major, DataVersionIden::Minor, DataVersionIden::Patch])
        .values_panic([
            data_version.major.into(),
            data_version.minor.into(),
            data_version.patch.into(),
        ])
        .take();
    let (sql, values) = query.build(SqliteQueryBuilder);
    connection
        .execute(&sql, rusqlite::params_from_iter(values_to_params(values)))
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
    let query = Query::select()
        .columns([DataVersionIden::Major, DataVersionIden::Minor, DataVersionIden::Patch])
        .from(DataVersionIden::Table)
        .take();
    let (sql, values) = query.build(SqliteQueryBuilder);
    let mut statement = connection
        .prepare(&sql)
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    let rows = statement
        .query_map(rusqlite::params_from_iter(values_to_params(values)), |row| {
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

    /// STRICT 生效验证：向 INTEGER 列插入 TEXT 值时被数据库拒绝（非 STRICT 表会静默接受）。
    #[test]
    fn test_data_version_strict_type_enforced() {
        let connection = Connection::open_in_memory().unwrap();
        create_table(&connection).unwrap();
        assert!(matches!(
            connection.execute(
                "INSERT INTO data_version (major, minor, patch) VALUES ('not-an-integer', 0, 0)",
                [],
            ),
            Err(_)
        ));
    }
}
