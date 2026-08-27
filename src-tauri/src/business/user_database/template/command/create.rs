use crate::business::user_database::entity::Template;
use crate::business::user_database::template::service;
use crate::error_code::ErrorCode;

/// 新建一个模板。
///
/// # 参数
/// - `name`: 模板名称。
///
/// # 返回值
/// 返回新建的模板；若发生错误则返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_template_create(name: String) -> Result<Template, ErrorCode> {
    preprocess(name)
}

/// `user_database_template_create` 的 preprocess 函数：校验参数后接入 service 层的 create 函数。
pub fn preprocess(name: String) -> Result<Template, ErrorCode> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(ErrorCode::EmptyTemplateName);
    }
    service::create(name)
}
