use rusqlite::Connection;

use crate::business::user_database::node::dao::NodeIden;
use crate::error_code::ErrorCode;
use crate::util::sea_query_util::values_to_params;
use sea_query::{
    Asterisk, ColumnDef, ColumnType, Expr, ExprTrait, ForeignKey, ForeignKeyAction, Func, Index,
    JoinType, Order, Query, SqliteQueryBuilder, Table,
};

/// node_tag 表的标识符集合，作为 sea-query 构建语句时使用的受控术语表。
#[derive(sea_query::Iden)]
pub(crate) enum NodeTagIden {
    #[iden = "node_tag"]
    Table,
    NodeId,
    Tag,
}

/// 新建 node_tag 表（节点与标签的多对多关联表）。
///
/// # 参数
/// - `connection`: 数据库连接。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn create_table(connection: &Connection) -> Result<(), ErrorCode> {
    let mut fk_node = ForeignKey::create();
    fk_node
        .from(NodeTagIden::Table, NodeTagIden::NodeId)
        .to(NodeIden::Table, NodeIden::Id)
        .on_delete(ForeignKeyAction::Cascade);
    let mut primary_key = Index::create();
    primary_key
        .col(NodeTagIden::NodeId)
        .col(NodeTagIden::Tag);
    let table = Table::create()
        .table(NodeTagIden::Table)
        .col(ColumnDef::new_with_type(NodeTagIden::NodeId, ColumnType::custom("TEXT")).not_null())
        .col(ColumnDef::new_with_type(NodeTagIden::Tag, ColumnType::custom("TEXT")).not_null())
        .primary_key(&mut primary_key)
        .foreign_key(&mut fk_node)
        .check(Expr::col(NodeTagIden::Tag).ne(""))
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

/// 向 node_tag 表插入一条节点标签关联记录。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `node_id`: 节点 id。
/// - `tag`: 标签名称（调用方保证非空）。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn insert(connection: &Connection, node_id: &str, tag: &str) -> Result<(), ErrorCode> {
    let query = Query::insert()
        .into_table(NodeTagIden::Table)
        .columns([NodeTagIden::NodeId, NodeTagIden::Tag])
        .values_panic([node_id.into(), tag.into()])
        .take();
    let (sql, values) = query.build(SqliteQueryBuilder);
    connection
        .execute(&sql, rusqlite::params_from_iter(values_to_params(values)))
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    Ok(())
}

/// 删除指定节点的全部标签关联记录。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `node_id`: 节点 id。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn delete_by_node_id(connection: &Connection, node_id: &str) -> Result<(), ErrorCode> {
    let query = Query::delete()
        .from_table(NodeTagIden::Table)
        .and_where(Expr::col(NodeTagIden::NodeId).eq(node_id))
        .take();
    let (sql, values) = query.build(SqliteQueryBuilder);
    connection
        .execute(&sql, rusqlite::params_from_iter(values_to_params(values)))
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    Ok(())
}

/// 查询指定节点的全部标签，按标签名称升序。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `node_id`: 节点 id。
///
/// # 返回值
/// 返回该节点的标签名称列表；若发生错误则返回对应的 `ErrorCode`。
pub fn select_tags_by_node_id(
    connection: &Connection,
    node_id: &str,
) -> Result<Vec<String>, ErrorCode> {
    let query = Query::select()
        .column(NodeTagIden::Tag)
        .from(NodeTagIden::Table)
        .and_where(Expr::col(NodeTagIden::NodeId).eq(node_id))
        .order_by(NodeTagIden::Tag, Order::Asc)
        .take();
    let (sql, values) = query.build(SqliteQueryBuilder);
    let mut statement = connection
        .prepare(&sql)
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    let rows = statement
        .query_map(rusqlite::params_from_iter(values_to_params(values)), |row| {
            row.get(0)
        })
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    rows.collect::<Result<Vec<String>, _>>()
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })
}

/// 查询全部标签及其关联的未删除数据节点数量，按数量降序、标签名称升序。
///
/// 只统计未逻辑删除、非影子节点（INNER JOIN node 过滤 node.deleted = 0
/// 且 node.shadow_producing_edge_id IS NULL）。
///
/// # 参数
/// - `connection`: 数据库连接。
///
/// # 返回值
/// 返回 (标签名称, 节点数量) 列表；若发生错误则返回对应的 `ErrorCode`。
pub fn select_all_with_count(connection: &Connection) -> Result<Vec<(String, i64)>, ErrorCode> {
    let query = Query::select()
        .column((NodeTagIden::Table, NodeTagIden::Tag))
        .expr(Func::count(Expr::col(Asterisk)))
        .from(NodeTagIden::Table)
        .join(
            JoinType::Join,
            NodeIden::Table,
            Expr::col((NodeIden::Table, NodeIden::Id))
                .equals((NodeTagIden::Table, NodeTagIden::NodeId)),
        )
        .and_where(Expr::col((NodeIden::Table, NodeIden::Deleted)).eq(0))
        .and_where(Expr::col((NodeIden::Table, NodeIden::ShadowProducingEdgeId)).is_null())
        .group_by_col((NodeTagIden::Table, NodeTagIden::Tag))
        .order_by_expr(Func::count(Expr::col(Asterisk)).into(), Order::Desc)
        .order_by((NodeTagIden::Table, NodeTagIden::Tag), Order::Asc)
        .take();
    let (sql, values) = query.build(SqliteQueryBuilder);
    let mut statement = connection
        .prepare(&sql)
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    let rows = statement
        .query_map(rusqlite::params_from_iter(values_to_params(values)), |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    rows.collect::<Result<Vec<(String, i64)>, _>>()
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::business::user_database::canvas::dao as canvas_dao;
    use crate::business::user_database::edge::dao as edge_dao;
    use crate::business::user_database::entity::{Canvas, Node};
    use crate::business::user_database::node::dao as node_dao;

    /// 构造测试用 Node，各字段可由调用方再修改。
    fn node(id: &str, canvas_id: &str) -> Node {
        Node {
            id: id.to_string(),
            canvas_id: canvas_id.to_string(),
            x: 0.0,
            y: 0.0,
            title: format!("title-{id}"),
            subtitle: format!("sub-title-{id}"),
            canvas_ref_id: None,
            deleted: false,
            color: String::new(),
            shadow_producing_edge_id: None,
            bookmarked: false,
        }
    }

    /// 覆盖 node_tag dao 模块所有 dao 函数的成功与失败路径。
    #[test]
    fn test_node_tag_dao_all_functions() {
        let connection = Connection::open_in_memory().unwrap();
        // dao 单表测试聚焦本表 SQL，关闭外键以隔离父表依赖；外键级联行为由末尾的独立测试覆盖。
        connection
            .execute_batch("PRAGMA foreign_keys = OFF;")
            .unwrap();

        // insert 失败路径：表不存在时报 DatabaseError。
        assert!(matches!(
            insert(&connection, "n1", "tag-a"),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // delete_by_node_id 失败路径：表不存在时报 DatabaseError。
        assert!(matches!(
            delete_by_node_id(&connection, "n1"),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // select_tags_by_node_id 失败路径：表不存在时报 DatabaseError。
        assert!(matches!(
            select_tags_by_node_id(&connection, "n1"),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // select_all_with_count 失败路径：表不存在时报 DatabaseError。
        assert!(matches!(
            select_all_with_count(&connection),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // create_table 成功路径。
        create_table(&connection).unwrap();

        // create_table 失败路径：重复建表报 DatabaseError。
        assert!(matches!(
            create_table(&connection),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // insert 与 select_tags_by_node_id 成功路径：多标签按名称升序读出。
        insert(&connection, "n1", "beta").unwrap();
        insert(&connection, "n1", "alpha").unwrap();
        insert(&connection, "n2", "alpha").unwrap();
        assert_eq!(
            select_tags_by_node_id(&connection, "n1").unwrap(),
            vec!["alpha", "beta"]
        );
        // select_tags_by_node_id 成功路径：节点无标签时返回空列表。
        assert!(select_tags_by_node_id(&connection, "n-x").unwrap().is_empty());

        // insert 失败路径：同一 node_id + tag 重复插入时报 DatabaseError（联合主键约束）。
        assert!(matches!(
            insert(&connection, "n1", "alpha"),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // 不同节点使用相同标签成功（多对多语义）。
        assert_eq!(
            select_tags_by_node_id(&connection, "n2").unwrap(),
            vec!["alpha"]
        );

        // CHECK 约束失败路径：空标签被数据库拒绝。
        assert!(matches!(
            insert(&connection, "n3", ""),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // delete_by_node_id 成功路径：只删除指定节点的标签，其它节点不受影响。
        delete_by_node_id(&connection, "n1").unwrap();
        assert!(select_tags_by_node_id(&connection, "n1").unwrap().is_empty());
        assert_eq!(
            select_tags_by_node_id(&connection, "n2").unwrap(),
            vec!["alpha"]
        );

        // select_all_with_count 成功路径：建 canvas / node 表并提供节点行，
        // 计数按数量降序、标签名称升序，且排除已删除节点与影子节点。
        canvas_dao::create_table(&connection).unwrap();
        node_dao::create_table(&connection).unwrap();
        let canvas = Canvas {
            id: "canvas-1".to_string(),
            parent_id: None,
            name: "tag-canvas".to_string(),
            x: 0.0,
            y: 0.0,
            deleted: false,
            color: String::new(),
        };
        canvas_dao::insert(&connection, &canvas).unwrap();
        node_dao::insert(&connection, &node("n1", "canvas-1")).unwrap();
        node_dao::insert(&connection, &node("n2", "canvas-1")).unwrap();
        let mut deleted_node = node("n3", "canvas-1");
        deleted_node.deleted = true;
        node_dao::insert(&connection, &deleted_node).unwrap();
        let mut shadow_node = node("n4", "canvas-1");
        shadow_node.shadow_producing_edge_id = Some("edge-1".to_string());
        node_dao::insert(&connection, &shadow_node).unwrap();

        // 清空 n2 遗留的 alpha 标签行，避免后续重复插入触发联合主键冲突。
        delete_by_node_id(&connection, "n2").unwrap();

        // n1: alpha, beta；n2: alpha, gamma；n3（已删除）: alpha；n4（影子）: alpha。
        insert(&connection, "n1", "alpha").unwrap();
        insert(&connection, "n1", "beta").unwrap();
        insert(&connection, "n2", "alpha").unwrap();
        insert(&connection, "n2", "gamma").unwrap();
        insert(&connection, "n3", "alpha").unwrap();
        insert(&connection, "n4", "alpha").unwrap();

        let counts = select_all_with_count(&connection).unwrap();
        // alpha 计数 2（n3 已删除、n4 影子被排除），beta / gamma 各 1；
        // 数量降序，同数量按标签名称升序：alpha, beta, gamma。
        assert_eq!(
            counts,
            vec![
                ("alpha".to_string(), 2),
                ("beta".to_string(), 1),
                ("gamma".to_string(), 1),
            ]
        );

        // select_all_with_count 成功路径：所有节点都被逻辑删除时返回空列表。
        let mut n1 = node_dao::select_by_id(&connection, "n1").unwrap().unwrap();
        n1.deleted = true;
        node_dao::update(&connection, &n1).unwrap();
        let mut n2 = node_dao::select_by_id(&connection, "n2").unwrap().unwrap();
        n2.deleted = true;
        node_dao::update(&connection, &n2).unwrap();
        assert!(select_all_with_count(&connection).unwrap().is_empty());
    }

    /// STRICT 生效验证：向 TEXT 列（tag）插入 BLOB 值时被数据库拒绝（非 STRICT 表会静默接受）。
    #[test]
    fn test_node_tag_strict_type_enforced() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch("PRAGMA foreign_keys = OFF;")
            .unwrap();
        create_table(&connection).unwrap();
        assert!(matches!(
            connection.execute(
                "INSERT INTO node_tag (node_id, tag) VALUES ('n1', x'0102')",
                [],
            ),
            Err(_)
        ));
    }

    /// 级联删除：物理删除节点后，该节点的 node_tag 行经外键 ON DELETE CASCADE 一并消失。
    #[test]
    fn test_node_tag_cascade_delete() {
        let connection = Connection::open_in_memory().unwrap();
        // 先建表并插入数据（外键关闭，便于按需构造），再开启外键验证级联行为。
        connection
            .execute_batch("PRAGMA foreign_keys = OFF;")
            .unwrap();
        canvas_dao::create_table(&connection).unwrap();
        // node 表有指向 edge 表的外键，开启外键后 SQLite 要求父表存在。
        edge_dao::create_table(&connection).unwrap();
        node_dao::create_table(&connection).unwrap();
        create_table(&connection).unwrap();
        let canvas = Canvas {
            id: "cascade-canvas".to_string(),
            parent_id: None,
            name: "cascade-canvas".to_string(),
            x: 0.0,
            y: 0.0,
            deleted: false,
            color: String::new(),
        };
        canvas_dao::insert(&connection, &canvas).unwrap();
        node_dao::insert(&connection, &node("cascade-n1", "cascade-canvas")).unwrap();
        node_dao::insert(&connection, &node("cascade-n2", "cascade-canvas")).unwrap();
        insert(&connection, "cascade-n1", "tag-a").unwrap();
        insert(&connection, "cascade-n1", "tag-b").unwrap();
        insert(&connection, "cascade-n2", "tag-a").unwrap();

        connection
            .execute_batch("PRAGMA foreign_keys = ON;")
            .unwrap();

        // 物理删除 cascade-n1：其两条标签行级联消失，cascade-n2 的标签行保留。
        node_dao::delete_by_id(&connection, "cascade-n1").unwrap();
        assert!(select_tags_by_node_id(&connection, "cascade-n1")
            .unwrap()
            .is_empty());
        assert_eq!(
            select_tags_by_node_id(&connection, "cascade-n2").unwrap(),
            vec!["tag-a"]
        );
        // 表内不再存在指向已删除节点的标签行。
        let counts = select_all_with_count(&connection).unwrap();
        assert_eq!(counts, vec![("tag-a".to_string(), 1)]);
    }
}

