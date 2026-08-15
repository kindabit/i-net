use rusqlite::{Connection, Row};

use crate::business::user_database::entity::NodeField;
use crate::error_code::ErrorCode;

/// 从查询结果行构造 NodeField。
fn map_row(row: &Row) -> rusqlite::Result<NodeField> {
    Ok(NodeField {
        node_id: row.get(0)?,
        name: row.get(1)?,
        field_type: row.get(2)?,
        type_config: row.get(3)?,
        field_value: row.get(4)?,
        order: row.get(5)?,
        dictionary_id: row.get(6)?,
    })
}

/// 新建 node_field 表。
///
/// # 参数
/// - `connection`: 数据库连接。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn create_table(connection: &Connection) -> Result<(), ErrorCode> {
    connection
        .execute(
            "CREATE TABLE node_field (
                node_id TEXT NOT NULL REFERENCES node(id) ON DELETE CASCADE,
                name TEXT NOT NULL,
                field_type TEXT NOT NULL,
                type_config TEXT,
                field_value BLOB,
                \"order\" INTEGER NOT NULL,
                dictionary_id TEXT,
                PRIMARY KEY (node_id, name)
            ) STRICT",
            [],
        )
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    Ok(())
}

/// 向 node_field 表插入一条字段记录。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `node_field`: 要插入的节点字段。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn insert(connection: &Connection, node_field: &NodeField) -> Result<(), ErrorCode> {
    connection
        .execute(
            "INSERT INTO node_field (node_id, name, field_type, type_config, field_value, \"order\", dictionary_id)
            VALUES (:node_id, :name, :field_type, :type_config, :field_value, :order, :dictionary_id)",
            rusqlite::named_params! {
                ":node_id": node_field.node_id,
                ":name": node_field.name,
                ":field_type": node_field.field_type,
                ":type_config": node_field.type_config,
                ":field_value": node_field.field_value,
                ":order": node_field.order,
                ":dictionary_id": node_field.dictionary_id,
            },
        )
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    Ok(())
}

/// 按节点 id 查询其全部字段，按 "order" 升序。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `node_id`: 节点 id。
///
/// # 返回值
/// 返回查询到的字段列表；若发生错误则返回对应的 `ErrorCode`。
pub fn select_by_node_id(
    connection: &Connection,
    node_id: &str,
) -> Result<Vec<NodeField>, ErrorCode> {
    let mut statement = connection
        .prepare(
            "SELECT node_id, name, field_type, type_config, field_value, \"order\", dictionary_id
            FROM node_field
            WHERE node_id = :node_id
            ORDER BY \"order\" ASC",
        )
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    let rows = statement
        .query_map(
            rusqlite::named_params! {":node_id": node_id},
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

/// 删除指定节点的全部字段。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `node_id`: 节点 id。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn delete_by_node_id(connection: &Connection, node_id: &str) -> Result<(), ErrorCode> {
    connection
        .execute(
            "DELETE FROM node_field WHERE node_id = :node_id",
            rusqlite::named_params! {":node_id": node_id},
        )
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    Ok(())
}

/// 将引用了已不存在字典条目的 node_field.dictionary_id 置空。
///
/// 该 SQL 依赖 dictionary 表已存在。
///
/// # 参数
/// - `connection`: 数据库连接。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn clear_dangling_dictionary_ids(connection: &Connection) -> Result<(), ErrorCode> {
    connection
        .execute(
            "UPDATE node_field SET dictionary_id = NULL
            WHERE dictionary_id IS NOT NULL
            AND dictionary_id NOT IN (SELECT id FROM dictionary)",
            [],
        )
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造测试用 NodeField。
    fn nf(node_id: &str, name: &str, order: i64) -> NodeField {
        NodeField {
            node_id: node_id.to_string(),
            name: name.to_string(),
            field_type: "text".to_string(),
            type_config: None,
            field_value: None,
            order,
            dictionary_id: None,
        }
    }

    /// 覆盖 node_field dao 模块所有 dao 函数的成功与失败路径。
    #[test]
    fn test_node_field_dao_all_functions() {
        let connection = Connection::open_in_memory().unwrap();
        // dao 单表测试聚焦本表 SQL，关闭外键以隔离父表依赖；外键级联行为由 service 测试端到端覆盖。
        connection
            .execute_batch("PRAGMA foreign_keys = OFF;")
            .unwrap();

        // insert 失败路径：表不存在时报 DatabaseError。
        assert!(matches!(
            insert(&connection, &nf("n1", "f1", 1)),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // select_by_node_id 失败路径：表不存在时报 DatabaseError。
        assert!(matches!(
            select_by_node_id(&connection, "n1"),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // delete_by_node_id 失败路径：表不存在时报 DatabaseError。
        assert!(matches!(
            delete_by_node_id(&connection, "n1"),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // clear_dangling_dictionary_ids 失败路径：表不存在时报 DatabaseError。
        assert!(matches!(
            clear_dangling_dictionary_ids(&connection),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // create_table 成功路径。
        create_table(&connection).unwrap();

        // create_table 失败路径：重复建表报 DatabaseError。
        assert!(matches!(
            create_table(&connection),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // insert 成功路径：插入后 select_by_node_id 按 order 升序取回。
        insert(&connection, &nf("n1", "f3", 3)).unwrap();
        insert(&connection, &nf("n1", "f1", 1)).unwrap();
        insert(&connection, &nf("n1", "f2", 2)).unwrap();
        let fields = select_by_node_id(&connection, "n1").unwrap();
        assert_eq!(fields.len(), 3);
        assert_eq!(fields[0].name, "f1");
        assert_eq!(fields[1].name, "f2");
        assert_eq!(fields[2].name, "f3");

        // type_config / field_value / dictionary_id 为 Some 和 None 的往返一致。
        let mut rich = nf("n2", "rich", 1);
        rich.type_config = Some(r#"{"precision":"day"}"#.to_string());
        rich.field_value = Some(vec![0x01, 0x02, 0x03]);
        rich.dictionary_id = Some("dict-1".to_string());
        insert(&connection, &rich).unwrap();
        let selected = select_by_node_id(&connection, "n2").unwrap();
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].type_config.as_deref(), Some(r#"{"precision":"day"}"#));
        assert_eq!(selected[0].field_value.as_deref(), Some(&vec![0x01, 0x02, 0x03][..]));
        assert_eq!(selected[0].dictionary_id.as_deref(), Some("dict-1"));
        assert!(selected[0].order == 1);

        // insert 失败路径：联合主键 (node_id, name) 重复报 DatabaseError。
        assert!(matches!(
            insert(&connection, &nf("n1", "f1", 99)),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // 同 name 不同 node_id 可共存（成功路径）。
        let other_node = nf("n3", "f1", 1);
        insert(&connection, &other_node).unwrap();
        assert_eq!(select_by_node_id(&connection, "n3").unwrap().len(), 1);

        // delete_by_node_id 成功路径：只删目标节点字段，其它节点不受影响。
        delete_by_node_id(&connection, "n1").unwrap();
        assert!(select_by_node_id(&connection, "n1").unwrap().is_empty());
        assert_eq!(select_by_node_id(&connection, "n2").unwrap().len(), 1);
        assert_eq!(select_by_node_id(&connection, "n3").unwrap().len(), 1);

        // clear_dangling_dictionary_ids 成功路径。
        // 先建字典表并插入 dict-1。
        crate::business::user_database::dictionary::dao::create_table(&connection).unwrap();
        let dict_entry = crate::business::user_database::entity::Dictionary {
            id: "dict-1".to_string(),
            parent_id: None,
            value: "val-dict-1".to_string(),
            order: 1,
        };
        crate::business::user_database::dictionary::dao::batch_insert(&connection, &[dict_entry])
            .unwrap();

        // 插入两条 node_field：一条引用存在的 "dict-1"，一条引用不存在的 "dict-x"。
        let mut with_existing = nf("n4", "f-exist", 1);
        with_existing.dictionary_id = Some("dict-1".to_string());
        insert(&connection, &with_existing).unwrap();
        let mut with_dangling = nf("n4", "f-dangling", 2);
        with_dangling.dictionary_id = Some("dict-x".to_string());
        insert(&connection, &with_dangling).unwrap();
        // 插入一条 dictionary_id 为 None 的字段作为对照组。
        insert(&connection, &nf("n4", "f-none", 3)).unwrap();

        // 执行清理。
        clear_dangling_dictionary_ids(&connection).unwrap();

        let after = select_by_node_id(&connection, "n4").unwrap();
        let exist_field = after.iter().find(|f| f.name == "f-exist").unwrap();
        assert_eq!(exist_field.dictionary_id.as_deref(), Some("dict-1"));
        let dangling_field = after.iter().find(|f| f.name == "f-dangling").unwrap();
        assert!(dangling_field.dictionary_id.is_none());
        let none_field = after.iter().find(|f| f.name == "f-none").unwrap();
        assert!(none_field.dictionary_id.is_none());
    }
}
