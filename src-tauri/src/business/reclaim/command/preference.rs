//! `reclaim_preference` 命令：重新读取 preference 数据库的磁盘文件，替换内存 connection。

use crate::business::preference;
use crate::error_code::ErrorCode;

/// 重新读取 preference 数据库的磁盘文件，替换内存 connection。
#[tauri::command]
pub fn reclaim_preference() -> Result<(), ErrorCode> {
    tracing::info!("[RECLAIM] preference: re-reading from disk");
    preference::service::initialize()
}
