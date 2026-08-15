use rusqlite::{Connection, OptionalExtension, Row};

use super::entity::Metadata;
use crate::error_code::ErrorCode;

/// 从查询结果行构造 Metadata。
fn map_row(row: &Row) -> rusqlite::Result<Metadata> {
    Ok(Metadata {
        id: row.get(0)?,
        name: row.get(1)?,
        archived: row.get::<_, i64>(2)? != 0,
        create_time: row.get(3)?,
        modify_time: row.get(4)?,
        last_open_time: row.get(5)?,
    })
}

/// 新建 metadata 表。
///
/// # 参数
/// - `connection`: 数据库连接。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn create_table(connection: &Connection) -> Result<(), ErrorCode> {
    connection
        .execute(
            "CREATE TABLE metadata (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                archived INTEGER NOT NULL,
                create_time INTEGER NOT NULL,
                modify_time INTEGER NOT NULL,
                last_open_time INTEGER NOT NULL
            ) STRICT",
            [],
        )
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    Ok(())
}

/// 判断 metadata 表是否存在。
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
            WHERE type = 'table' AND name = 'metadata'",
            [],
            |row| row.get(0),
        )
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    Ok(count > 0)
}

/// 向 metadata 表插入一条用户数据库元数据。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `metadata`: 要插入的元数据。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn insert(connection: &Connection, metadata: &Metadata) -> Result<(), ErrorCode> {
    connection
        .execute(
            "INSERT INTO metadata (id, name, archived, create_time, modify_time, last_open_time)
            VALUES (:id, :name, :archived, :create_time, :modify_time, :last_open_time)",
            rusqlite::named_params! {
                ":id": metadata.id,
                ":name": metadata.name,
                ":archived": metadata.archived as i64,
                ":create_time": metadata.create_time,
                ":modify_time": metadata.modify_time,
                ":last_open_time": metadata.last_open_time,
            },
        )
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    Ok(())
}

/// 按 id 查询用户数据库元数据。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `id`: 数据库 id。
///
/// # 返回值
/// 返回查询到的元数据，不存在时返回 `None`；若发生错误则返回对应的 `ErrorCode`。
pub fn select_by_id(connection: &Connection, id: &str) -> Result<Option<Metadata>, ErrorCode> {
    connection
        .query_row(
            "SELECT id, name, archived, create_time, modify_time, last_open_time
            FROM metadata
            WHERE id = :id",
            rusqlite::named_params! {":id": id},
            map_row,
        )
        .optional()
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })
}

/// 按名称查询用户数据库元数据。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `name`: 数据库名称。
///
/// # 返回值
/// 返回查询到的元数据，不存在时返回 `None`；若发生错误则返回对应的 `ErrorCode`。
pub fn select_by_name(connection: &Connection, name: &str) -> Result<Option<Metadata>, ErrorCode> {
    connection
        .query_row(
            "SELECT id, name, archived, create_time, modify_time, last_open_time
            FROM metadata
            WHERE name = :name",
            rusqlite::named_params! {":name": name},
            map_row,
        )
        .optional()
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })
}

/// 按归档状态查询用户数据库元数据，按最后打开时间从大到小排序，
/// 最后打开时间相同的按 name 排序。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `archived`: 归档状态。
///
/// # 返回值
/// 返回查询到的元数据列表；若发生错误则返回对应的 `ErrorCode`。
pub fn select_by_archived(
    connection: &Connection,
    archived: bool,
) -> Result<Vec<Metadata>, ErrorCode> {
    let mut statement = connection
        .prepare(
            "SELECT id, name, archived, create_time, modify_time, last_open_time
            FROM metadata
            WHERE archived = :archived
            ORDER BY last_open_time DESC, name ASC",
        )
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    let rows = statement
        .query_map(
            rusqlite::named_params! {":archived": archived as i64},
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

/// 更新一条用户数据库元数据（按 id 匹配，整行覆盖）。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `metadata`: 要更新的元数据。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn update(connection: &Connection, metadata: &Metadata) -> Result<(), ErrorCode> {
    connection
        .execute(
            "UPDATE metadata
            SET name = :name,
                archived = :archived,
                create_time = :create_time,
                modify_time = :modify_time,
                last_open_time = :last_open_time
            WHERE id = :id",
            rusqlite::named_params! {
                ":id": metadata.id,
                ":name": metadata.name,
                ":archived": metadata.archived as i64,
                ":create_time": metadata.create_time,
                ":modify_time": metadata.modify_time,
                ":last_open_time": metadata.last_open_time,
            },
        )
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    Ok(())
}

/// 按 id 删除一条用户数据库元数据。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `id`: 数据库 id。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn delete_by_id(connection: &Connection, id: &str) -> Result<(), ErrorCode> {
    connection
        .execute(
            "DELETE FROM metadata
            WHERE id = :id",
            rusqlite::named_params! {":id": id},
        )
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造测试用 Metadata，各字段可由调用方再修改。
    fn metadata(id: &str, name: &str) -> Metadata {
        Metadata {
            id: id.to_string(),
            name: name.to_string(),
            archived: false,
            create_time: 100,
            modify_time: 100,
            last_open_time: 100,
        }
    }

    /// 覆盖 metadata dao 模块所有 dao 函数的成功与失败路径。
    #[test]
    fn test_metadata_dao_all_functions() {
        let connection = Connection::open_in_memory().unwrap();

        // exist_table 成功路径：表不存在时返回 false。
        assert!(!exist_table(&connection).unwrap());

        // insert 失败路径：表不存在时报 DatabaseError。
        assert!(matches!(
            insert(&connection, &metadata("id-1", "db-1")),
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

        // insert 成功路径：插入后 select_by_id 与 select_by_name 均能查到。
        insert(&connection, &metadata("id-1", "db-1")).unwrap();
        let selected = select_by_id(&connection, "id-1").unwrap().unwrap();
        assert_eq!(selected.name, "db-1");
        assert!(!selected.archived);
        assert_eq!(
            select_by_name(&connection, "db-1").unwrap().unwrap().id,
            "id-1"
        );

        // select_by_id / select_by_name 成功路径：不存在时返回 None。
        assert!(select_by_id(&connection, "id-x").unwrap().is_none());
        assert!(select_by_name(&connection, "db-x").unwrap().is_none());

        // insert 失败路径：id 重复时报 DatabaseError（主键约束）。
        assert!(matches!(
            insert(&connection, &metadata("id-1", "db-2")),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // insert 失败路径：name 重复时报 DatabaseError（唯一键约束）。
        assert!(matches!(
            insert(&connection, &metadata("id-2", "db-1")),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // select_by_archived 成功路径：按最后打开时间从大到小排序，
        // 最后打开时间相同的按 name 排序。
        let mut second = metadata("id-2", "db-2");
        second.last_open_time = 300;
        let mut third = metadata("id-3", "db-3");
        third.last_open_time = 200;
        insert(&connection, &second).unwrap();
        insert(&connection, &third).unwrap();
        let list = select_by_archived(&connection, false).unwrap();
        let ids: Vec<&str> = list.iter().map(|m| m.id.as_str()).collect();
        assert_eq!(ids, vec!["id-2", "id-3", "id-1"]);

        // update 成功路径：归档后 select_by_archived 能按归档状态分流。
        let mut first = select_by_id(&connection, "id-1").unwrap().unwrap();
        first.archived = true;
        update(&connection, &first).unwrap();
        assert_eq!(select_by_archived(&connection, false).unwrap().len(), 2);
        let archived_list = select_by_archived(&connection, true).unwrap();
        assert_eq!(archived_list.len(), 1);
        assert_eq!(archived_list[0].id, "id-1");
        assert!(archived_list[0].archived);

        // delete_by_id 成功路径：删除后查不到该记录。
        delete_by_id(&connection, "id-1").unwrap();
        assert!(select_by_id(&connection, "id-1").unwrap().is_none());
    }
}
