//! `file_system_roots` 命令：返回文件系统根路径列表。

use crate::business::file_system::service;
use crate::error_code::ErrorCode;

/// 命令入口：返回当前平台的文件系统根路径列表。
///
/// # 参数
/// 无。
///
/// # 返回值
/// Windows 平台返回各磁盘挂载点（形如 `C:\`）；其它平台返回 `/`。
#[tauri::command]
pub fn file_system_roots() -> Result<Vec<String>, ErrorCode> {
    service::roots()
}
