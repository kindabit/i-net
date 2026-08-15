use crate::business::user_database::dictionary::service;
use crate::business::user_database::entity::Dictionary;
use crate::error_code::ErrorCode;

/// 获取字典条目全量列表。
///
/// # 返回值
/// 返回字典条目列表；若发生错误则返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_dictionary_list() -> Result<Vec<Dictionary>, ErrorCode> {
    preprocess()
}

/// `user_database_dictionary_list` 的 preprocess 函数：无参，直接接入 service 层的 list 函数。
pub fn preprocess() -> Result<Vec<Dictionary>, ErrorCode> {
    service::list()
}
