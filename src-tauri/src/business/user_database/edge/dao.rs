use rusqlite::{Connection, OptionalExtension, Row};

use crate::business::user_database::entity::Edge;
use crate::error_code::ErrorCode;

/// 从查询结果行构造 Edge。
fn map_row(row: &Row) -> rusqlite::Result<Edge> {
    Ok(Edge {
        id: row.get(0)?,
        canvas_id: row.get(1)?,
        source_id: row.get(2)?,
        source_port: row.get(3)?,
        target_id: row.get(4)?,
        target_port: row.get(5)?,
        title: row.get(6)?,
        description: row.get(7)?,
    })
}

/// 新建 edge 表。
///
/// # 参数
/// - `connection`: 数据库连接。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn create_table(connection: &Connection) -> Result<(), ErrorCode> {
    connection
        .execute(
            "CREATE TABLE edge (
                id TEXT PRIMARY KEY,
                canvas_id TEXT NOT NULL REFERENCES canvas(id) ON DELETE CASCADE,
                source_id TEXT NOT NULL REFERENCES node(id) ON DELETE CASCADE,
                source_port TEXT NOT NULL,
                target_id TEXT NOT NULL REFERENCES node(id) ON DELETE CASCADE,
                target_port TEXT NOT NULL,
                title TEXT NOT NULL DEFAULT '',
                description TEXT NOT NULL DEFAULT '',
                UNIQUE (source_id, target_id)
            ) STRICT",
            [],
        )
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    Ok(())
}

/// 向 edge 表插入一条边。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `edge`: 要插入的边。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn insert(connection: &Connection, edge: &Edge) -> Result<(), ErrorCode> {
    connection
        .execute(
            "INSERT INTO edge (id, canvas_id, source_id, source_port, target_id, target_port, title, description)
            VALUES (:id, :canvas_id, :source_id, :source_port, :target_id, :target_port, :title, :description)",
            rusqlite::named_params! {
                ":id": edge.id,
                ":canvas_id": edge.canvas_id,
                ":source_id": edge.source_id,
                ":source_port": edge.source_port,
                ":target_id": edge.target_id,
                ":target_port": edge.target_port,
                ":title": edge.title,
                ":description": edge.description,
            },
        )
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    Ok(())
}

/// 按 id 查询边。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `id`: 边 id。
///
/// # 返回值
/// 返回查询到的边，不存在时返回 `None`；若发生错误则返回对应的 `ErrorCode`。
pub fn select_by_id(connection: &Connection, id: &str) -> Result<Option<Edge>, ErrorCode> {
    connection
        .query_row(
            "SELECT id, canvas_id, source_id, source_port, target_id, target_port, title, description
            FROM edge
            WHERE id = :id",
            rusqlite::named_params! {":id": id},
            map_row,
        )
        .optional()
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })
}

/// 更新边的标题和详情。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `id`: 边 id。
/// - `title`: 新标题。
/// - `description`: 新详情。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn update_title_and_description(
    connection: &Connection,
    id: &str,
    title: &str,
    description: &str,
) -> Result<(), ErrorCode> {
    connection
        .execute(
            "UPDATE edge
            SET title = :title, description = :description
            WHERE id = :id",
            rusqlite::named_params! {
                ":id": id,
                ":title": title,
                ":description": description,
            },
        )
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    Ok(())
}

/// 更新边的源节点连接桩和目标节点连接桩。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `id`: 边 id。
/// - `source_port`: 新源节点连接桩。
/// - `target_port`: 新目标节点连接桩。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn update_ports(
    connection: &Connection,
    id: &str,
    source_port: &str,
    target_port: &str,
) -> Result<(), ErrorCode> {
    connection
        .execute(
            "UPDATE edge SET source_port = :source_port, target_port = :target_port WHERE id = :id",
            rusqlite::named_params! {
                ":id": id,
                ":source_port": source_port,
                ":target_port": target_port,
            },
        )
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    Ok(())
}

/// 查询指定画布内的全部边。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `canvas_id`: 画布 id。
///
/// # 返回值
/// 返回查询到的边列表；若发生错误则返回对应的 `ErrorCode`。
pub fn select_by_canvas_id(
    connection: &Connection,
    canvas_id: &str,
) -> Result<Vec<Edge>, ErrorCode> {
    let mut statement = connection
        .prepare(
            "SELECT id, canvas_id, source_id, source_port, target_id, target_port, title, description
            FROM edge
            WHERE canvas_id = :canvas_id",
        )
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    let rows = statement
        .query_map(rusqlite::named_params! {":canvas_id": canvas_id}, map_row)
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })
}

/// 按 id 删除一条边。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `id`: 边 id。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn delete_by_id(connection: &Connection, id: &str) -> Result<(), ErrorCode> {
    connection
        .execute(
            "DELETE FROM edge
            WHERE id = :id",
            rusqlite::named_params! {":id": id},
        )
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    Ok(())
}

/// 批量更新边的画布归属：只更新 canvas_id 一列，prepared statement 只 prepare 一次。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `ids`: 要更新的边 id 列表。
/// - `canvas_id`: 目标画布 id。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn batch_update_canvas_id(
    connection: &Connection,
    ids: &[String],
    canvas_id: &str,
) -> Result<(), ErrorCode> {
    let mut statement = connection
        .prepare("UPDATE edge SET canvas_id = :canvas_id WHERE id = :id")
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    for id in ids {
        statement
            .execute(rusqlite::named_params! {
                ":id": id,
                ":canvas_id": canvas_id,
            })
            .map_err(|e| ErrorCode::DatabaseError {
                detail: e.to_string(),
            })?;
    }
    Ok(())
}

/// 判断两个节点之间是否已存在边（以源节点和目标节点精确匹配）。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `source_id`: 源节点 id。
/// - `target_id`: 目标节点 id。
///
/// # 返回值
/// 返回边是否存在的布尔值；若发生错误则返回对应的 `ErrorCode`。
pub fn exists_between(
    connection: &Connection,
    source_id: &str,
    target_id: &str,
) -> Result<bool, ErrorCode> {
    let count: i64 = connection
        .query_row(
            "SELECT COUNT(*)
            FROM edge
            WHERE source_id = :source_id AND target_id = :target_id",
            rusqlite::named_params! {
                ":source_id": source_id,
                ":target_id": target_id,
            },
            |row| row.get(0),
        )
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    Ok(count > 0)
}

/// 按源节点和目标节点精确查询边。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `source_id`: 源节点 id。
/// - `target_id`: 目标节点 id。
///
/// # 返回值
/// 返回查询到的边，不存在时返回 `None`；若发生错误则返回对应的 `ErrorCode`。
pub fn select_between(
    connection: &Connection,
    source_id: &str,
    target_id: &str,
) -> Result<Option<Edge>, ErrorCode> {
    connection
        .query_row(
            "SELECT id, canvas_id, source_id, source_port, target_id, target_port, title, description
            FROM edge
            WHERE source_id = :source_id AND target_id = :target_id",
            rusqlite::named_params! {":source_id": source_id, ":target_id": target_id},
            map_row,
        )
        .optional()
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造测试用 Edge，各字段可由调用方再修改。
    fn edge(id: &str, canvas_id: &str, source_id: &str, target_id: &str) -> Edge {
        Edge {
            id: id.to_string(),
            canvas_id: canvas_id.to_string(),
            source_id: source_id.to_string(),
            source_port: "right".to_string(),
            target_id: target_id.to_string(),
            target_port: "left".to_string(),
            title: String::new(),
            description: String::new(),
        }
    }

    /// 覆盖 edge dao 模块所有 dao 函数的成功与失败路径。
    #[test]
    fn test_edge_dao_all_functions() {
        let connection = Connection::open_in_memory().unwrap();
        // dao 单表测试聚焦本表 SQL，关闭外键以隔离父表依赖；外键级联行为由 service 测试端到端覆盖。
        connection
            .execute_batch("PRAGMA foreign_keys = OFF;")
            .unwrap();

        // insert 失败路径：表不存在时报 DatabaseError。
        assert!(matches!(
            insert(&connection, &edge("id-1", "canvas-1", "node-1", "node-2")),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // create_table 成功路径。
        create_table(&connection).unwrap();

        // create_table 失败路径：重复建表报 DatabaseError。
        assert!(matches!(
            create_table(&connection),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // insert 成功路径：插入后 select_by_id 能查到。
        insert(&connection, &edge("id-1", "canvas-1", "node-1", "node-2")).unwrap();
        let selected = select_by_id(&connection, "id-1").unwrap().unwrap();
        assert_eq!(selected.source_id, "node-1");
        assert_eq!(selected.target_id, "node-2");
        assert_eq!(selected.source_port, "right");

        // select_by_id 成功路径：不存在时返回 None。
        assert!(select_by_id(&connection, "id-x").unwrap().is_none());

        // insert 失败路径：id 重复时报 DatabaseError（主键约束）。
        assert!(matches!(
            insert(&connection, &edge("id-1", "canvas-1", "node-3", "node-4")),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // insert 失败路径：源节点和目标节点重复时报 DatabaseError（联合唯一键约束）。
        assert!(matches!(
            insert(&connection, &edge("id-2", "canvas-2", "node-1", "node-2")),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // exists_between 成功路径：存在的边返回 true，不存在或方向相反返回 false。
        assert!(exists_between(&connection, "node-1", "node-2").unwrap());
        assert!(!exists_between(&connection, "node-2", "node-1").unwrap());
        assert!(!exists_between(&connection, "node-1", "node-3").unwrap());

        // select_between 成功路径：精确匹配 source_id 与 target_id 时返回该行。
        let selected_between = select_between(&connection, "node-1", "node-2").unwrap().unwrap();
        assert_eq!(selected_between.id, "id-1");
        assert_eq!(selected_between.canvas_id, "canvas-1");
        assert_eq!(selected_between.source_id, "node-1");
        assert_eq!(selected_between.source_port, "right");
        assert_eq!(selected_between.target_id, "node-2");
        assert_eq!(selected_between.target_port, "left");

        // select_between 成功路径：方向相反时返回 None（精确匹配，不做反向查找）。
        assert!(select_between(&connection, "node-2", "node-1").unwrap().is_none());

        // select_between 成功路径：源节点或目标节点不存在时返回 None。
        assert!(select_between(&connection, "node-x", "node-2").unwrap().is_none());
        assert!(select_between(&connection, "node-1", "node-x").unwrap().is_none());

        // select_by_canvas_id 成功路径：只返回指定画布内的边。
        insert(&connection, &edge("id-2", "canvas-1", "node-2", "node-3")).unwrap();
        insert(&connection, &edge("id-3", "canvas-2", "node-3", "node-4")).unwrap();
        let edges = select_by_canvas_id(&connection, "canvas-1").unwrap();
        assert_eq!(edges.len(), 2);
        assert!(select_by_canvas_id(&connection, "canvas-x")
            .unwrap()
            .is_empty());

        // delete_by_id 成功路径：删除后查不到该记录。
        insert(&connection, &edge("id-4", "canvas-2", "node-4", "node-5")).unwrap();
        delete_by_id(&connection, "id-4").unwrap();
        assert!(select_by_id(&connection, "id-4").unwrap().is_none());

        // update_title_and_description 成功路径：标题和详情被更新。
        update_title_and_description(&connection, "id-1", "new title", "new desc").unwrap();
        let updated = select_by_id(&connection, "id-1").unwrap().unwrap();
        assert_eq!(updated.title, "new title");
        assert_eq!(updated.description, "new desc");

        // select_between 成功路径：更新后再次查询，标题与详情随更新结果返回。
        let updated_between = select_between(&connection, "node-1", "node-2").unwrap().unwrap();
        assert_eq!(updated_between.title, "new title");
        assert_eq!(updated_between.description, "new desc");

        // ===== update_ports 成功路径 =====
        // 更新连接桩后 select_by_id 往返一致，其它字段（canvas_id / source_id / target_id /
        // title / description）保持不变。
        update_ports(&connection, "id-1", "top", "bottom").unwrap();
        let ports_updated = select_by_id(&connection, "id-1").unwrap().unwrap();
        assert_eq!(ports_updated.source_port, "top");
        assert_eq!(ports_updated.target_port, "bottom");
        assert_eq!(ports_updated.canvas_id, "canvas-1");
        assert_eq!(ports_updated.source_id, "node-1");
        assert_eq!(ports_updated.target_id, "node-2");
        assert_eq!(ports_updated.title, "new title");
        assert_eq!(ports_updated.description, "new desc");

        // update_ports 幂等：连接桩完全相同的重复更新也成功，字段值不变。
        update_ports(&connection, "id-1", "top", "bottom").unwrap();
        let ports_idempotent = select_by_id(&connection, "id-1").unwrap().unwrap();
        assert_eq!(ports_idempotent.source_port, "top");
        assert_eq!(ports_idempotent.target_port, "bottom");

        // update_ports 成功路径：不存在的 id 不会报错（SQLite UPDATE 不命中即 0 行），存在性校验是 service 层职责。
        update_ports(&connection, "no-such-id", "top", "bottom").unwrap();

        // update_ports 失败路径：表不存在时报 DatabaseError。
        let connection3 = Connection::open_in_memory().unwrap();
        connection3
            .execute_batch("PRAGMA foreign_keys = OFF;")
            .unwrap();
        assert!(matches!(
            update_ports(&connection3, "any-id", "top", "bottom"),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // ===== batch_update_canvas_id 成功路径 =====
        // 先插一条带 title / description 的边，再更新其 canvas_id，验证 canvas_id 改变而其它字段不变。
        let mut titled = edge("id-buc", "canvas-1", "node-buc-src", "node-buc-tgt");
        titled.title = "titled edge".to_string();
        titled.description = "titled desc".to_string();
        insert(&connection, &titled).unwrap();
        batch_update_canvas_id(&connection, &["id-buc".to_string()], "canvas-2").unwrap();
        let updated_canvas = select_by_id(&connection, "id-buc").unwrap().unwrap();
        assert_eq!(updated_canvas.canvas_id, "canvas-2");
        assert_eq!(updated_canvas.source_id, "node-buc-src");
        assert_eq!(updated_canvas.target_id, "node-buc-tgt");
        assert_eq!(updated_canvas.title, "titled edge");
        assert_eq!(updated_canvas.description, "titled desc");

        // batch_update_canvas_id 成功路径：不存在的 id 不会报错（SQLite UPDATE 不命中即 0 行），存在性校验是 service 层职责。
        batch_update_canvas_id(&connection, &["no-such-id".to_string()], "canvas-2").unwrap();

        // batch_update_canvas_id 成功路径：空列表直接返回 Ok（no-op）。
        batch_update_canvas_id(&connection, &[], "canvas-2").unwrap();

        // batch_update_canvas_id 失败路径：表不存在时报 DatabaseError。
        let connection2 = Connection::open_in_memory().unwrap();
        connection2
            .execute_batch("PRAGMA foreign_keys = OFF;")
            .unwrap();
        assert!(matches!(
            batch_update_canvas_id(&connection2, &["any-id".to_string()], "canvas-x"),
            Err(ErrorCode::DatabaseError { .. })
        ));
    }
}
