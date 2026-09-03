use std::collections::HashSet;

use crate::business::user_database::entity::{Action, TemplateField};
use crate::business::user_database::template::dao;
use crate::business::user_database::template::vo::TemplateFieldVO;
use crate::business::user_database::{dictionary, log, state};
use crate::error_code::ErrorCode;

/// 设置指定模板的字段集合（全量覆盖）：先删除旧字段再逐条插入新字段。
///
/// 产生 TemplateFieldsSet 日志，载荷为模板名称和字段名称列表。
/// 字段类型与字段配置对后端不透明，此处仅校验字段名唯一性与字典引用存在性。
///
/// # 参数
/// - `template_id`: 模板 id。
/// - `fields`: 要设置的字段列表，顺序即存储顺序。
///
/// # 返回值
/// 成功时返回 `Ok(())`；模板不存在时返回 `ErrorCode::NoTemplateWithSuchId`，
/// 字段名称重复时返回 `ErrorCode::DuplicateNodeFieldName`，
/// 发生其他错误时返回对应的 `ErrorCode`。
pub fn set_fields(template_id: &str, fields: &[TemplateFieldVO]) -> Result<(), ErrorCode> {
    let connection = state::lock_connection();
    let template = dao::select_by_id(&connection, template_id)?
        .ok_or_else(|| ErrorCode::NoTemplateWithSuchId {
            id: template_id.to_string(),
        })?;
    let template_name = template.name;

    let mut seen = HashSet::new();
    for f in fields {
        if !seen.insert(&f.name) {
            return Err(ErrorCode::DuplicateNodeFieldName {
                name: f.name.clone(),
            });
        }
    }

    for f in fields {
        if let Some(ref dict_id) = f.dictionary_id {
            if !dictionary::dao::exist_by_id(&connection, dict_id)? {
                return Err(ErrorCode::NoDictionaryEntryWithSuchId {
                    id: dict_id.clone(),
                });
            }
        }
    }

    dao::delete_fields_by_template_id(&connection, template_id)?;

    for (i, f) in fields.iter().enumerate() {
        let field = TemplateField {
            template_id: template_id.to_string(),
            name: f.name.clone(),
            field_type: f.field_type.clone(),
            order: i as i64,
            dictionary_id: f.dictionary_id.clone(),
        };
        dao::insert_field(&connection, &field)?;
    }

    let field_names: Vec<String> = fields.iter().map(|f| f.name.clone()).collect();
    log::service::create(
        template_id,
        Action::TemplateFieldsSet {
            template_name,
            field_names,
        },
    )?;
    Ok(())
}
