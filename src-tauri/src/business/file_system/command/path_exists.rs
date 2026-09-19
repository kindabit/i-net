//! `file_system_path_exists` 命令：判断指定路径是否存在。

use std::path::Path;

use crate::error_code::ErrorCode;
use crate::util::file_system_util;

/// 命令入口：判断指定路径是否存在。
///
/// 该接口为只读接口，供前端保存模式的覆盖确认使用。
///
/// # 参数
/// - `path`：需要检查的路径。
///
/// # 返回值
/// 返回路径是否存在；检查失败时返回对应的 `ErrorCode`。
#[tauri::command]
pub fn file_system_path_exists(path: String) -> Result<bool, ErrorCode> {
    file_system_util::try_exists(Path::new(&path))
}
