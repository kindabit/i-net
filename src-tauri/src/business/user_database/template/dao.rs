use rusqlite::{Connection, OptionalExtension, Row};

use crate::business::user_database::dictionary::dao::DictionaryIden;
use crate::business::user_database::entity::{Template, TemplateField};
use crate::error_code::ErrorCode;
use crate::util::sea_query_util::values_to_params;
use sea_query::{
    ColumnDef, ColumnType, Expr, ExprTrait, ForeignKey, ForeignKeyAction, Func, Index, Order, Query,
    SqliteQueryBuilder, Table,
};

/// template 表的标识符集合，作为 sea-query 构建语句时使用的受控术语表。
#[derive(sea_query::Iden)]
enum TemplateIden {
    #[iden = "template"]
    Table,
    Id,
    Name,
    Order,
}

/// template_field 表的标识符集合，作为 sea-query 构建语句时使用的受控术语表。
#[derive(sea_query::Iden)]
enum TemplateFieldIden {
    #[iden = "template_field"]
    Table,
    TemplateId,
    Name,
    FieldType,
    Order,
    DictionaryId,
}

/// 从查询结果行构造 Template。
fn map_template_row(row: &Row) -> rusqlite::Result<Template> {
    Ok(Template {
        id: row.get(0)?,
        name: row.get(1)?,
        order: row.get(2)?,
    })
}

/// 从查询结果行构造 TemplateField。
fn map_field_row(row: &Row) -> rusqlite::Result<TemplateField> {
    Ok(TemplateField {
        template_id: row.get(0)?,
        name: row.get(1)?,
        field_type: row.get(2)?,
        order: row.get(3)?,
        dictionary_id: row.get(4)?,
    })
}

/// 新建 template 表和 template_field 表。
///
/// # 参数
/// - `connection`: 数据库连接。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn create_table(connection: &Connection) -> Result<(), ErrorCode> {
    let table = Table::create()
        .table(TemplateIden::Table)
        .col(
            ColumnDef::new_with_type(TemplateIden::Id, ColumnType::custom("TEXT"))
                .primary_key()
                .not_null(),
        )
        .col(
            ColumnDef::new_with_type(TemplateIden::Name, ColumnType::custom("TEXT"))
                .unique_key()
                .not_null(),
        )
        .col(
            ColumnDef::new_with_type(TemplateIden::Order, ColumnType::custom("INTEGER")).not_null(),
        )
        .extra("STRICT")
        .take();
    let sql = table.to_string(SqliteQueryBuilder);
    connection
        .execute(&sql, [])
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;

    let mut fk_template = ForeignKey::create();
    fk_template
        .from(TemplateFieldIden::Table, TemplateFieldIden::TemplateId)
        .to(TemplateIden::Table, TemplateIden::Id)
        .on_delete(ForeignKeyAction::Cascade);
    let mut pk = Index::create();
    pk.primary()
        .col(TemplateFieldIden::TemplateId)
        .col(TemplateFieldIden::Name);
    let table = Table::create()
        .table(TemplateFieldIden::Table)
        .col(
            ColumnDef::new_with_type(TemplateFieldIden::TemplateId, ColumnType::custom("TEXT"))
                .not_null(),
        )
        .col(ColumnDef::new_with_type(TemplateFieldIden::Name, ColumnType::custom("TEXT")).not_null())
        .col(
            ColumnDef::new_with_type(TemplateFieldIden::FieldType, ColumnType::custom("TEXT"))
                .not_null(),
        )
        .col(
            ColumnDef::new_with_type(TemplateFieldIden::Order, ColumnType::custom("INTEGER"))
                .not_null(),
        )
        .col(ColumnDef::new_with_type(TemplateFieldIden::DictionaryId, ColumnType::custom("TEXT")))
        .foreign_key(&mut fk_template)
        .index(&mut pk)
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

/// 向 template 表插入一个模板。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `template`: 要插入的模板。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn insert(connection: &Connection, template: &Template) -> Result<(), ErrorCode> {
    let query = Query::insert()
        .into_table(TemplateIden::Table)
        .columns([TemplateIden::Id, TemplateIden::Name, TemplateIden::Order])
        .values_panic([
            (&template.id).into(),
            (&template.name).into(),
            template.order.into(),
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

/// 查询全部模板，按 "order" 升序。
///
/// # 参数
/// - `connection`: 数据库连接。
///
/// # 返回值
/// 返回查询到的模板列表；若发生错误则返回对应的 `ErrorCode`。
pub fn select_all(connection: &Connection) -> Result<Vec<Template>, ErrorCode> {
    let query = Query::select()
        .columns([TemplateIden::Id, TemplateIden::Name, TemplateIden::Order])
        .from(TemplateIden::Table)
        .order_by(TemplateIden::Order, Order::Asc)
        .take();
    let (sql, values) = query.build(SqliteQueryBuilder);
    let mut statement = connection
        .prepare(&sql)
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    let rows = statement
        .query_map(rusqlite::params_from_iter(values_to_params(values)), map_template_row)
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })
}

/// 按 id 查询模板。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `id`: 模板 id。
///
/// # 返回值
/// 返回查询到的模板，不存在时返回 `None`；若发生错误则返回对应的 `ErrorCode`。
pub fn select_by_id(connection: &Connection, id: &str) -> Result<Option<Template>, ErrorCode> {
    let query = Query::select()
        .columns([TemplateIden::Id, TemplateIden::Name, TemplateIden::Order])
        .from(TemplateIden::Table)
        .and_where(Expr::col(TemplateIden::Id).eq(id))
        .take();
    let (sql, values) = query.build(SqliteQueryBuilder);
    connection
        .query_row(
            &sql,
            rusqlite::params_from_iter(values_to_params(values)),
            map_template_row,
        )
        .optional()
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })
}

/// 按 name 查询模板。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `name`: 模板名称。
///
/// # 返回值
/// 返回查询到的模板，不存在时返回 `None`；若发生错误则返回对应的 `ErrorCode`。
pub fn select_by_name(
    connection: &Connection,
    name: &str,
) -> Result<Option<Template>, ErrorCode> {
    let query = Query::select()
        .columns([TemplateIden::Id, TemplateIden::Name, TemplateIden::Order])
        .from(TemplateIden::Table)
        .and_where(Expr::col(TemplateIden::Name).eq(name))
        .take();
    let (sql, values) = query.build(SqliteQueryBuilder);
    connection
        .query_row(
            &sql,
            rusqlite::params_from_iter(values_to_params(values)),
            map_template_row,
        )
        .optional()
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })
}

/// 更新模板名称。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `id`: 模板 id。
/// - `new_name`: 新名称。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn update_name(
    connection: &Connection,
    id: &str,
    new_name: &str,
) -> Result<(), ErrorCode> {
    let query = Query::update()
        .table(TemplateIden::Table)
        .value(TemplateIden::Name, new_name)
        .and_where(Expr::col(TemplateIden::Id).eq(id))
        .take();
    let (sql, values) = query.build(SqliteQueryBuilder);
    connection
        .execute(&sql, rusqlite::params_from_iter(values_to_params(values)))
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    Ok(())
}

/// 按 id 删除模板。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `id`: 模板 id。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn delete_by_id(connection: &Connection, id: &str) -> Result<(), ErrorCode> {
    let query = Query::delete()
        .from_table(TemplateIden::Table)
        .and_where(Expr::col(TemplateIden::Id).eq(id))
        .take();
    let (sql, values) = query.build(SqliteQueryBuilder);
    connection
        .execute(&sql, rusqlite::params_from_iter(values_to_params(values)))
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    Ok(())
}

/// 删除全部 template 记录。
///
/// # 参数
/// - `connection`: 数据库连接。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn delete_all_templates(connection: &Connection) -> Result<(), ErrorCode> {
    let query = Query::delete()
        .from_table(TemplateIden::Table)
        .take();
    let (sql, values) = query.build(SqliteQueryBuilder);
    connection
        .execute(&sql, rusqlite::params_from_iter(values_to_params(values)))
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    Ok(())
}

/// 删除全部 template_field 记录。
///
/// # 参数
/// - `connection`: 数据库连接。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn delete_all_fields(connection: &Connection) -> Result<(), ErrorCode> {
    let query = Query::delete()
        .from_table(TemplateFieldIden::Table)
        .take();
    let (sql, values) = query.build(SqliteQueryBuilder);
    connection
        .execute(&sql, rusqlite::params_from_iter(values_to_params(values)))
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    Ok(())
}

/// 查询 template 表中 "order" 的最大值，空表返回 -1。
///
/// # 参数
/// - `connection`: 数据库连接。
///
/// # 返回值
/// 返回最大 "order" 值（-1 表示表为空）；若发生错误则返回对应的 `ErrorCode`。
pub fn max_order(connection: &Connection) -> Result<i64, ErrorCode> {
    let query = Query::select()
        .expr(Func::coalesce([Expr::col(TemplateIden::Order).max(), Expr::val(-1)]))
        .from(TemplateIden::Table)
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

/// 向 template_field 表插入一条字段定义。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `field`: 要插入的模板字段。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn insert_field(connection: &Connection, field: &TemplateField) -> Result<(), ErrorCode> {
    let query = Query::insert()
        .into_table(TemplateFieldIden::Table)
        .columns([
            TemplateFieldIden::TemplateId,
            TemplateFieldIden::Name,
            TemplateFieldIden::FieldType,
            TemplateFieldIden::Order,
            TemplateFieldIden::DictionaryId,
        ])
        .values_panic([
            (&field.template_id).into(),
            (&field.name).into(),
            (&field.field_type).into(),
            field.order.into(),
            field.dictionary_id.clone().into(),
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

/// 按模板 id 查询其全部字段定义，按 "order" 升序。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `template_id`: 模板 id。
///
/// # 返回值
/// 返回查询到的字段定义列表；若发生错误则返回对应的 `ErrorCode`。
pub fn select_fields_by_template_id(
    connection: &Connection,
    template_id: &str,
) -> Result<Vec<TemplateField>, ErrorCode> {
    let query = Query::select()
        .columns([
            TemplateFieldIden::TemplateId,
            TemplateFieldIden::Name,
            TemplateFieldIden::FieldType,
            TemplateFieldIden::Order,
            TemplateFieldIden::DictionaryId,
        ])
        .from(TemplateFieldIden::Table)
        .and_where(Expr::col(TemplateFieldIden::TemplateId).eq(template_id))
        .order_by(TemplateFieldIden::Order, Order::Asc)
        .take();
    let (sql, values) = query.build(SqliteQueryBuilder);
    let mut statement = connection
        .prepare(&sql)
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    let rows = statement
        .query_map(rusqlite::params_from_iter(values_to_params(values)), map_field_row)
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })
}

/// 删除指定模板的全部字段定义。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `template_id`: 模板 id。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn delete_fields_by_template_id(
    connection: &Connection,
    template_id: &str,
) -> Result<(), ErrorCode> {
    let query = Query::delete()
        .from_table(TemplateFieldIden::Table)
        .and_where(Expr::col(TemplateFieldIden::TemplateId).eq(template_id))
        .take();
    let (sql, values) = query.build(SqliteQueryBuilder);
    connection
        .execute(&sql, rusqlite::params_from_iter(values_to_params(values)))
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })?;
    Ok(())
}

/// 将引用了已不存在字典条目的 template_field.dictionary_id 置空。
///
/// 该 SQL 依赖 dictionary 表已存在。
///
/// # 参数
/// - `connection`: 数据库连接。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn clear_dangling_field_dictionary_ids(connection: &Connection) -> Result<(), ErrorCode> {
    let mut sub = Query::select();
    sub.column(DictionaryIden::Id).from(DictionaryIden::Table);
    let query = Query::update()
        .table(TemplateFieldIden::Table)
        .value(TemplateFieldIden::DictionaryId, None::<String>)
        .and_where(Expr::col(TemplateFieldIden::DictionaryId).is_not_null())
        .and_where(Expr::col(TemplateFieldIden::DictionaryId).not_in_subquery(sub.take()))
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

    /// 构造测试用 Template。
    fn tmpl(id: &str, name: &str, order: i64) -> Template {
        Template {
            id: id.to_string(),
            name: name.to_string(),
            order,
        }
    }

    /// 构造测试用 TemplateField。
    fn tf(template_id: &str, name: &str, order: i64) -> TemplateField {
        TemplateField {
            template_id: template_id.to_string(),
            name: name.to_string(),
            field_type: "string:single-line".to_string(),
            order,
            dictionary_id: None,
        }
    }

    /// 覆盖 template dao 模块所有 dao 函数的成功与失败路径。
    #[test]
    fn test_template_dao_all_functions() {
        let connection = Connection::open_in_memory().unwrap();
        // dao 单表测试聚焦本表 SQL，关闭外键以隔离父表依赖；外键级联行为由 service 测试端到端覆盖。
        connection
            .execute_batch("PRAGMA foreign_keys = OFF;")
            .unwrap();

        // insert 失败路径：表不存在时报 DatabaseError。
        assert!(matches!(
            insert(&connection, &tmpl("t1", "tpl-1", 1)),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // insert_field 失败路径：表不存在时报 DatabaseError。
        assert!(matches!(
            insert_field(&connection, &tf("t1", "f1", 1)),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // delete_all_templates 失败路径：表不存在时报 DatabaseError。
        assert!(matches!(
            delete_all_templates(&connection),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // delete_all_fields 失败路径：表不存在时报 DatabaseError。
        assert!(matches!(
            delete_all_fields(&connection),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // create_table 成功路径。
        create_table(&connection).unwrap();

        // create_table 失败路径：重复建表报 DatabaseError。
        assert!(matches!(
            create_table(&connection),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // == template 表测试 ==

        // select_all/max_order 空表：select_all 为空，max_order 返回 -1。
        assert!(select_all(&connection).unwrap().is_empty());
        assert_eq!(max_order(&connection).unwrap(), -1);

        // insert 成功路径：插入后 select_all 按 order 升序取回。
        insert(&connection, &tmpl("t3", "tpl-3", 30)).unwrap();
        insert(&connection, &tmpl("t1", "tpl-1", 10)).unwrap();
        insert(&connection, &tmpl("t2", "tpl-2", 20)).unwrap();
        let all = select_all(&connection).unwrap();
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].id, "t1");
        assert_eq!(all[1].id, "t2");
        assert_eq!(all[2].id, "t3");
        assert_eq!(max_order(&connection).unwrap(), 30);

        // select_by_id 成功路径：存在返回 Some，不存在返回 None。
        let found = select_by_id(&connection, "t1").unwrap().unwrap();
        assert_eq!(found.name, "tpl-1");
        assert!(select_by_id(&connection, "t-x").unwrap().is_none());

        // select_by_name 成功路径：存在返回 Some，不存在返回 None。
        let found = select_by_name(&connection, "tpl-2").unwrap().unwrap();
        assert_eq!(found.id, "t2");
        assert!(select_by_name(&connection, "no-such").unwrap().is_none());

        // update_name 成功路径。
        update_name(&connection, "t1", "tpl-1-updated").unwrap();
        let updated = select_by_id(&connection, "t1").unwrap().unwrap();
        assert_eq!(updated.name, "tpl-1-updated");

        // delete_by_id 成功路径：删除后查不到该记录。
        delete_by_id(&connection, "t1").unwrap();
        assert!(select_by_id(&connection, "t1").unwrap().is_none());
        assert_eq!(select_all(&connection).unwrap().len(), 2);

        // insert 失败路径：id 重复报 DatabaseError（主键约束）。
        assert!(matches!(
            insert(&connection, &tmpl("t2", "dup-id", 99)),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // insert 失败路径：name 重复（不同 id）报 DatabaseError（UNIQUE 约束）。
        assert!(matches!(
            insert(&connection, &tmpl("t4", "tpl-2", 40)),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // == template_field 表测试 ==

        // insert_field 成功路径：insert 后 select_fields 按 order 升序取回。
        insert_field(&connection, &tf("t2", "f3", 3)).unwrap();
        insert_field(&connection, &tf("t2", "f1", 1)).unwrap();
        insert_field(&connection, &tf("t2", "f2", 2)).unwrap();
        let fields = select_fields_by_template_id(&connection, "t2").unwrap();
        assert_eq!(fields.len(), 3);
        assert_eq!(fields[0].name, "f1");
        assert_eq!(fields[1].name, "f2");
        assert_eq!(fields[2].name, "f3");

        // dictionary_id 为 Some 和 None 的往返一致。
        let mut rich = tf("t3", "rich", 1);
        rich.dictionary_id = Some("dict-1".to_string());
        let t3 = tmpl("t3-x", "tpl-3-x", 40);
        insert(&connection, &t3).unwrap();
        // 用新模板 id
        rich.template_id = "t3-x".to_string();
        insert_field(&connection, &rich).unwrap();
        let selected = select_fields_by_template_id(&connection, "t3-x").unwrap();
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].dictionary_id.as_deref(), Some("dict-1"));

        // insert_field 失败路径：联合主键重复报 DatabaseError。
        assert!(matches!(
            insert_field(&connection, &tf("t2", "f1", 99)),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // delete_fields_by_template_id 成功路径：只删目标模板字段，其它模板不受影响。
        insert_field(&connection, &tf("t99", "fx", 1)).unwrap();
        delete_fields_by_template_id(&connection, "t2").unwrap();
        assert!(select_fields_by_template_id(&connection, "t2").unwrap().is_empty());
        assert_eq!(select_fields_by_template_id(&connection, "t99").unwrap().len(), 1);

        // delete_all_templates 成功路径：删除全部 template 记录。
        delete_all_templates(&connection).unwrap();
        assert!(select_all(&connection).unwrap().is_empty());

        // delete_all_fields 成功路径：删除全部 template_field 记录。
        delete_all_fields(&connection).unwrap();
        assert!(select_fields_by_template_id(&connection, "t99").unwrap().is_empty());

        // clear_dangling_field_dictionary_ids 成功路径。
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

        // 插入两条 template_field：一条引用存在的 "dict-1"，一条引用不存在的 "dict-x"。
        let mut with_existing = tf("t-clear", "f-exist", 1);
        with_existing.dictionary_id = Some("dict-1".to_string());
        let t_clear = tmpl("t-clear", "tpl-clear", 50);
        insert(&connection, &t_clear).unwrap();
        insert_field(&connection, &with_existing).unwrap();
        let mut with_dangling = tf("t-clear", "f-dangling", 2);
        with_dangling.dictionary_id = Some("dict-x".to_string());
        insert_field(&connection, &with_dangling).unwrap();
        // 插入一条 dictionary_id 为 None 的字段作为对照组。
        insert_field(&connection, &tf("t-clear", "f-none", 3)).unwrap();

        // 执行清理。
        clear_dangling_field_dictionary_ids(&connection).unwrap();

        let after = select_fields_by_template_id(&connection, "t-clear").unwrap();
        let exist_field = after.iter().find(|f| f.name == "f-exist").unwrap();
        assert_eq!(exist_field.dictionary_id.as_deref(), Some("dict-1"));
        let dangling_field = after.iter().find(|f| f.name == "f-dangling").unwrap();
        assert!(dangling_field.dictionary_id.is_none());
        let none_field = after.iter().find(|f| f.name == "f-none").unwrap();
        assert!(none_field.dictionary_id.is_none());
    }

    /// STRICT 生效验证：向 template 表的 TEXT 列（name）插入 BLOB 值时被数据库拒绝（非 STRICT 表会静默接受）。
    #[test]
    fn test_template_strict_type_enforced() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch("PRAGMA foreign_keys = OFF;")
            .unwrap();
        create_table(&connection).unwrap();
        assert!(matches!(
            connection.execute(
                "INSERT INTO template (id, name, \"order\") VALUES ('strict-violation', x'0102', 1)",
                [],
            ),
            Err(_)
        ));
    }

    /// STRICT 生效验证：向 template_field 表的 TEXT 列（field_type）插入 BLOB 值时被数据库拒绝（非 STRICT 表会静默接受）。
    #[test]
    fn test_template_field_strict_type_enforced() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch("PRAGMA foreign_keys = OFF;")
            .unwrap();
        create_table(&connection).unwrap();
        assert!(matches!(
            connection.execute(
                "INSERT INTO template_field (template_id, name, field_type, \"order\", dictionary_id)
                VALUES ('template-1', 'strict-violation', x'0102', 1, NULL)",
                [],
            ),
            Err(_)
        ));
    }
}