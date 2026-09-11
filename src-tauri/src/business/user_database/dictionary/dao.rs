use rusqlite::{Connection, Row};

use crate::business::user_database::entity::Dictionary;
use crate::error_code::ErrorCode;
use crate::util::sea_query_util::values_to_params;
use sea_query::{ColumnDef, ColumnType, Expr, ExprTrait, Order, Query, SqliteQueryBuilder, Table};

/// dictionary 表的标识符集合，作为 sea-query 构建语句时使用的受控术语表。
/// pub(crate) 供 node_field 与 template 的 NOT IN 子查询引用。
#[derive(sea_query::Iden)]
pub(crate) enum DictionaryIden {
    #[iden = "dictionary"]
    Table,
    Id,
    ParentId,
    Value,
    Order,
}

/// 从查询结果行构造 Dictionary。
fn map_row(row: &Row) -> rusqlite::Result<Dictionary> {
    Ok(Dictionary {
        id: row.get(0)?,
        parent_id: row.get(1)?,
        value: row.get(2)?,
        order: row.get(3)?,
    })
}

/// 新建 dictionary 表。
///
/// # 参数
/// - `connection`: 数据库连接。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn create_table(connection: &Connection) -> Result<(), ErrorCode> {
    let table = Table::create()
        .table(DictionaryIden::Table)
        .col(
            ColumnDef::new_with_type(DictionaryIden::Id, ColumnType::custom("TEXT"))
                .primary_key()
                .not_null(),
        )
        .col(ColumnDef::new_with_type(DictionaryIden::ParentId, ColumnType::custom("TEXT")))
        .col(
            ColumnDef::new_with_type(DictionaryIden::Value, ColumnType::custom("TEXT")).not_null(),
        )
        .col(
            ColumnDef::new_with_type(DictionaryIden::Order, ColumnType::custom("INTEGER")).not_null(),
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

/// 向 dictionary 表批量插入字典条目。
///
/// 复用同一 prepared statement 逐条绑定执行，避免重复解析 SQL。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `dictionaries`: 要插入的字典条目列表。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn batch_insert(connection: &Connection, dictionaries: &[Dictionary]) -> Result<(), ErrorCode> {
    // 占位符顺序约定：build 产出的 SQL 中 ? 依次对应 columns 声明序（id, parent_id, value, order），
    // 因此循环内按位绑定 params![id, parent_id, value, order]。
    let (sql, _) = Query::insert()
        .into_table(DictionaryIden::Table)
        .columns([
            DictionaryIden::Id,
            DictionaryIden::ParentId,
            DictionaryIden::Value,
            DictionaryIden::Order,
        ])
        .values_panic(["".into(), None::<String>.into(), "".into(), 0i64.into()])
        .build(SqliteQueryBuilder);
    let mut statement = connection
        .prepare(&sql)
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    for dictionary in dictionaries {
        statement
            .execute(rusqlite::params![
                dictionary.id,
                dictionary.parent_id,
                dictionary.value,
                dictionary.order,
            ])
            .map_err(|e| ErrorCode::DatabaseError {
                detail: e.to_string(),
            })?;
    }
    Ok(())
}

/// 查询全部字典条目，按 "order" 升序。
///
/// # 参数
/// - `connection`: 数据库连接。
///
/// # 返回值
/// 返回查询到的字典条目列表；若发生错误则返回对应的 `ErrorCode`。
pub fn select_all(connection: &Connection) -> Result<Vec<Dictionary>, ErrorCode> {
    let query = Query::select()
        .columns([
            DictionaryIden::Id,
            DictionaryIden::ParentId,
            DictionaryIden::Value,
            DictionaryIden::Order,
        ])
        .from(DictionaryIden::Table)
        .order_by(DictionaryIden::Order, Order::Asc)
        .take();
    let (sql, values) = query.build(SqliteQueryBuilder);
    let mut statement = connection
        .prepare(&sql)
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    let rows = statement
        .query_map(rusqlite::params_from_iter(values_to_params(values)), map_row)
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })
}

/// 删除 dictionary 表中全部条目。
///
/// # 参数
/// - `connection`: 数据库连接。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn delete_all(connection: &Connection) -> Result<(), ErrorCode> {
    let query = Query::delete()
        .from_table(DictionaryIden::Table)
        .take();
    let (sql, values) = query.build(SqliteQueryBuilder);
    connection
        .execute(&sql, rusqlite::params_from_iter(values_to_params(values)))
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    Ok(())
}

/// 按 id 判断字典条目是否存在。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `id`: 字典条目 id。
///
/// # 返回值
/// 存在返回 `true`，不存在返回 `false`；若发生错误则返回对应的 `ErrorCode`。
pub fn exist_by_id(connection: &Connection, id: &str) -> Result<bool, ErrorCode> {
    let query = Query::select()
        .expr(Expr::exists(
            Query::select()
                .expr(Expr::val(1))
                .from(DictionaryIden::Table)
                .and_where(Expr::col(DictionaryIden::Id).eq(id))
                .take(),
        ))
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
    Ok(count != 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造测试用 Dictionary。
    fn dict(id: &str, parent_id: Option<&str>, value: &str, order: i64) -> Dictionary {
        Dictionary {
            id: id.to_string(),
            parent_id: parent_id.map(|s| s.to_string()),
            value: value.to_string(),
            order,
        }
    }

    /// 覆盖 dictionary dao 模块所有 dao 函数的成功与失败路径。
    #[test]
    fn test_dictionary_dao_all_functions() {
        let connection = Connection::open_in_memory().unwrap();

        // batch_insert 失败路径：表不存在时报 DatabaseError。
        assert!(matches!(
            batch_insert(&connection, &[dict("id-1", None, "val-1", 1)]),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // select_all 失败路径：表不存在时报 DatabaseError。
        assert!(matches!(
            select_all(&connection),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // delete_all 失败路径：表不存在时报 DatabaseError。
        assert!(matches!(
            delete_all(&connection),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // exist_by_id 失败路径：表不存在时报 DatabaseError。
        assert!(matches!(
            exist_by_id(&connection, "id-1"),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // create_table 成功路径。
        create_table(&connection).unwrap();

        // create_table 失败路径：重复建表报 DatabaseError。
        assert!(matches!(
            create_table(&connection),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // batch_insert 成功路径：空列表不产生任何写入。
        batch_insert(&connection, &[]).unwrap();
        assert!(select_all(&connection).unwrap().is_empty());

        // batch_insert 成功路径：批量插入后 select_all 能按 order 升序取回（插乱序验证排序）。
        batch_insert(
            &connection,
            &[
                dict("id-2", None, "val-2", 20),
                dict("id-1", Some("id-2"), "val-1", 10),
                dict("id-3", Some("id-2"), "val-3", 30),
            ],
        )
        .unwrap();
        let all = select_all(&connection).unwrap();
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].id, "id-1");
        assert_eq!(all[1].id, "id-2");
        assert_eq!(all[2].id, "id-3");

        // parent_id 为 Some 和 None 的条目往返一致。
        let root = select_all(&connection)
            .unwrap()
            .into_iter()
            .find(|d| d.id == "id-2")
            .unwrap();
        assert!(root.parent_id.is_none());
        let child = select_all(&connection)
            .unwrap()
            .into_iter()
            .find(|d| d.id == "id-1")
            .unwrap();
        assert_eq!(child.parent_id.as_deref(), Some("id-2"));

        // batch_insert 失败路径：id 与已有条目重复时报 DatabaseError（主键约束）。
        assert!(matches!(
            batch_insert(&connection, &[dict("id-1", None, "dup", 99)]),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // exist_by_id 成功路径：存在返回 true。
        assert!(exist_by_id(&connection, "id-1").unwrap());

        // exist_by_id 成功路径：不存在返回 false。
        assert!(!exist_by_id(&connection, "id-x").unwrap());

        // delete_all 成功路径：删除后 select_all 为空。
        delete_all(&connection).unwrap();
        assert!(select_all(&connection).unwrap().is_empty());
    }

    /// STRICT 生效验证：向 TEXT 列（value）插入 BLOB 值时被数据库拒绝（非 STRICT 表会静默接受）。
    #[test]
    fn test_dictionary_strict_type_enforced() {
        let connection = Connection::open_in_memory().unwrap();
        create_table(&connection).unwrap();
        assert!(matches!(
            connection.execute(
                "INSERT INTO dictionary (id, parent_id, value, \"order\")
                VALUES ('strict-violation', NULL, x'0102', 1)",
                [],
            ),
            Err(_)
        ));
    }
}