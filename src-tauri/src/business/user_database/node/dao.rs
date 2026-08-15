use rusqlite::{Connection, OptionalExtension, Row};

use crate::business::user_database::entity::Node;
use crate::business::user_database::node::response::NodeColorEntry;
use crate::business::user_database::node::response::NodeSearchResponse;
use crate::error_code::ErrorCode;

/// 从查询结果行构造 Node。
fn map_row(row: &Row) -> rusqlite::Result<Node> {
    Ok(Node {
        id: row.get(0)?,
        canvas_id: row.get(1)?,
        x: row.get(2)?,
        y: row.get(3)?,
        title: row.get(4)?,
        sub_title: row.get(5)?,
        canvas_ref_id: row.get(6)?,
        deleted: row.get::<_, i64>(7)? != 0,
        color: row.get(8)?,
    })
}

/// 新建 node 表。
///
/// # 参数
/// - `connection`: 数据库连接。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn create_table(connection: &Connection) -> Result<(), ErrorCode> {
    connection
        .execute(
            "CREATE TABLE node (
                id TEXT PRIMARY KEY,
                canvas_id TEXT NOT NULL REFERENCES canvas(id) ON DELETE CASCADE,
                x REAL NOT NULL,
                y REAL NOT NULL,
                title TEXT NOT NULL,
                sub_title TEXT NOT NULL,
                canvas_ref_id TEXT UNIQUE REFERENCES canvas(id) ON DELETE CASCADE,
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

/// 向 node 表插入一个节点。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `node`: 要插入的节点。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn insert(connection: &Connection, node: &Node) -> Result<(), ErrorCode> {
    connection
        .execute(
            "INSERT INTO node (id, canvas_id, x, y, title, sub_title, canvas_ref_id, deleted, color)
            VALUES (:id, :canvas_id, :x, :y, :title, :sub_title, :canvas_ref_id, :deleted, :color)",
            rusqlite::named_params! {
                ":id": node.id,
                ":canvas_id": node.canvas_id,
                ":x": node.x,
                ":y": node.y,
                ":title": node.title,
                ":sub_title": node.sub_title,
                ":canvas_ref_id": node.canvas_ref_id,
                ":deleted": node.deleted as i64,
                ":color": node.color,
            },
        )
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    Ok(())
}

/// 按 id 查询节点。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `id`: 节点 id。
///
/// # 返回值
/// 返回查询到的节点，不存在时返回 `None`；若发生错误则返回对应的 `ErrorCode`。
pub fn select_by_id(connection: &Connection, id: &str) -> Result<Option<Node>, ErrorCode> {
    connection
        .query_row(
            "SELECT id, canvas_id, x, y, title, sub_title, canvas_ref_id, deleted, color
            FROM node
            WHERE id = :id",
            rusqlite::named_params! {":id": id},
            map_row,
        )
        .optional()
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })
}

/// 按画布 id 和逻辑删除标志查询节点。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `canvas_id`: 画布 id。
/// - `deleted`: 逻辑删除标志。
///
/// # 返回值
/// 返回查询到的节点列表；若发生错误则返回对应的 `ErrorCode`。
pub fn select_by_canvas_id_and_deleted(
    connection: &Connection,
    canvas_id: &str,
    deleted: bool,
) -> Result<Vec<Node>, ErrorCode> {
    let mut statement = connection
        .prepare(
            "SELECT id, canvas_id, x, y, title, sub_title, canvas_ref_id, deleted, color
            FROM node
            WHERE canvas_id = :canvas_id AND deleted = :deleted",
        )
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    let rows = statement
        .query_map(
            rusqlite::named_params! {
                ":canvas_id": canvas_id,
                ":deleted": deleted as i64,
            },
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

/// 更新一个节点（按 id 匹配，整行覆盖）。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `node`: 要更新的节点。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn update(connection: &Connection, node: &Node) -> Result<(), ErrorCode> {
    connection
        .execute(
            "UPDATE node
            SET canvas_id = :canvas_id,
                x = :x,
                y = :y,
                title = :title,
                sub_title = :sub_title,
                canvas_ref_id = :canvas_ref_id,
                deleted = :deleted,
                color = :color
            WHERE id = :id",
            rusqlite::named_params! {
                ":id": node.id,
                ":canvas_id": node.canvas_id,
                ":x": node.x,
                ":y": node.y,
                ":title": node.title,
                ":sub_title": node.sub_title,
                ":canvas_ref_id": node.canvas_ref_id,
                ":deleted": node.deleted as i64,
                ":color": node.color,
            },
        )
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    Ok(())
}

/// 按 id 删除一个节点。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `id`: 节点 id。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn delete_by_id(connection: &Connection, id: &str) -> Result<(), ErrorCode> {
    connection
        .execute(
            "DELETE FROM node
            WHERE id = :id",
            rusqlite::named_params! {":id": id},
        )
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    Ok(())
}

/// 按 canvas_ref_id 查询节点。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `canvas_ref_id`: 节点引用的子画布 id。
///
/// # 返回值
/// 返回查询到的节点，不存在时返回 `None`；若发生错误则返回对应的 `ErrorCode`。
pub fn select_by_canvas_ref_id(
    connection: &Connection,
    canvas_ref_id: &str,
) -> Result<Option<Node>, ErrorCode> {
    connection
        .query_row(
            "SELECT id, canvas_id, x, y, title, sub_title, canvas_ref_id, deleted, color
            FROM node
            WHERE canvas_ref_id = :canvas_ref_id",
            rusqlite::named_params! {":canvas_ref_id": canvas_ref_id},
            map_row,
        )
        .optional()
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })
}

/// 从查询结果行构造 NodeSearchResponse。
fn map_search_row(row: &Row) -> rusqlite::Result<NodeSearchResponse> {
    Ok(NodeSearchResponse {
        id: row.get(0)?,
        canvas_id: row.get(1)?,
        x: row.get(2)?,
        y: row.get(3)?,
        title: row.get(4)?,
        sub_title: row.get(5)?,
        canvas_ref_id: row.get(6)?,
        canvas_name: row.get(7)?,
    })
}

/// 转义 LIKE 模式中的特殊字符（`\`、`%`、`_`），使它们按字面字符匹配。
fn escape_like_pattern(pattern: &str) -> String {
    pattern.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_")
}

/// 按关键词列表在所有画布中搜索节点（AND 语义）。
///
/// 每个关键词独立匹配节点标题、节点副标题或所在画布名称（OR），关键词之间为 AND 关系。
/// 逻辑删除的节点与逻辑删除的画布内的节点均被排除。
/// 结果按画布名称、节点标题排序，最多返回 50 条。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `keywords`: 预处理后的关键词列表，调用方保证非空且每个关键词非空。
///
/// # 返回值
/// 返回搜索结果列表；若发生错误则返回对应的 `ErrorCode`。
pub fn search_by_keywords(
    connection: &Connection,
    keywords: &[String],
) -> Result<Vec<NodeSearchResponse>, ErrorCode> {
    let mut sql = String::from(
        "SELECT node.id, node.canvas_id, node.x, node.y, node.title, node.sub_title, node.canvas_ref_id, canvas.name
         FROM node
         JOIN canvas ON node.canvas_id = canvas.id
         WHERE node.deleted = 0 AND canvas.deleted = 0",
    );
    let mut params: Vec<String> = Vec::new();
    for keyword in keywords {
        let escaped = escape_like_pattern(keyword);
        let pattern = format!("%{escaped}%");
        sql.push_str(
            " AND (node.title LIKE ? ESCAPE '\\' OR node.sub_title LIKE ? ESCAPE '\\' OR canvas.name LIKE ? ESCAPE '\\')",
        );
        params.push(pattern.clone());
        params.push(pattern.clone());
        params.push(pattern);
    }
    sql.push_str(" ORDER BY canvas.name, node.title LIMIT 50");
    let mut statement = connection
        .prepare(&sql)
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    let rows = statement
        .query_map(rusqlite::params_from_iter(params.iter()), map_search_row)
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })
}

/// 批量移动节点坐标：只更新 x、y 两列，prepared statement 只 prepare 一次。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `items`: 要更新的节点列表，每项为 (id, x, y)。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn batch_move(connection: &Connection, items: &[(String, f64, f64)]) -> Result<(), ErrorCode> {
    let mut statement = connection
        .prepare("UPDATE node SET x = :x, y = :y WHERE id = :id")
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

/// 查询所有未删除且设置了颜色的节点的标题与颜色。
///
/// # 参数
/// - `connection`: 数据库连接。
///
/// # 返回值
/// 返回符合条件的节点颜色条目列表；若发生错误则返回对应的 `ErrorCode`。
pub fn select_colored(connection: &Connection) -> Result<Vec<NodeColorEntry>, ErrorCode> {
    let mut statement = connection
        .prepare(
            "SELECT title, color
            FROM node
            WHERE deleted = 0 AND color != ''",
        )
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    let rows = statement
        .query_map([], |row| {
            Ok(NodeColorEntry {
                title: row.get(0)?,
                color: row.get(1)?,
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

    /// 构造测试用 Node，各字段可由调用方再修改。
    fn node(id: &str, canvas_id: &str) -> Node {
        Node {
            id: id.to_string(),
            canvas_id: canvas_id.to_string(),
            x: 0.0,
            y: 0.0,
            title: format!("title-{id}"),
            sub_title: format!("sub-title-{id}"),
            canvas_ref_id: None,
            deleted: false,
            color: String::new(),
        }
    }

    /// 覆盖 node dao 模块所有 dao 函数的成功与失败路径。
    #[test]
    fn test_node_dao_all_functions() {
        let connection = Connection::open_in_memory().unwrap();
        // dao 单表测试聚焦本表 SQL，关闭外键以隔离父表依赖；外键级联行为由 service 测试端到端覆盖。
        connection
            .execute_batch("PRAGMA foreign_keys = OFF;")
            .unwrap();

        // insert 失败路径：表不存在时报 DatabaseError。
        assert!(matches!(
            insert(&connection, &node("id-1", "canvas-1")),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // search_by_keywords 失败路径：node 表与 canvas 表不存在时报 DatabaseError。
        assert!(matches!(
            search_by_keywords(&connection, &["keyword".to_string()]),
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
        insert(&connection, &node("id-1", "canvas-1")).unwrap();
        let selected = select_by_id(&connection, "id-1").unwrap().unwrap();
        assert_eq!(selected.canvas_id, "canvas-1");
        assert_eq!(selected.title, "title-id-1");
        assert!(!selected.deleted);

        // select_by_id 成功路径：不存在时返回 None。
        assert!(select_by_id(&connection, "id-x").unwrap().is_none());

        // insert 失败路径：id 重复时报 DatabaseError（主键约束）。
        assert!(matches!(
            insert(&connection, &node("id-1", "canvas-2")),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // select_by_canvas_id_and_deleted 成功路径：按画布 id 和逻辑删除标志分流。
        let mut second = node("id-2", "canvas-1");
        second.deleted = true;
        insert(&connection, &second).unwrap();
        insert(&connection, &node("id-3", "canvas-2")).unwrap();
        let normal = select_by_canvas_id_and_deleted(&connection, "canvas-1", false).unwrap();
        assert_eq!(normal.len(), 1);
        assert_eq!(normal[0].id, "id-1");
        let deleted = select_by_canvas_id_and_deleted(&connection, "canvas-1", true).unwrap();
        assert_eq!(deleted.len(), 1);
        assert_eq!(deleted[0].id, "id-2");
        assert!(
            select_by_canvas_id_and_deleted(&connection, "canvas-x", false)
                .unwrap()
                .is_empty()
        );

        // update 成功路径：整行覆盖后各字段均被更新。
        let mut first = select_by_id(&connection, "id-1").unwrap().unwrap();
        first.x = 10.0;
        first.y = 20.0;
        first.title = "title-updated".to_string();
        first.sub_title = "sub-title-updated".to_string();
        first.deleted = true;
        update(&connection, &first).unwrap();
        let updated = select_by_id(&connection, "id-1").unwrap().unwrap();
        assert_eq!(updated.x, 10.0);
        assert_eq!(updated.title, "title-updated");
        assert!(updated.deleted);

        // delete_by_id 成功路径：删除后查不到该记录。
        delete_by_id(&connection, "id-1").unwrap();
        assert!(select_by_id(&connection, "id-1").unwrap().is_none());

        // canvas_ref_id 读写成功路径：设置后查询往返一致。
        let canvas_ref = uuid::Uuid::new_v4().to_string();
        let mut first = node("id-r1", "canvas-r");
        first.canvas_ref_id = Some(canvas_ref.clone());
        insert(&connection, &first).unwrap();
        let selected = select_by_id(&connection, "id-r1").unwrap().unwrap();
        assert_eq!(selected.canvas_ref_id.as_deref(), Some(canvas_ref.as_str()));

        // select_by_canvas_ref_id 命中：查到该节点。
        let found = select_by_canvas_ref_id(&connection, &canvas_ref).unwrap().unwrap();
        assert_eq!(found.id, "id-r1");

        // select_by_canvas_ref_id 未命中：返回 None。
        assert!(select_by_canvas_ref_id(&connection, "no-such-ref").unwrap().is_none());

        // canvas_ref_id UNIQUE 失败路径：两个节点引用同一个 canvas_ref_id 时报 DatabaseError。
        let mut second = node("id-r2", "canvas-r");
        second.canvas_ref_id = Some(canvas_ref.clone());
        assert!(matches!(
            insert(&connection, &second),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // 多个 NULL canvas_ref_id 成功路径：多个数据节点（canvas_ref_id 为 None）可共存。
        let mut third = node("id-r3", "canvas-r");
        third.canvas_ref_id = None;
        insert(&connection, &third).unwrap();
        assert!(select_by_id(&connection, "id-r3").unwrap().is_some());
        assert!(select_by_canvas_ref_id(&connection, "no-such-ref").unwrap().is_none());

        // canvas_ref_id 通过 update 写回：更新后再读出验证。
        let mut updated = select_by_id(&connection, "id-r3").unwrap().unwrap();
        let new_ref = uuid::Uuid::new_v4().to_string();
        updated.canvas_ref_id = Some(new_ref.clone());
        update(&connection, &updated).unwrap();
        let after = select_by_id(&connection, "id-r3").unwrap().unwrap();
        assert_eq!(after.canvas_ref_id.as_deref(), Some(new_ref.as_str()));

        // update canvas_ref_id 为 None：可正常清空。
        let mut cleared = select_by_id(&connection, "id-r3").unwrap().unwrap();
        cleared.canvas_ref_id = None;
        update(&connection, &cleared).unwrap();
        let after = select_by_id(&connection, "id-r3").unwrap().unwrap();
        assert!(after.canvas_ref_id.is_none());

        // canvas_ref_id 为 None 时 select_by_canvas_ref_id 未命中（SQLite NULL ≠ NULL）。
        assert!(select_by_canvas_ref_id(&connection, "").unwrap().is_none());

        // ===== search_by_keywords 成功路径 =====
        // 搜索段画布与节点统一使用 s- 前缀，与上文流程节点隔离，
        // 避免上文流程中未删除节点（如 canvas-2 中的 id-3）被 INNER JOIN 命中而干扰断言。
        crate::business::user_database::canvas::dao::create_table(&connection).unwrap();

        // 准备画布数据。
        let canvas1 = crate::business::user_database::entity::Canvas {
            id: "s-canvas-1".to_string(),
            parent_id: None,
            name: "Alpha Canvas".to_string(),
            x: 0.0,
            y: 0.0,
            deleted: false,
            color: String::new(),
        };
        crate::business::user_database::canvas::dao::insert(&connection, &canvas1).unwrap();
        let canvas2 = crate::business::user_database::entity::Canvas {
            id: "s-canvas-2".to_string(),
            parent_id: None,
            name: "Beta Canvas".to_string(),
            x: 0.0,
            y: 0.0,
            deleted: false,
            color: String::new(),
        };
        crate::business::user_database::canvas::dao::insert(&connection, &canvas2).unwrap();

        // 准备节点数据。
        let mut node1 = node("s-node-1", "s-canvas-1");
        node1.title = "Rust Programming".to_string();
        node1.sub_title = "systems language".to_string();
        insert(&connection, &node1).unwrap();
        let mut node2 = node("s-node-2", "s-canvas-1");
        node2.title = "Python Script".to_string();
        node2.sub_title = "dynamic language".to_string();
        insert(&connection, &node2).unwrap();
        let mut node3 = node("s-node-3", "s-canvas-2");
        node3.title = "JavaScript Web".to_string();
        node3.sub_title = "frontend language".to_string();
        insert(&connection, &node3).unwrap();

        // ===== batch_move 成功路径 =====
        // 插入多行后批量移动，验证坐标更新且其它字段不变。
        let mut n1 = node("batch-1", "canvas-1");
        n1.x = 1.0;
        n1.y = 2.0;
        insert(&connection, &n1).unwrap();
        let mut n2 = node("batch-2", "canvas-1");
        n2.x = 3.0;
        n2.y = 4.0;
        insert(&connection, &n2).unwrap();

        let items = vec![
            ("batch-1".to_string(), 10.0, 20.0),
            ("batch-2".to_string(), 30.0, 40.0),
        ];
        batch_move(&connection, &items).unwrap();

        let updated1 = select_by_id(&connection, "batch-1").unwrap().unwrap();
        assert_eq!((updated1.x, updated1.y), (10.0, 20.0));
        assert_eq!(updated1.title, "title-batch-1");
        assert_eq!(updated1.sub_title, "sub-title-batch-1");
        assert!(!updated1.deleted);
        let updated2 = select_by_id(&connection, "batch-2").unwrap().unwrap();
        assert_eq!((updated2.x, updated2.y), (30.0, 40.0));
        assert_eq!(updated2.title, "title-batch-2");

        // batch_move 成功路径：不存在的 id 不会报错（SQLite UPDATE 不命中即 0 行），存在性校验是 service 层职责。
        let items_no_exist = vec![("no-such-id".to_string(), 5.0, 6.0)];
        batch_move(&connection, &items_no_exist).unwrap();

        // batch_move 成功路径：空列表直接返回 Ok（no-op）。
        batch_move(&connection, &[]).unwrap();

        // batch_move 失败路径：表不存在时报 DatabaseError。
        let connection2 = Connection::open_in_memory().unwrap();
        connection2
            .execute_batch("PRAGMA foreign_keys = OFF;")
            .unwrap();
        assert!(matches!(
            batch_move(
                &connection2,
                &[("any-id".to_string(), 0.0, 0.0)]
            ),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // search_by_keywords 成功路径：单个关键词命中节点标题。
        let results = search_by_keywords(&connection, &["Rust".to_string()]).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "s-node-1");
        assert_eq!(results[0].title, "Rust Programming");

        // search_by_keywords 成功路径：单个关键词命中节点副标题。
        let results = search_by_keywords(&connection, &["dynamic".to_string()]).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "s-node-2");

        // search_by_keywords 成功路径：单个关键词命中画布名称。
        let results = search_by_keywords(&connection, &["Beta".to_string()]).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "s-node-3");
        assert_eq!(results[0].canvas_name, "Beta Canvas");

        // search_by_keywords 成功路径：两个关键词 AND 语义（同时满足才命中）。
        let results = search_by_keywords(&connection, &["Rust".to_string(), "systems".to_string()]).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "s-node-1");

        // search_by_keywords 成功路径：两个关键词 AND 语义，不同时满足则不命中。
        let results = search_by_keywords(&connection, &["Rust".to_string(), "dynamic".to_string()]).unwrap();
        assert!(results.is_empty());

        // search_by_keywords 成功路径：逻辑删除的节点不出现在结果中。
        let mut deleted_node = node("s-node-deleted", "s-canvas-1");
        deleted_node.title = "Rust Deleted".to_string();
        deleted_node.deleted = true;
        insert(&connection, &deleted_node).unwrap();
        let results = search_by_keywords(&connection, &["Rust".to_string()]).unwrap();
        assert_eq!(results.len(), 1);
        assert!(!results.iter().any(|n| n.id == "s-node-deleted"));

        // search_by_keywords 成功路径：位于逻辑删除画布中的节点不出现在结果中。
        let canvas3 = crate::business::user_database::entity::Canvas {
            id: "s-canvas-3".to_string(),
            parent_id: None,
            name: "Gamma Canvas".to_string(),
            x: 0.0,
            y: 0.0,
            deleted: true,
            color: String::new(),
        };
        crate::business::user_database::canvas::dao::insert(&connection, &canvas3).unwrap();
        let mut node_in_deleted_canvas = node("s-node-hidden", "s-canvas-3");
        node_in_deleted_canvas.title = "Rust Hidden".to_string();
        insert(&connection, &node_in_deleted_canvas).unwrap();
        let results = search_by_keywords(&connection, &["Rust".to_string()]).unwrap();
        assert_eq!(results.len(), 1);
        assert!(!results.iter().any(|n| n.id == "s-node-hidden"));

        // search_by_keywords 成功路径：关键词含 % 时按字面字符匹配（LIKE 转义）。
        let mut node_percent = node("s-node-percent", "s-canvas-1");
        node_percent.title = "100% Pure".to_string();
        insert(&connection, &node_percent).unwrap();
        let results = search_by_keywords(&connection, &["%".to_string()]).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "s-node-percent");

        // search_by_keywords 成功路径：关键词含 _ 时按字面字符匹配（LIKE 转义）。
        let mut node_underscore = node("s-node-underscore", "s-canvas-1");
        node_underscore.title = "foo_bar".to_string();
        insert(&connection, &node_underscore).unwrap();
        let results = search_by_keywords(&connection, &["_".to_string()]).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "s-node-underscore");

        // search_by_keywords 成功路径：关键词含 \ 时按字面字符匹配（LIKE 转义）。
        let mut node_backslash = node("s-node-backslash", "s-canvas-1");
        node_backslash.title = r"foo\bar".to_string();
        insert(&connection, &node_backslash).unwrap();
        let results = search_by_keywords(&connection, &["\\".to_string()]).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "s-node-backslash");

        // search_by_keywords 成功路径：结果按画布名称、节点标题排序。
        let results = search_by_keywords(&connection, &["language".to_string()]).unwrap();
        assert_eq!(results.len(), 3);
        // s-canvas-1 (Alpha) 在前，s-canvas-2 (Beta) 在后；同画布内按 title 排序
        assert_eq!(results[0].id, "s-node-2"); // Alpha / "Python Script"
        assert_eq!(results[1].id, "s-node-1"); // Alpha / "Rust Programming"
        assert_eq!(results[2].id, "s-node-3"); // Beta / "JavaScript Web"

        // ===== select_colored 成功路径 =====
        // 插入带色节点、无色节点和已删除带色节点，验证只返回带色未删除节点。
        let mut colored = node("s-node-colored", "s-canvas-1");
        colored.title = "Colored Node".to_string();
        colored.color = "{\"fill\":\"#ff0000\"}".to_string();
        insert(&connection, &colored).unwrap();
        let mut deleted_colored = node("s-node-deleted-colored", "s-canvas-1");
        deleted_colored.title = "Deleted Colored".to_string();
        deleted_colored.color = "{\"fill\":\"#00ff00\"}".to_string();
        deleted_colored.deleted = true;
        insert(&connection, &deleted_colored).unwrap();
        // s-node-1 为未删除无色节点，不应出现在结果中。
        let colored_entries = select_colored(&connection).unwrap();
        assert_eq!(colored_entries.len(), 1);
        assert_eq!(colored_entries[0].title, "Colored Node");
        assert_eq!(colored_entries[0].color, "{\"fill\":\"#ff0000\"}");
    }
}
