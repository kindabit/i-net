//! `restore_probe` 命令：弹出文件对话框并做校验探测，不替换数据。

use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;

use crate::business::backup::command::response::ProbeResult;
use crate::business::backup::service;
use crate::error_code::ErrorCode;

/// 命令入口：弹出文件对话框并做校验探测，不替换数据。
///
/// # 参数
/// - `app_handle`：Tauri 应用句柄（由 Tauri 自动注入）。
///
/// # 返回值
/// - 用户取消对话框时返回 `Ok(None)`。
/// - 校验完成时返回 `Ok(Some(ProbeResult))`。
/// - 用户选择的路径无法转换为本地文件系统路径、或文件损坏不可读时返回对应的 `ErrorCode`。
#[tauri::command]
pub fn backup_restore_probe(app_handle: AppHandle) -> Result<Option<ProbeResult>, ErrorCode> {
    let target = match app_handle.dialog().file().blocking_pick_file() {
        Some(p) => p.into_path().map_err(|e| ErrorCode::InvalidPath {
            detail: format!(
                "failed to convert selected file path to filesystem path: {}",
                e
            ),
        })?,
        None => return Ok(None),
    };
    let (recoverable, lost, limit) = service::probe(&target)?;
    Ok(Some(ProbeResult {
        recoverable,
        lost,
        limit,
        source_path: target.to_string_lossy().to_string(),
    }))
}