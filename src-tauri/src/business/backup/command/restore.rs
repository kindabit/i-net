//! `restore` 命令：执行完整还原。
//!
//! 前端必须先经过 [`restore_probe`](crate::business::backup::command::restore_probe::backup_restore_probe)、
//! 弹出文件选择器让用户选定源路径后，再调用本命令。
//!
//! 流程：command 仅负责与 Tauri 交互（接收 IPC 参数、把 [`AppHandle`] 包装成
//! 进度回调闭包），转发给 [`preprocess`]；[`preprocess`] 校验路径非空后调用
//! [`service::unpack`]。preprocess 与 service 不依赖任何 Tauri 类型。

use tauri::AppHandle;

use crate::business::backup::command::progress::{progress_emitter, RESTORE_PROGRESS_EVENT};
use crate::business::backup::progress::Phase;
use crate::business::backup::service;
use crate::error_code::ErrorCode;
use crate::util::preprocess_util;

/// 命令入口：接收 IPC 参数（备份文件路径），把 [`AppHandle`] 包装成进度回调闭包，
/// 转发给 [`preprocess`]。
///
/// # 参数
/// - `app_handle`：Tauri 应用句柄（由 Tauri 自动注入）。
/// - `source_path`：备份文件的绝对路径（来自前端 `restore_probe` 选择的文件）。
///
/// # 返回值
/// 还原完成时返回 `Ok(())`；任意错误返回对应的 `ErrorCode`。
#[tauri::command]
pub fn backup_restore(app_handle: AppHandle, source_path: String) -> Result<(), ErrorCode> {
    let on_progress = progress_emitter(&app_handle, RESTORE_PROGRESS_EVENT);
    preprocess(source_path, &on_progress)
}

/// `restore` 命令的 preprocess 函数：清洗并校验路径非空，再调用 [`service::unpack`]。
///
/// # 参数
/// - `source_path`：备份文件的绝对路径。
/// - `on_progress`：进度回调，透传给 [`service::unpack`] 上报还原进度。
///
/// # 返回值
/// 成功时返回 `Ok(())`；路径非法或还原失败时返回对应的 `ErrorCode`。
pub fn preprocess(source_path: String, on_progress: &dyn Fn(Phase, f32)) -> Result<(), ErrorCode> {
    let resolved = preprocess_util::preprocess_file_path(source_path)?;
    service::unpack(std::path::Path::new(&resolved), on_progress)
}
