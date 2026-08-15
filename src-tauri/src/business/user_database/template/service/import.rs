use crate::business::user_database::dictionary::dao as dictionary_dao;
use crate::business::user_database::dictionary::service::set as dictionary_set;
use crate::business::user_database::node_field::dao as node_field_dao;
use crate::business::user_database::template::dao as template_dao;
use crate::business::user_database::state;
use crate::common::connection::service::open_file;
use crate::error_code::ErrorCode;

/// 从 SQLite 文件导入模板数据：读取源文件的 template、template_field、dictionary 数据，
/// 替换当前数据库的三张表，并清理 node_field 和 template_field 中引用已不存在字典条目的悬空 id。
/// 使用 connection 模块打开源文件以校验 data_version。
///
/// # 参数
/// - `source_path`: 源文件路径。
///
/// # 返回值
/// 成功时返回 `Ok(())`；发生错误时返回对应的 `ErrorCode`。
pub fn import(source_path: &str) -> Result<(), ErrorCode> {
    let source = open_file(std::path::Path::new(source_path))?;

    let templates = template_dao::select_all(&source)?;
    let mut all_fields = Vec::new();
    for template in &templates {
        let mut fields = template_dao::select_fields_by_template_id(&source, &template.id)?;
        all_fields.append(&mut fields);
    }
    let dictionaries = dictionary_dao::select_all(&source)?;

    drop(source);

    let connection = state::lock_connection();

    template_dao::delete_all_fields(&connection)?;
    template_dao::delete_all_templates(&connection)?;
    dictionary_dao::delete_all(&connection)?;

    for template in &templates {
        template_dao::insert(&connection, template)?;
    }

    for field in &all_fields {
        template_dao::insert_field(&connection, field)?;
    }

    dictionary_set(&dictionaries)?;

    node_field_dao::clear_dangling_dictionary_ids(&connection)?;
    template_dao::clear_dangling_field_dictionary_ids(&connection)?;

    Ok(())
}
