use rusqlite::{Connection, OptionalExtension, Row};

use crate::business::user_database::entity::Viewport;
use crate::error_code::ErrorCode;

/// 从查询结果行构造 Viewport。
fn map_row(row: &Row) -> rusqlite::Result<Viewport> {
    Ok(Viewport {
        canvas_id: row.get(0)?,
        x: row.get(1)?,
        y: row.get(2)?,
        zoom: row.get(3)?,
    })
}

/// 新建 viewport 表。
///
/// # 参数
/// - `connection`: 数据库连接。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn create_table(connection: &Connection) -> Result<(), ErrorCode> {
    connection
        .execute(
            "CREATE TABLE viewport (
                canvas_id TEXT PRIMARY KEY,
                x REAL NOT NULL,
                y REAL NOT NULL,
                zoom REAL NOT NULL
            ) STRICT",
            [],
        )
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    Ok(())
}

/// 插入或更新一个视口（按 canvas_id 匹配，存在时整行覆盖）。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `viewport`: 要插入或更新的视口。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn upsert(connection: &Connection, viewport: &Viewport) -> Result<(), ErrorCode> {
    connection
        .execute(
            "INSERT OR REPLACE INTO viewport (canvas_id, x, y, zoom)
            VALUES (:canvas_id, :x, :y, :zoom)",
            rusqlite::named_params! {
                ":canvas_id": viewport.canvas_id,
                ":x": viewport.x,
                ":y": viewport.y,
                ":zoom": viewport.zoom,
            },
        )
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    Ok(())
}

/// 按 canvas_id 查询视口。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `canvas_id`: 画布 id 或画布宇宙视口特殊值。
///
/// # 返回值
/// 返回查询到的视口，不存在时返回 `None`；若发生错误则返回对应的 `ErrorCode`。
pub fn select_by_canvas_id(
    connection: &Connection,
    canvas_id: &str,
) -> Result<Option<Viewport>, ErrorCode> {
    connection
        .query_row(
            "SELECT canvas_id, x, y, zoom
            FROM viewport
            WHERE canvas_id = :canvas_id",
            rusqlite::named_params! {":canvas_id": canvas_id},
            map_row,
        )
        .optional()
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })
}

/// 删除指定画布的视口。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `canvas_id`: 画布 id。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn delete_by_canvas_id(connection: &Connection, canvas_id: &str) -> Result<(), ErrorCode> {
    connection
        .execute(
            "DELETE FROM viewport
            WHERE canvas_id = :canvas_id",
            rusqlite::named_params! {":canvas_id": canvas_id},
        )
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造测试用 Viewport，各字段可由调用方再修改。
    fn viewport(canvas_id: &str) -> Viewport {
        Viewport {
            canvas_id: canvas_id.to_string(),
            x: 0.0,
            y: 0.0,
            zoom: 1.0,
        }
    }

    /// 覆盖 viewport dao 模块所有 dao 函数的成功与失败路径。
    #[test]
    fn test_viewport_dao_all_functions() {
        let connection = Connection::open_in_memory().unwrap();

        // upsert 失败路径：表不存在时报 DatabaseError。
        assert!(matches!(
            upsert(&connection, &viewport("canvas-1")),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // select_by_canvas_id 失败路径：表不存在时报 DatabaseError。
        assert!(matches!(
            select_by_canvas_id(&connection, "canvas-1"),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // delete_by_canvas_id 失败路径：表不存在时报 DatabaseError。
        assert!(matches!(
            delete_by_canvas_id(&connection, "canvas-1"),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // create_table 成功路径。
        create_table(&connection).unwrap();

        // create_table 失败路径：重复建表报 DatabaseError。
        assert!(matches!(
            create_table(&connection),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // select_by_canvas_id 成功路径：不存在时返回 None。
        assert!(select_by_canvas_id(&connection, "canvas-1")
            .unwrap()
            .is_none());

        // upsert 成功路径：插入后能查到。
        let mut first = viewport("canvas-1");
        first.x = 1.5;
        first.y = -2.5;
        first.zoom = 2.0;
        upsert(&connection, &first).unwrap();
        let selected = select_by_canvas_id(&connection, "canvas-1")
            .unwrap()
            .unwrap();
        assert_eq!(selected.x, 1.5);
        assert_eq!(selected.y, -2.5);
        assert_eq!(selected.zoom, 2.0);

        // upsert 成功路径：canvas_id 相同时整行覆盖。
        let mut replaced = viewport("canvas-1");
        replaced.x = 10.0;
        replaced.zoom = 0.5;
        upsert(&connection, &replaced).unwrap();
        let selected = select_by_canvas_id(&connection, "canvas-1")
            .unwrap()
            .unwrap();
        assert_eq!(selected.x, 10.0);
        assert_eq!(selected.y, 0.0);
        assert_eq!(selected.zoom, 0.5);

        // upsert 成功路径：不同 canvas_id 互不影响。
        upsert(&connection, &viewport("canvas-2")).unwrap();
        assert_eq!(
            select_by_canvas_id(&connection, "canvas-1")
                .unwrap()
                .unwrap()
                .x,
            10.0
        );
        assert!(select_by_canvas_id(&connection, "canvas-2")
            .unwrap()
            .is_some());

        // delete_by_canvas_id 成功路径：指定画布的视口被删除，其它画布不受影响。
        delete_by_canvas_id(&connection, "canvas-1").unwrap();
        assert!(select_by_canvas_id(&connection, "canvas-1")
            .unwrap()
            .is_none());
        assert!(select_by_canvas_id(&connection, "canvas-2")
            .unwrap()
            .is_some());
    }
}
