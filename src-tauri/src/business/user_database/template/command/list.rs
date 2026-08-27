use crate::business::user_database::entity::Template;
use crate::business::user_database::template::service;
use crate::error_code::ErrorCode;

/// 查询全部模板。
///
/// # 参数
/// 无。
///
/// # 返回值
/// 返回查询到的模板列表；若发生错误则返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_template_list() -> Result<Vec<Template>, ErrorCode> {
    preprocess()
}

/// `user_database_template_list` 的 preprocess 函数：无参数校验，直接接入 service 层的 list 函数。
pub fn preprocess() -> Result<Vec<Template>, ErrorCode> {
    service::list()
}
