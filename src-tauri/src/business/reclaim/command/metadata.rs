//! `reclaim_metadata` 命令：重新读取 metadata 数据库的磁盘文件，替换内存 connection。

use crate::business::metadata;
use crate::error_code::ErrorCode;

/// 重新读取 metadata 数据库的磁盘文件，替换内存 connection。
#[tauri::command]
pub fn reclaim_metadata() -> Result<(), ErrorCode> {
    tracing::info!("[RECLAIM] metadata: re-reading from disk");
    metadata::service::initialize()
}
