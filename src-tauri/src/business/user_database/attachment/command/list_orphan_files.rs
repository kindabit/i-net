use crate::business::user_database::attachment::service;
use crate::error_code::ErrorCode;

/// 列出孤儿附件文件：附件目录中存在、但 attachment 表中没有对应元数据的文件。
///
/// # 返回值
/// 返回孤儿文件的 id（解析不出 uuid 时为原文件名）；若发生错误则返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_attachment_list_orphan_files() -> Result<Vec<String>, ErrorCode> {
    preprocess()
}

/// `user_database_attachment_list_orphan_files` 的 preprocess 函数：无参，直接接入 service 层的 list_orphan_files 函数。
pub fn preprocess() -> Result<Vec<String>, ErrorCode> {
    service::list_orphan_files()
}
