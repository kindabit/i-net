use rusqlite::{Connection, OptionalExtension, Row};

use crate::business::user_database::entity::Attachment;
use crate::business::user_database::node::dao::NodeIden;
use crate::error_code::ErrorCode;
use crate::util::sea_query_util::values_to_params;
use sea_query::{
    ColumnDef, ColumnType, Expr, ExprTrait, ForeignKey, ForeignKeyAction, Func, Index, Order, Query,
    SqliteQueryBuilder, Table,
};

/// attachment 表的标识符集合，作为 sea-query 构建语句时使用的受控术语表。
#[derive(sea_query::Iden)]
enum AttachmentIden {
    #[iden = "attachment"]
    Table,
    Id,
    NodeId,
    FileName,
    Size,
    CreateTime,
    Deleted,
    SortOrder,
    Compressed,
    CompressParam,
}

/// 从查询结果行构造 Attachment。
fn map_row(row: &Row) -> rusqlite::Result<Attachment> {
    Ok(Attachment {
        id: row.get(0)?,
        node_id: row.get(1)?,
        file_name: row.get(2)?,
        size: row.get(3)?,
        create_time: row.get(4)?,
        deleted: row.get::<_, i64>(5)? != 0,
        sort_order: row.get(6)?,
        compressed: row.get::<_, i64>(7)? != 0,
        compress_param: row.get(8)?,
    })
}

/// 新建 attachment 表。
///
/// # 参数
/// - `connection`: 数据库连接。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn create_table(connection: &Connection) -> Result<(), ErrorCode> {
    let mut fk_node = ForeignKey::create();
    fk_node
        .from(AttachmentIden::Table, AttachmentIden::NodeId)
        .to(NodeIden::Table, NodeIden::Id)
        .on_delete(ForeignKeyAction::Cascade);
    let mut node_sort_unique = Index::create();
    node_sort_unique
        .unique()
        .col(AttachmentIden::NodeId)
        .col(AttachmentIden::SortOrder);
    let table = Table::create()
        .table(AttachmentIden::Table)
        .col(
            ColumnDef::new_with_type(AttachmentIden::Id, ColumnType::custom("TEXT"))
                .primary_key()
                .not_null(),
        )
        .col(
            ColumnDef::new_with_type(AttachmentIden::NodeId, ColumnType::custom("TEXT")).not_null(),
        )
        .col(
            ColumnDef::new_with_type(AttachmentIden::FileName, ColumnType::custom("TEXT")).not_null(),
        )
        .col(ColumnDef::new_with_type(AttachmentIden::Size, ColumnType::custom("INTEGER")).not_null())
        .col(
            ColumnDef::new_with_type(AttachmentIden::CreateTime, ColumnType::custom("INTEGER"))
                .not_null(),
        )
        .col(
            ColumnDef::new_with_type(AttachmentIden::Deleted, ColumnType::custom("INTEGER")).not_null(),
        )
        .col(
            ColumnDef::new_with_type(AttachmentIden::SortOrder, ColumnType::custom("INTEGER"))
                .not_null(),
        )
        .col(
            ColumnDef::new_with_type(AttachmentIden::Compressed, ColumnType::custom("INTEGER"))
                .not_null(),
        )
        .col(
            ColumnDef::new_with_type(AttachmentIden::CompressParam, ColumnType::custom("TEXT"))
                .not_null(),
        )
        .foreign_key(&mut fk_node)
        .index(&mut node_sort_unique)
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

/// 向 attachment 表插入一条附件记录。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `attachment`: 要插入的附件。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn insert(connection: &Connection, attachment: &Attachment) -> Result<(), ErrorCode> {
    let query = Query::insert()
        .into_table(AttachmentIden::Table)
        .columns([
            AttachmentIden::Id,
            AttachmentIden::NodeId,
            AttachmentIden::FileName,
            AttachmentIden::Size,
            AttachmentIden::CreateTime,
            AttachmentIden::Deleted,
            AttachmentIden::SortOrder,
            AttachmentIden::Compressed,
            AttachmentIden::CompressParam,
        ])
        .values_panic([
            (&attachment.id).into(),
            (&attachment.node_id).into(),
            (&attachment.file_name).into(),
            attachment.size.into(),
            attachment.create_time.into(),
            (attachment.deleted as i64).into(),
            attachment.sort_order.into(),
            (attachment.compressed as i64).into(),
            (&attachment.compress_param).into(),
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

/// 按 id 查询附件。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `id`: 附件 id。
///
/// # 返回值
/// 返回查询到的附件，不存在时返回 `None`；若发生错误则返回对应的 `ErrorCode`。
pub fn select_by_id(connection: &Connection, id: &str) -> Result<Option<Attachment>, ErrorCode> {
    let query = Query::select()
        .columns([
            AttachmentIden::Id,
            AttachmentIden::NodeId,
            AttachmentIden::FileName,
            AttachmentIden::Size,
            AttachmentIden::CreateTime,
            AttachmentIden::Deleted,
            AttachmentIden::SortOrder,
            AttachmentIden::Compressed,
            AttachmentIden::CompressParam,
        ])
        .from(AttachmentIden::Table)
        .and_where(Expr::col(AttachmentIden::Id).eq(id))
        .take();
    let (sql, values) = query.build(SqliteQueryBuilder);
    connection
        .query_row(
            &sql,
            rusqlite::params_from_iter(values_to_params(values)),
            map_row,
        )
        .optional()
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })
}

/// 按节点 id 和逻辑删除标志查询附件，按 sort_order 升序。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `node_id`: 节点 id。
/// - `deleted`: 逻辑删除标志。
///
/// # 返回值
/// 返回查询到的附件列表；若发生错误则返回对应的 `ErrorCode`。
pub fn select_by_node_id(
    connection: &Connection,
    node_id: &str,
    deleted: bool,
) -> Result<Vec<Attachment>, ErrorCode> {
    let query = Query::select()
        .columns([
            AttachmentIden::Id,
            AttachmentIden::NodeId,
            AttachmentIden::FileName,
            AttachmentIden::Size,
            AttachmentIden::CreateTime,
            AttachmentIden::Deleted,
            AttachmentIden::SortOrder,
            AttachmentIden::Compressed,
            AttachmentIden::CompressParam,
        ])
        .from(AttachmentIden::Table)
        .and_where(Expr::col(AttachmentIden::NodeId).eq(node_id))
        .and_where(Expr::col(AttachmentIden::Deleted).eq(deleted as i64))
        .order_by(AttachmentIden::SortOrder, Order::Asc)
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

/// 按多个节点 id 查询附件，不分逻辑删除标志全量返回，按 sort_order 升序。
/// 供节点/画布物理删除的级联编排使用。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `node_ids`: 节点 id 列表；为空时不查询直接返回空列表。
///
/// # 返回值
/// 返回查询到的附件列表；若发生错误则返回对应的 `ErrorCode`。
pub fn select_by_node_ids(
    connection: &Connection,
    node_ids: &[String],
) -> Result<Vec<Attachment>, ErrorCode> {
    if node_ids.is_empty() {
        return Ok(Vec::new());
    }
    let query = Query::select()
        .columns([
            AttachmentIden::Id,
            AttachmentIden::NodeId,
            AttachmentIden::FileName,
            AttachmentIden::Size,
            AttachmentIden::CreateTime,
            AttachmentIden::Deleted,
            AttachmentIden::SortOrder,
            AttachmentIden::Compressed,
            AttachmentIden::CompressParam,
        ])
        .from(AttachmentIden::Table)
        .and_where(Expr::col(AttachmentIden::NodeId).is_in(node_ids.iter()))
        .order_by(AttachmentIden::SortOrder, Order::Asc)
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

/// 查询全部附件 id，供孤儿文件检测（文件存在但元数据不存在）使用。
///
/// # 参数
/// - `connection`: 数据库连接。
///
/// # 返回值
/// 返回全部附件 id 的列表；若发生错误则返回对应的 `ErrorCode`。
pub fn select_all_ids(connection: &Connection) -> Result<Vec<String>, ErrorCode> {
    let query = Query::select()
        .columns([AttachmentIden::Id])
        .from(AttachmentIden::Table)
        .take();
    let (sql, values) = query.build(SqliteQueryBuilder);
    let mut statement = connection
        .prepare(&sql)
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    let rows = statement
        .query_map(rusqlite::params_from_iter(values_to_params(values)), |row| row.get(0))
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    rows.collect::<Result<Vec<String>, _>>()
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })
}

/// 更新指定附件的逻辑删除标志。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `id`: 附件 id。
/// - `deleted`: 逻辑删除标志。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn update_deleted(connection: &Connection, id: &str, deleted: bool) -> Result<(), ErrorCode> {
    let query = Query::update()
        .table(AttachmentIden::Table)
        .value(AttachmentIden::Deleted, deleted as i64)
        .and_where(Expr::col(AttachmentIden::Id).eq(id))
        .take();
    let (sql, values) = query.build(SqliteQueryBuilder);
    connection
        .execute(&sql, rusqlite::params_from_iter(values_to_params(values)))
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    Ok(())
}

/// 按 id 整体更新一条附件记录，所有字段均以传入的 Attachment 为准。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `attachment`: 更新后的附件，按其中的 id 定位要更新的行。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn update(connection: &Connection, attachment: &Attachment) -> Result<(), ErrorCode> {
    let query = Query::update()
        .table(AttachmentIden::Table)
        .values([
            (AttachmentIden::NodeId, attachment.node_id.clone().into()),
            (AttachmentIden::FileName, attachment.file_name.clone().into()),
            (AttachmentIden::Size, attachment.size.into()),
            (AttachmentIden::CreateTime, attachment.create_time.into()),
            (AttachmentIden::Deleted, (attachment.deleted as i64).into()),
            (AttachmentIden::SortOrder, attachment.sort_order.into()),
            (AttachmentIden::Compressed, (attachment.compressed as i64).into()),
            (AttachmentIden::CompressParam, attachment.compress_param.clone().into()),
        ])
        .and_where(Expr::col(AttachmentIden::Id).eq(attachment.id.clone()))
        .take();
    let (sql, values) = query.build(SqliteQueryBuilder);
    connection
        .execute(&sql, rusqlite::params_from_iter(values_to_params(values)))
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    Ok(())
}

/// 交换两个附件的 sort_order。
/// 通过 prepared statement 复用 SQL 编译结果，分三步使用临时值避免 UNIQUE 约束冲突。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `id1`: 附件1 id。
/// - `id2`: 附件2 id。
/// - `order1`: 附件1 的当前 sort_order。
/// - `order2`: 附件2 的当前 sort_order。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn swap_sort_order(
    connection: &Connection,
    id1: &str,
    id2: &str,
    order1: i64,
    order2: i64,
) -> Result<(), ErrorCode> {
    // 占位符顺序约定：build 产出的 SQL 中 ? 依次对应 value() 调用序（sort_order）与 WHERE 序（id），
    // 因此三步交换逐条按位绑定 params![sort_order, id]。
    let (sql, _) = Query::update()
        .table(AttachmentIden::Table)
        .value(AttachmentIden::SortOrder, 0i64)
        .and_where(Expr::col(AttachmentIden::Id).eq(""))
        .build(SqliteQueryBuilder);
    let mut stmt = connection
        .prepare(&sql)
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    // 三步交换：id1 临时值 → id2 写入 order1 → id1 写入 order2
    stmt.execute(rusqlite::params![-1_i64, id1])
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    stmt.execute(rusqlite::params![order1, id2])
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    stmt.execute(rusqlite::params![order2, id1])
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    Ok(())
}

/// 查询指定节点下附件的最大 sort_order，无附件时返回 0。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `node_id`: 节点 id。
///
/// # 返回值
/// 返回最大 sort_order；若发生错误则返回对应的 `ErrorCode`。
pub fn select_max_sort_order(connection: &Connection, node_id: &str) -> Result<i64, ErrorCode> {
    let query = Query::select()
        .expr(Func::coalesce([Expr::col(AttachmentIden::SortOrder).max(), Expr::val(0)]))
        .from(AttachmentIden::Table)
        .and_where(Expr::col(AttachmentIden::NodeId).eq(node_id))
        .take();
    let (sql, values) = query.build(SqliteQueryBuilder);
    let result: i64 = connection
        .query_row(
            &sql,
            rusqlite::params_from_iter(values_to_params(values)),
            |row| row.get::<_, i64>(0),
        )
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    Ok(result)
}

/// 按 id 删除一条附件记录。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `id`: 附件 id。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn delete_by_id(connection: &Connection, id: &str) -> Result<(), ErrorCode> {
    let query = Query::delete()
        .from_table(AttachmentIden::Table)
        .and_where(Expr::col(AttachmentIden::Id).eq(id))
        .take();
    let (sql, values) = query.build(SqliteQueryBuilder);
    connection
        .execute(&sql, rusqlite::params_from_iter(values_to_params(values)))
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造测试用 Attachment，deleted 固定为 false，sort_order 由调用方指定。
    fn attachment_with_sort_order(
        id: &str,
        node_id: &str,
        create_time: i64,
        sort_order: i64,
    ) -> Attachment {
        Attachment {
            id: id.to_string(),
            node_id: node_id.to_string(),
            file_name: format!("file-{id}.pdf"),
            size: 100,
            create_time,
            deleted: false,
            sort_order,
            compressed: false,
            compress_param: String::new(),
        }
    }

    /// 覆盖 attachment dao 模块所有 dao 函数的成功与失败路径。
    #[test]
    fn test_attachment_dao_all_functions() {
        let connection = Connection::open_in_memory().unwrap();
        // dao 单表测试聚焦本表 SQL，关闭外键以隔离父表依赖；外键级联行为由 service 测试端到端覆盖。
        connection
            .execute_batch("PRAGMA foreign_keys = OFF;")
            .unwrap();

        // insert 失败路径：表不存在时报 DatabaseError。
        assert!(matches!(
            insert(&connection, &attachment_with_sort_order("a1", "n1", 1, 0)),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // select_by_id 失败路径：表不存在时报 DatabaseError。
        assert!(matches!(
            select_by_id(&connection, "a1"),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // select_by_node_id 失败路径：表不存在时报 DatabaseError。
        assert!(matches!(
            select_by_node_id(&connection, "n1", false),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // select_by_node_ids 失败路径：表不存在时报 DatabaseError。
        assert!(matches!(
            select_by_node_ids(&connection, &["n1".to_string()]),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // select_all_ids 失败路径：表不存在时报 DatabaseError。
        assert!(matches!(
            select_all_ids(&connection),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // update_deleted 失败路径：表不存在时报 DatabaseError。
        assert!(matches!(
            update_deleted(&connection, "a1", true),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // update 失败路径：表不存在时报 DatabaseError。
        assert!(matches!(
            update(&connection, &attachment_with_sort_order("a1", "n1", 1, 0)),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // swap_sort_order 失败路径：表不存在时报 DatabaseError。
        assert!(matches!(
            swap_sort_order(&connection, "a1", "a2", 0, 10),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // delete_by_id 失败路径：表不存在时报 DatabaseError。
        assert!(matches!(
            delete_by_id(&connection, "a1"),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // create_table 成功路径。
        create_table(&connection).unwrap();

        // create_table 失败路径：重复建表报 DatabaseError。
        assert!(matches!(
            create_table(&connection),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // insert 成功路径：插入后 select_by_id 能查到，各字段往返一致。
        insert(&connection, &attachment_with_sort_order("a1", "n1", 3, 0)).unwrap();
        let selected = select_by_id(&connection, "a1").unwrap().unwrap();
        assert_eq!(selected.node_id, "n1");
        assert_eq!(selected.file_name, "file-a1.pdf");
        assert_eq!(selected.size, 100);
        assert_eq!(selected.create_time, 3);
        assert!(!selected.deleted);
        assert_eq!(selected.sort_order, 0);
        // compressed / compress_param 往返：默认未压缩，compress_param 为空。
        assert!(!selected.compressed);
        assert_eq!(selected.compress_param, "");

        // select_by_id 成功路径：不存在时返回 None。
        assert!(select_by_id(&connection, "a-x").unwrap().is_none());

        // insert 失败路径：id 重复时报 DatabaseError（主键约束）。
        assert!(matches!(
            insert(&connection, &attachment_with_sort_order("a1", "n2", 4, 0)),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // select_by_node_id 成功路径：按 deleted 分流并按 sort_order 升序。
        insert(&connection, &attachment_with_sort_order("a2", "n1", 1, 1)).unwrap();
        let mut deleted_attachment = attachment_with_sort_order("a3", "n1", 2, 2);
        deleted_attachment.deleted = true;
        insert(&connection, &deleted_attachment).unwrap();
        insert(&connection, &attachment_with_sort_order("a4", "n2", 4, 3)).unwrap();
        let normal = select_by_node_id(&connection, "n1", false).unwrap();
        assert_eq!(normal.len(), 2);
        assert_eq!(normal[0].id, "a1");
        assert_eq!(normal[1].id, "a2");
        let deleted = select_by_node_id(&connection, "n1", true).unwrap();
        assert_eq!(deleted.len(), 1);
        assert_eq!(deleted[0].id, "a3");
        assert!(
            select_by_node_id(&connection, "n-x", false)
                .unwrap()
                .is_empty()
        );

        // select_by_node_ids 成功路径：不分 deleted 全量返回多个节点的附件，按 sort_order 升序。
        let multi =
            select_by_node_ids(&connection, &["n1".to_string(), "n2".to_string()]).unwrap();
        assert_eq!(multi.len(), 4);
        // a1 (n1, sort_order=0), a2 (n1, sort_order=1), a3 (n1, sort_order=2), a4 (n2, sort_order=3)
        assert_eq!(multi[0].id, "a1");
        assert_eq!(multi[1].id, "a2");
        assert_eq!(multi[2].id, "a3");
        assert_eq!(multi[3].id, "a4");

        // select_by_node_ids 成功路径：空输入不触碰表直接返回空列表。
        assert!(select_by_node_ids(&connection, &[]).unwrap().is_empty());

        // select_all_ids 成功路径：返回全部附件 id（顺序无关）。
        let mut ids = select_all_ids(&connection).unwrap();
        ids.sort();
        assert_eq!(ids, vec!["a1", "a2", "a3", "a4"]);

        // update_deleted 成功路径：标志翻转后 select_by_node_id 按新标志分流。
        update_deleted(&connection, "a1", true).unwrap();
        assert_eq!(
            select_by_node_id(&connection, "n1", true).unwrap().len(),
            2
        );
        assert_eq!(
            select_by_node_id(&connection, "n1", false).unwrap().len(),
            1
        );

        // update 成功路径：整体更新该行各字段，其它记录不受影响。
        insert(&connection, &attachment_with_sort_order("a5", "n3", 6, 0)).unwrap();
        let mut updated = attachment_with_sort_order("a5", "n3", 6, 5);
        updated.file_name = "updated.pdf".to_string();
        updated.size = 200;
        updated.create_time = 7;
        updated.deleted = true;
        // compressed / compress_param 往返：更新为已压缩状态并携带压缩参数。
        updated.compressed = true;
        updated.compress_param = "{\"algorithm\":\"zstd\",\"level\":19}".to_string();
        update(&connection, &updated).unwrap();
        let selected = select_by_id(&connection, "a5").unwrap().unwrap();
        assert_eq!(selected.file_name, "updated.pdf");
        assert_eq!(selected.size, 200);
        assert_eq!(selected.create_time, 7);
        assert!(selected.deleted);
        assert_eq!(selected.sort_order, 5);
        assert!(selected.compressed);
        assert_eq!(selected.compress_param, "{\"algorithm\":\"zstd\",\"level\":19}");
        // 其它记录不受影响。
        assert_eq!(select_by_id(&connection, "a1").unwrap().unwrap().size, 100);

        // delete_by_id 成功路径：删除后查不到该记录，其它记录不受影响。
        delete_by_id(&connection, "a3").unwrap();
        assert!(select_by_id(&connection, "a3").unwrap().is_none());
        assert!(select_by_id(&connection, "a1").unwrap().is_some());


        // 通过直接 SQL 修改 a2 的 sort_order 为 10，为后续交换测试做准备。
        connection
            .execute(
                "UPDATE attachment SET sort_order = ? WHERE id = ?",
                rusqlite::params![10_i64, "a2"],
            )
            .unwrap();
        assert_eq!(
            select_by_id(&connection, "a2").unwrap().unwrap().sort_order,
            10
        );

        // swap_sort_order 成功路径：交换两个附件的 sort_order。
        // a1 sort_order = 0, a2 sort_order = 10
        swap_sort_order(&connection, "a1", "a2", 0, 10).unwrap();
        assert_eq!(select_by_id(&connection, "a1").unwrap().unwrap().sort_order, 10);
        assert_eq!(select_by_id(&connection, "a2").unwrap().unwrap().sort_order, 0);

        // swap_sort_order 成功路径：再次交换恢复原状。
        swap_sort_order(&connection, "a1", "a2", 10, 0).unwrap();
        assert_eq!(select_by_id(&connection, "a1").unwrap().unwrap().sort_order, 0);
        assert_eq!(select_by_id(&connection, "a2").unwrap().unwrap().sort_order, 10);

        // select_max_sort_order 成功路径：返回指定节点下最大的 sort_order。
        assert_eq!(select_max_sort_order(&connection, "n1").unwrap(), 10);
        assert_eq!(select_max_sort_order(&connection, "n2").unwrap(), 3);
        // select_max_sort_order 成功路径：节点无附件时返回 0。
        assert_eq!(select_max_sort_order(&connection, "n-x").unwrap(), 0);
    }

    /// STRICT 生效验证：向 TEXT 列（file_name）插入 BLOB 值时被数据库拒绝（非 STRICT 表会静默接受）。
    #[test]
    fn test_attachment_strict_type_enforced() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch("PRAGMA foreign_keys = OFF;")
            .unwrap();
        create_table(&connection).unwrap();
        assert!(matches!(
            connection.execute(
                "INSERT INTO attachment (id, node_id, file_name, size, create_time, deleted, sort_order, compressed, compress_param)
                VALUES ('strict-violation', 'node-1', x'0102', 100, 1, 0, 0, 0, '')",
                [],
            ),
            Err(_)
        ));
    }
}