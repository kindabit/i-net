use crate::business::user_database::template::service;
use crate::error_code::ErrorCode;

/// 重命名指定模板。
///
/// # 参数
/// - `id`: 模板 id。
/// - `new_name`: 新模板名称。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_template_rename(id: String, new_name: String) -> Result<(), ErrorCode> {
    preprocess(id, new_name)
}

/// `user_database_template_rename` 的 preprocess 函数：校验参数后接入 service 层的 rename 函数。
pub fn preprocess(id: String, new_name: String) -> Result<(), ErrorCode> {
    let id = preprocess_template_id(id)?;
    let new_name = new_name.trim().to_string();
    if new_name.is_empty() {
        return Err(ErrorCode::EmptyTemplateName);
    }
    service::rename(&id, new_name)
}

/// 预处理模板 id：去除首尾空白字符，并校验 id 是标准小写连字符格式的 uuid。
fn preprocess_template_id(id: String) -> Result<String, ErrorCode> {
    let id = id.trim().to_string();
    match uuid::Uuid::parse_str(&id) {
        Ok(uuid) if uuid.to_string() == id => Ok(id),
        _ => Err(ErrorCode::InvalidTemplateId { id }),
    }
}
