use crate::business::user_database::template::service;
use crate::business::user_database::template::vo::TemplateFieldVO;
use crate::error_code::ErrorCode;

/// 获取指定模板的全部字段定义。
///
/// # 参数
/// - `id`: 模板 id。
///
/// # 返回值
/// 返回字段值对象列表；若发生错误则返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_template_get_fields(id: String) -> Result<Vec<TemplateFieldVO>, ErrorCode> {
    preprocess(id)
}

/// `user_database_template_get_fields` 的 preprocess 函数：校验参数后接入 service 层的 get_fields 函数。
pub fn preprocess(id: String) -> Result<Vec<TemplateFieldVO>, ErrorCode> {
    let id = preprocess_template_id(id)?;
    service::get_fields(&id)
}

/// 预处理模板 id：去除首尾空白字符，并校验 id 是标准小写连字符格式的 uuid。
fn preprocess_template_id(id: String) -> Result<String, ErrorCode> {
    let id = id.trim().to_string();
    match uuid::Uuid::parse_str(&id) {
        Ok(uuid) if uuid.to_string() == id => Ok(id),
        _ => Err(ErrorCode::InvalidTemplateId { id }),
    }
}
