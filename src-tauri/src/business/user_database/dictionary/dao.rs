use rusqlite::{Connection, Row};

use crate::business::user_database::entity::Dictionary;
use crate::error_code::ErrorCode;

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
    connection
        .execute(
            "CREATE TABLE dictionary (
                id TEXT PRIMARY KEY,
                parent_id TEXT,
                value TEXT NOT NULL,
                \"order\" INTEGER NOT NULL
            ) STRICT",
            [],
        )
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
    let mut statement = connection
        .prepare(
            "INSERT INTO dictionary (id, parent_id, value, \"order\")
            VALUES (:id, :parent_id, :value, :order)",
        )
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    for dictionary in dictionaries {
        statement
            .execute(rusqlite::named_params! {
                ":id": dictionary.id,
                ":parent_id": dictionary.parent_id,
                ":value": dictionary.value,
                ":order": dictionary.order,
            })
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
    let mut statement = connection
        .prepare("SELECT id, parent_id, value, \"order\" FROM dictionary ORDER BY \"order\" ASC")
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    let rows = statement
        .query_map([], map_row)
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
    connection
        .execute("DELETE FROM dictionary", [])
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
    let count: i64 = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM dictionary WHERE id = :id)",
            rusqlite::named_params! {":id": id},
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
}
