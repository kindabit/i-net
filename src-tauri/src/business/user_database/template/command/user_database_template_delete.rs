use crate::business::user_database::template::service;
use crate::error_code::ErrorCode;

/// 物理删除指定模板。
///
/// # 参数
/// - `id`: 模板 id。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_template_delete(id: String) -> Result<(), ErrorCode> {
    preprocess(id)
}

/// `user_database_template_delete` 的 preprocess 函数：校验参数后接入 service 层的 delete 函数。
pub fn preprocess(id: String) -> Result<(), ErrorCode> {
    let id = preprocess_template_id(id)?;
    service::delete(&id)
}

/// 预处理模板 id：去除首尾空白字符，并校验 id 是标准小写连字符格式的 uuid。
fn preprocess_template_id(id: String) -> Result<String, ErrorCode> {
    let id = id.trim().to_string();
    match uuid::Uuid::parse_str(&id) {
        Ok(uuid) if uuid.to_string() == id => Ok(id),
        _ => Err(ErrorCode::InvalidTemplateId { id }),
    }
}
