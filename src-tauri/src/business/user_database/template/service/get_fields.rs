use crate::business::user_database::template::dao;
use crate::business::user_database::template::vo::TemplateFieldVO;
use crate::business::user_database::state;
use crate::error_code::ErrorCode;

/// 获取指定模板的全部字段定义。
///
/// # 参数
/// - `template_id`: 模板 id。
///
/// # 返回值
/// 返回字段值对象列表；模板不存在时返回 `ErrorCode::NoTemplateWithSuchId`，
/// 发生其他错误时返回对应的 `ErrorCode`。
pub fn get_fields(template_id: &str) -> Result<Vec<TemplateFieldVO>, ErrorCode> {
    let connection = state::lock_connection();
    dao::select_by_id(&connection, template_id)?
        .ok_or_else(|| ErrorCode::NoTemplateWithSuchId {
            id: template_id.to_string(),
        })?;
    let fields = dao::select_fields_by_template_id(&connection, template_id)?;
    let mut vos = Vec::with_capacity(fields.len());
    for field in fields {
        vos.push(TemplateFieldVO {
            name: field.name,
            field_type: field.field_type,
            dictionary_id: field.dictionary_id,
        });
    }
    Ok(vos)
}
