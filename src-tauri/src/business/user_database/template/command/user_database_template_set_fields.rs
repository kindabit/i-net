use crate::business::user_database::template::service;
use crate::business::user_database::template::vo::TemplateFieldVO;
use crate::error_code::ErrorCode;

/// 设置指定模板的字段集合（全量覆盖）。
///
/// # 参数
/// - `id`: 模板 id。
/// - `fields`: 要设置的字段列表。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_template_set_fields(
    id: String,
    fields: Vec<TemplateFieldVO>,
) -> Result<(), ErrorCode> {
    preprocess(id, fields)
}

/// `user_database_template_set_fields` 的 preprocess 函数：校验参数后接入 service 层的 set_fields 函数。
///
/// 字段名称 trim 后回写；dictionary_id 在 Some 时校验 uuid 格式。
pub fn preprocess(
    id: String,
    mut fields: Vec<TemplateFieldVO>,
) -> Result<(), ErrorCode> {
    let id = preprocess_template_id(id)?;
    for f in &mut fields {
        f.name = f.name.trim().to_string();
        if f.name.is_empty() {
            return Err(ErrorCode::EmptyNodeFieldName);
        }
        if let Some(ref dict_id) = f.dictionary_id {
            match uuid::Uuid::parse_str(dict_id) {
                Ok(uuid) if uuid.to_string() == *dict_id => {}
                _ => {
                    return Err(ErrorCode::InvalidDictionaryId {
                        id: dict_id.clone(),
                    })
                }
            }
        }
    }
    service::set_fields(&id, &fields)
}

/// 预处理模板 id：去除首尾空白字符，并校验 id 是标准小写连字符格式的 uuid。
fn preprocess_template_id(id: String) -> Result<String, ErrorCode> {
    let id = id.trim().to_string();
    match uuid::Uuid::parse_str(&id) {
        Ok(uuid) if uuid.to_string() == id => Ok(id),
        _ => Err(ErrorCode::InvalidTemplateId { id }),
    }
}
