use rusqlite::{Connection, OptionalExtension, Row};

use crate::business::user_database::canvas::response::CanvasColorEntry;
use crate::business::user_database::entity::Canvas;
use crate::error_code::ErrorCode;

/// 从查询结果行构造 Canvas。
fn map_row(row: &Row) -> rusqlite::Result<Canvas> {
    Ok(Canvas {
        id: row.get(0)?,
        parent_id: row.get(1)?,
        name: row.get(2)?,
        x: row.get(3)?,
        y: row.get(4)?,
        deleted: row.get::<_, i64>(5)? != 0,
        color: row.get(6)?,
    })
}

/// 新建 canvas 表。
///
/// # 参数
/// - `connection`: 数据库连接。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn create_table(connection: &Connection) -> Result<(), ErrorCode> {
    connection
        .execute(
            "CREATE TABLE canvas (
                id TEXT PRIMARY KEY,
                parent_id TEXT REFERENCES canvas(id) ON DELETE CASCADE,
                name TEXT NOT NULL UNIQUE,
                x REAL NOT NULL,
                y REAL NOT NULL,
                deleted INTEGER NOT NULL,
                color TEXT NOT NULL
            ) STRICT",
            [],
        )
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    Ok(())
}

/// 向 canvas 表插入一个画布。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `canvas`: 要插入的画布。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn insert(connection: &Connection, canvas: &Canvas) -> Result<(), ErrorCode> {
    connection
        .execute(
            "INSERT INTO canvas (id, parent_id, name, x, y, deleted, color)
            VALUES (:id, :parent_id, :name, :x, :y, :deleted, :color)",
            rusqlite::named_params! {
                ":id": canvas.id,
                ":parent_id": canvas.parent_id,
                ":name": canvas.name,
                ":x": canvas.x,
                ":y": canvas.y,
                ":deleted": canvas.deleted as i64,
                ":color": canvas.color,
            },
        )
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    Ok(())
}

/// 按 id 查询画布。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `id`: 画布 id。
///
/// # 返回值
/// 返回查询到的画布，不存在时返回 `None`；若发生错误则返回对应的 `ErrorCode`。
pub fn select_by_id(connection: &Connection, id: &str) -> Result<Option<Canvas>, ErrorCode> {
    connection
        .query_row(
            "SELECT id, parent_id, name, x, y, deleted, color
            FROM canvas
            WHERE id = :id",
            rusqlite::named_params! {":id": id},
            map_row,
        )
        .optional()
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })
}

/// 按名称查询画布。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `name`: 画布名称。
///
/// # 返回值
/// 返回查询到的画布，不存在时返回 `None`；若发生错误则返回对应的 `ErrorCode`。
pub fn select_by_name(connection: &Connection, name: &str) -> Result<Option<Canvas>, ErrorCode> {
    connection
        .query_row(
            "SELECT id, parent_id, name, x, y, deleted, color
            FROM canvas
            WHERE name = :name",
            rusqlite::named_params! {":name": name},
            map_row,
        )
        .optional()
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })
}

/// 查询全部画布（包含已逻辑删除的），按名称排序。
///
/// # 参数
/// - `connection`: 数据库连接。
///
/// # 返回值
/// 返回查询到的画布列表；若发生错误则返回对应的 `ErrorCode`。
pub fn select_all(connection: &Connection) -> Result<Vec<Canvas>, ErrorCode> {
    let mut statement = connection
        .prepare(
            "SELECT id, parent_id, name, x, y, deleted, color
            FROM canvas
            ORDER BY name ASC",
        )
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

/// 按逻辑删除标志查询画布，按名称排序。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `deleted`: 逻辑删除标志，false 返回正常画布，true 返回已逻辑删除的画布。
///
/// # 返回值
/// 返回查询到的画布列表；若发生错误则返回对应的 `ErrorCode`。
pub fn select_by_deleted(connection: &Connection, deleted: bool) -> Result<Vec<Canvas>, ErrorCode> {
    let mut statement = connection
        .prepare(
            "SELECT id, parent_id, name, x, y, deleted, color
            FROM canvas
            WHERE deleted = :deleted
            ORDER BY name ASC",
        )
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    let rows = statement
        .query_map(
            rusqlite::named_params! {":deleted": deleted as i64},
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

/// 更新一个画布（按 id 匹配，整行覆盖）。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `canvas`: 要更新的画布。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn update(connection: &Connection, canvas: &Canvas) -> Result<(), ErrorCode> {
    connection
        .execute(
            "UPDATE canvas
            SET parent_id = :parent_id,
                name = :name,
                x = :x,
                y = :y,
                deleted = :deleted,
                color = :color
            WHERE id = :id",
            rusqlite::named_params! {
                ":id": canvas.id,
                ":parent_id": canvas.parent_id,
                ":name": canvas.name,
                ":x": canvas.x,
                ":y": canvas.y,
                ":deleted": canvas.deleted as i64,
                ":color": canvas.color,
            },
        )
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    Ok(())
}

/// 按 id 删除一个画布。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `id`: 画布 id。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn delete_by_id(connection: &Connection, id: &str) -> Result<(), ErrorCode> {
    connection
        .execute(
            "DELETE FROM canvas
            WHERE id = :id",
            rusqlite::named_params! {":id": id},
        )
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    Ok(())
}

/// 查询根画布（parent_id 为 NULL 的画布）。
///
/// # 参数
/// - `connection`: 数据库连接。
///
/// # 返回值
/// 返回查询到的根画布，不存在时返回 `None`；若发生错误则返回对应的 `ErrorCode`。
pub fn select_root(connection: &Connection) -> Result<Option<Canvas>, ErrorCode> {
    connection
        .query_row(
            "SELECT id, parent_id, name, x, y, deleted, color
            FROM canvas
            WHERE parent_id IS NULL",
            [],
            map_row,
        )
        .optional()
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })
}

/// 批量移动画布坐标：只更新 x、y 两列，prepared statement 只 prepare 一次。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `items`: 要更新的画布列表，每项为 (id, x, y)。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn batch_move(connection: &Connection, items: &[(String, f64, f64)]) -> Result<(), ErrorCode> {
    let mut statement = connection
        .prepare("UPDATE canvas SET x = :x, y = :y WHERE id = :id")
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    for (id, x, y) in items {
        statement
            .execute(rusqlite::named_params! {
                ":id": id,
                ":x": x,
                ":y": y,
            })
            .map_err(|e| ErrorCode::DatabaseError {
                detail: e.to_string(),
            })?;
    }
    Ok(())
}

/// 查询所有未删除且设置了颜色的画布的名称、父画布 id 与颜色。
///
/// # 参数
/// - `connection`: 数据库连接。
///
/// # 返回值
/// 返回符合条件的画布颜色条目列表；若发生错误则返回对应的 `ErrorCode`。
pub fn select_colored(connection: &Connection) -> Result<Vec<CanvasColorEntry>, ErrorCode> {
    let mut statement = connection
        .prepare(
            "SELECT name, parent_id, color
            FROM canvas
            WHERE deleted = 0 AND color != ''",
        )
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    let rows = statement
        .query_map([], |row| {
            Ok(CanvasColorEntry {
                name: row.get(0)?,
                parent_id: row.get(1)?,
                color: row.get(2)?,
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

    /// 构造测试用 Canvas，各字段可由调用方再修改。
    fn canvas(id: &str, name: &str) -> Canvas {
        Canvas {
            id: id.to_string(),
            parent_id: None,
            name: name.to_string(),
            x: 0.0,
            y: 0.0,
            deleted: false,
            color: String::new(),
        }
    }

    /// 覆盖 canvas dao 模块所有 dao 函数的成功与失败路径。
    #[test]
    fn test_canvas_dao_all_functions() {
        let connection = Connection::open_in_memory().unwrap();
        // dao 单表测试聚焦本表 SQL，关闭外键以隔离父表依赖；外键级联行为由 service 测试端到端覆盖。
        connection
            .execute_batch("PRAGMA foreign_keys = OFF;")
            .unwrap();

        // insert 失败路径：表不存在时报 DatabaseError。
        assert!(matches!(
            insert(&connection, &canvas("id-1", "canvas-1")),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // select_colored 失败路径：表不存在时报 DatabaseError。
        assert!(matches!(
            select_colored(&connection),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // create_table 成功路径。
        create_table(&connection).unwrap();

        // create_table 失败路径：重复建表报 DatabaseError。
        assert!(matches!(
            create_table(&connection),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // insert 成功路径：插入后 select_by_id 与 select_by_name 均能查到。
        let mut first = canvas("id-1", "canvas-1");
        first.x = 1.5;
        first.y = -2.5;
        insert(&connection, &first).unwrap();
        let selected = select_by_id(&connection, "id-1").unwrap().unwrap();
        assert_eq!(selected.name, "canvas-1");
        assert_eq!(selected.x, 1.5);
        assert_eq!(selected.y, -2.5);
        assert!(selected.parent_id.is_none());
        assert!(!selected.deleted);
        assert_eq!(
            select_by_name(&connection, "canvas-1").unwrap().unwrap().id,
            "id-1"
        );

        // select_by_id / select_by_name 成功路径：不存在时返回 None。
        assert!(select_by_id(&connection, "id-x").unwrap().is_none());
        assert!(select_by_name(&connection, "canvas-x").unwrap().is_none());

        // insert 失败路径：id 重复时报 DatabaseError（主键约束）。
        assert!(matches!(
            insert(&connection, &canvas("id-1", "canvas-2")),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // insert 失败路径：name 重复时报 DatabaseError（唯一键约束）。
        assert!(matches!(
            insert(&connection, &canvas("id-2", "canvas-1")),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // select_all 成功路径：按名称排序返回全部画布。
        let mut second = canvas("id-2", "canvas-2");
        second.parent_id = Some("id-1".to_string());
        insert(&connection, &second).unwrap();
        let all = select_all(&connection).unwrap();
        let names: Vec<&str> = all.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, vec!["canvas-1", "canvas-2"]);

        // update 成功路径：整行覆盖后各字段均被更新。
        let mut first = select_by_id(&connection, "id-1").unwrap().unwrap();
        first.name = "canvas-1-renamed".to_string();
        first.x = 10.0;
        first.y = 20.0;
        first.deleted = true;
        update(&connection, &first).unwrap();
        let updated = select_by_id(&connection, "id-1").unwrap().unwrap();
        assert_eq!(updated.name, "canvas-1-renamed");
        assert_eq!(updated.x, 10.0);
        assert_eq!(updated.y, 20.0);
        assert!(updated.deleted);

        // select_by_deleted 成功路径：按逻辑删除标志分流，且各自按名称排序。
        let normal = select_by_deleted(&connection, false).unwrap();
        assert_eq!(normal.len(), 1);
        assert_eq!(normal[0].name, "canvas-2");
        let deleted = select_by_deleted(&connection, true).unwrap();
        assert_eq!(deleted.len(), 1);
        assert_eq!(deleted[0].name, "canvas-1-renamed");

        // update 失败路径：name 与其它画布重复时报 DatabaseError（唯一键约束）。
        let mut second = select_by_id(&connection, "id-2").unwrap().unwrap();
        second.name = "canvas-1-renamed".to_string();
        assert!(matches!(
            update(&connection, &second),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // delete_by_id 成功路径：删除后查不到该记录。
        delete_by_id(&connection, "id-1").unwrap();
        assert!(select_by_id(&connection, "id-1").unwrap().is_none());
        assert_eq!(select_all(&connection).unwrap().len(), 1);

        // select_colored 成功路径：插入带色画布与已删除带色画布，验证只返回未删除带色画布。
        let mut colored = canvas("id-3", "canvas-3");
        colored.color = "{\"fill\":\"#ff0000\"}".to_string();
        insert(&connection, &colored).unwrap();
        let mut deleted_colored = canvas("id-4", "canvas-4");
        deleted_colored.color = "{\"fill\":\"#00ff00\"}".to_string();
        deleted_colored.deleted = true;
        insert(&connection, &deleted_colored).unwrap();
        // canvas-2 为未删除无色画布，不应出现在结果中。
        let colored_entries = select_colored(&connection).unwrap();
        assert_eq!(colored_entries.len(), 1);
        assert_eq!(colored_entries[0].name, "canvas-3");
        assert!(colored_entries[0].parent_id.is_none());
        assert_eq!(colored_entries[0].color, "{\"fill\":\"#ff0000\"}");

        // ===== select_root 成功路径 =====
        // 建表后、未插入任何画布前，根画布不存在。
        let connection2 = Connection::open_in_memory().unwrap();
        connection2
            .execute_batch("PRAGMA foreign_keys = OFF;")
            .unwrap();
        create_table(&connection2).unwrap();
        assert!(select_root(&connection2).unwrap().is_none());

        // 插入根画布后能查到。
        let root = canvas("root-id", "root");
        insert(&connection2, &root).unwrap();
        let found = select_root(&connection2).unwrap().unwrap();
        assert_eq!(found.id, "root-id");
        assert!(found.parent_id.is_none());

        // ===== batch_move 成功路径 =====
        // 插入多行后批量移动，验证坐标更新且其它字段不变。
        let mut c1 = canvas("batch-1", "batch-canvas-1");
        c1.x = 1.0;
        c1.y = 2.0;
        insert(&connection2, &c1).unwrap();
        let mut c2 = canvas("batch-2", "batch-canvas-2");
        c2.x = 3.0;
        c2.y = 4.0;
        insert(&connection2, &c2).unwrap();

        let items = vec![
            ("batch-1".to_string(), 10.0, 20.0),
            ("batch-2".to_string(), 30.0, 40.0),
        ];
        batch_move(&connection2, &items).unwrap();

        let updated1 = select_by_id(&connection2, "batch-1").unwrap().unwrap();
        assert_eq!((updated1.x, updated1.y), (10.0, 20.0));
        assert_eq!(updated1.name, "batch-canvas-1");
        assert!(!updated1.deleted);
        let updated2 = select_by_id(&connection2, "batch-2").unwrap().unwrap();
        assert_eq!((updated2.x, updated2.y), (30.0, 40.0));
        assert_eq!(updated2.name, "batch-canvas-2");

        // batch_move 成功路径：不存在的 id 不会报错（SQLite UPDATE 不命中即 0 行），存在性校验是 service 层职责。
        let items_no_exist = vec![("no-such-id".to_string(), 5.0, 6.0)];
        batch_move(&connection2, &items_no_exist).unwrap();

        // batch_move 成功路径：空列表直接返回 Ok（no-op）。
        batch_move(&connection2, &[]).unwrap();

        // batch_move 失败路径：表不存在时报 DatabaseError。
        let connection3 = Connection::open_in_memory().unwrap();
        connection3
            .execute_batch("PRAGMA foreign_keys = OFF;")
            .unwrap();
        assert!(matches!(
            batch_move(
                &connection3,
                &[("any-id".to_string(), 0.0, 0.0)]
            ),
            Err(ErrorCode::DatabaseError { .. })
        ));
    }
}
