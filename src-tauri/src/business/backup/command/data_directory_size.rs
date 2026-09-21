//! `backup_data_directory_size` 命令：查询当前数据目录（除 `logs/` 外）的总字节数，
//! 用于前端预估备份体积。

use crate::business::backup::service;
use crate::error_code::ErrorCode;
use crate::state::path;

/// 提供给前端的"当前数据目录大小"查询入口。
///
/// 命令异步执行，避免统计大量文件时长时间占用窗口事件循环。
#[tauri::command(async)]
pub fn backup_data_directory_size() -> Result<u64, ErrorCode> {
    service::data_directory_size(&path().data_directory)
}