use std::path::PathBuf;

use crate::error_code::ErrorCode;

/// 获取当前用户的主目录。
///
/// # 参数
///
/// 无。
///
/// # 返回值
///
/// 返回当前用户主目录的路径；无法确定主目录时返回
/// [`ErrorCode::FailToDetermineHomeDirectory`]。
pub fn home_directory() -> Result<PathBuf, ErrorCode> {
    directories::UserDirs::new()
        .map(|user_dirs| user_dirs.home_dir().to_path_buf())
        .ok_or_else(|| ErrorCode::FailToDetermineHomeDirectory {
            detail: "failed to determine the user home directory".to_string(),
        })
}
