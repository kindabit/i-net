//! `file_system_list_directory` 命令：列出指定目录下的条目。

use std::path::Path;

use crate::business::file_system::service;
use crate::business::file_system::vo::DirectoryListingVO;
use crate::error_code::ErrorCode;

/// 命令入口：列出指定目录下的条目，未指定路径时列出当前用户主目录。
///
/// # 参数
/// - `path`：需要列出的目录路径；为 `None` 时使用当前用户主目录。
///
/// # 返回值
/// 返回目录读取结果；路径不存在、不是目录或无法确定主目录时返回对应的 `ErrorCode`。
#[tauri::command]
pub fn file_system_list_directory(path: Option<String>) -> Result<DirectoryListingVO, ErrorCode> {
    match path {
        Some(path) => service::list_directory(Path::new(&path)),
        None => {
            let home = service::home_directory()?;
            service::list_directory(&home)
        }
    }
}
