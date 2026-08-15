use crate::business::user_database::dictionary::dao as dictionary_dao;
use crate::business::user_database::template::dao as template_dao;
use crate::business::user_database::state;
use crate::error_code::ErrorCode;
use crate::util::file_system_util;

/// 导出模板数据到 SQLite 文件：将 template、template_field、dictionary 三张表的数据导出到目标文件。
/// 导出前删除目标文件以确保完全覆盖，使用 connection 模块处理 data_version。
///
/// # 参数
/// - `target_path`: 导出目标文件路径。
///
/// # 返回值
/// 成功时返回 `Ok(())`；发生错误时返回对应的 `ErrorCode`。
pub fn export(target_path: &str) -> Result<(), ErrorCode> {
    let source = state::lock_connection();

    let templates = template_dao::select_all(&source)?;
    let mut all_fields = Vec::new();
    for template in &templates {
        let mut fields = template_dao::select_fields_by_template_id(&source, &template.id)?;
        all_fields.append(&mut fields);
    }
    let dictionaries = dictionary_dao::select_all(&source)?;

    let path = std::path::Path::new(target_path);
    if file_system_util::try_exists(path)? {
        file_system_util::remove_file(path)?;
    }

    let target = crate::common::connection::service::open_file(path)?;

    template_dao::create_table(&target)?;
    dictionary_dao::create_table(&target)?;

    for template in &templates {
        template_dao::insert(&target, template)?;
    }

    for field in &all_fields {
        template_dao::insert_field(&target, field)?;
    }

    dictionary_dao::batch_insert(&target, &dictionaries)?;

    crate::common::connection::service::save_file(path, &target)?;

    Ok(())
}
