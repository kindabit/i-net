use rusqlite::{Connection, OptionalExtension};

use crate::error_code::ErrorCode;
use crate::util::sea_query_util::values_to_params;
use sea_query::{
    Alias, Asterisk, ColumnDef, ColumnType, Expr, ExprTrait, Func, OnConflict, Query,
    SqliteQueryBuilder, Table,
};

/// variable 表的标识符集合，作为 sea-query 构建语句时使用的受控术语表。
#[derive(sea_query::Iden)]
enum VariableIden {
    #[iden = "variable"]
    Table,
    Name,
    Value,
}

/// 新建 variable 表。
///
/// # 参数
/// - `connection`: 数据库连接。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn create_table(connection: &Connection) -> Result<(), ErrorCode> {
    let table = Table::create()
        .table(VariableIden::Table)
        .col(
            ColumnDef::new_with_type(VariableIden::Name, ColumnType::custom("TEXT")).primary_key(),
        )
        .col(
            ColumnDef::new_with_type(VariableIden::Value, ColumnType::custom("TEXT")).not_null(),
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

/// 判断 variable 表是否存在。
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
        .and_where(Expr::col(Alias::new("name")).eq("variable"))
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
    let query = Query::insert()
        .into_table(VariableIden::Table)
        .columns([VariableIden::Name, VariableIden::Value])
        .values_panic([name.into(), value.into()])
        .on_conflict(
            OnConflict::column(VariableIden::Name)
                .update_column(VariableIden::Value)
                .to_owned(),
        )
        .take();
    let (sql, values) = query.build(SqliteQueryBuilder);
    connection
        .execute(&sql, rusqlite::params_from_iter(values_to_params(values)))
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
    let query = Query::select()
        .column(VariableIden::Value)
        .from(VariableIden::Table)
        .and_where(Expr::col(VariableIden::Name).eq(name))
        .take();
    let (sql, values) = query.build(SqliteQueryBuilder);
    connection
        .query_row(
            &sql,
            rusqlite::params_from_iter(values_to_params(values)),
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

    /// STRICT 生效验证：向 TEXT 列插入 BLOB 值时被数据库拒绝（非 STRICT 表会静默接受）。
    /// 注意：整数/实数会被 STRICT 表的亲和性规则无损转换为文本，因此用 BLOB 触发类型不匹配。
    #[test]
    fn test_variable_strict_type_enforced() {
        let connection = Connection::open_in_memory().unwrap();
        create_table(&connection).unwrap();
        assert!(matches!(
            connection.execute(
                "INSERT INTO variable (name, value) VALUES ('strict-violation', x'0102')",
                [],
            ),
            Err(_)
        ));
    }
}
