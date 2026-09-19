//! `restore_probe` 命令：对前端文件选择器选定的备份文件做校验探测，不替换数据。
//!
//! 用户取消选择由前端文件选择器处理，命令本身不再涉及对话框交互。

use std::path::PathBuf;

use crate::business::backup::command::response::ProbeResult;
use crate::business::backup::service;
use crate::error_code::ErrorCode;

/// 命令入口：对前端文件选择器选定的备份文件做校验探测，不替换数据。
///
/// # 参数
/// - `source_path`：备份文件路径，由前端文件选择器提供。
///
/// # 返回值
/// 校验完成时返回 `Ok(ProbeResult)`（`source_path` 回显入参路径）；
/// 路径非法或文件损坏不可读时返回对应的 `ErrorCode`。
#[tauri::command]
pub fn backup_restore_probe(source_path: String) -> Result<ProbeResult, ErrorCode> {
    let target = PathBuf::from(source_path);
    let (recoverable, lost, recoverable_limit) = service::probe(&target)?;
    Ok(ProbeResult {
        recoverable,
        lost,
        recoverable_limit,
        source_path: target.to_string_lossy().to_string(),
    })
}
