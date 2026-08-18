//! `backup` 命令：弹出系统保存对话框让用户选择目标文件，然后交给 [`preprocess`] 完成校验与打包。
//!
//! 流程：command 仅负责与 Tauri 交互（弹保存对话框、判断用户取消、
//! 把 [`AppHandle`] 包装成进度回调闭包）；选定路径后调用 [`preprocess`]，
//! 由它校验参数、补齐扩展名并调用 [`service::pack`]。
//! preprocess 与 service 不依赖任何 Tauri 类型。
//!
//! 用户在系统保存对话框中选择的路径若无 `.ibackup` 扩展名，会自动补上；
//! 这是为了与 [`restore`] 端的校验（依赖 `IBACKUP\0` magic）保持一致，
//! 同时让用户在文件管理器里能直观识别备份文件。
//!
//! 为防止备份覆盖应用自身的数据库/附件，目标路径必须位于 [`state::path::data_directory`] 之外，
//! 校验由 [`preprocess`] 内的 [`state::path::Path::ensure_outside_data_directory`] 完成。

use std::path::PathBuf;

use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;

use crate::business::backup::command::progress::{progress_emitter, BACKUP_PROGRESS_EVENT};
use crate::business::backup::progress::Phase;
use crate::business::backup::service;
use crate::error_code::ErrorCode;

/// 备份文件扩展名。
const BACKUP_EXTENSION: &str = "ibackup";

/// 命令入口：仅与 Tauri 交互 —— 弹出系统保存对话框让用户选择目标文件，
/// 用户取消时直接返回 `Ok(false)`；否则把 Tauri 的 [`tauri_plugin_dialog::FilePath`]
/// 转换为本地文件系统 [`PathBuf`]，并把 [`AppHandle`] 包装成进度回调闭包，
/// 一并交给 [`preprocess`]。
///
/// # 参数
/// - `app_handle`：Tauri 应用句柄（由 Tauri 自动注入）。
/// - `redundancy_ratio`：冗余比例，范围 `(0, 1)`。
///
/// # 返回值
/// - 用户取消对话框时返回 `Ok(false)`。
/// - 备份完成时返回 `Ok(true)`。
/// - 任意错误返回对应的 `ErrorCode`。
#[tauri::command]
pub fn backup_backup(app_handle: AppHandle, redundancy_ratio: f32) -> Result<bool, ErrorCode> {
    let target_path = app_handle.dialog().file().blocking_save_file();
    match target_path {
        Some(path) => {
            let path = path.into_path().map_err(|e| ErrorCode::InvalidPath {
                detail: format!(
                    "failed to convert selected file path to filesystem path: {}",
                    e
                ),
            })?;
            let on_progress = progress_emitter(&app_handle, BACKUP_PROGRESS_EVENT);
            preprocess(redundancy_ratio, path, &on_progress)?;
            Ok(true)
        }
        None => Ok(false),
    }
}

/// `backup` 命令的 preprocess 函数：校验冗余比例、拒绝位于应用数据目录内的目标路径、
/// 补齐文件扩展名并调用 [`service::pack`]。
///
/// # 参数
/// - `redundancy_ratio`：冗余比例，范围 `(0, 1)`。
/// - `target_path`：用户在保存对话框中选定的目标文件路径。
/// - `on_progress`：进度回调，透传给 [`service::pack`] 上报备份进度。
///
/// # 返回值
/// 成功时返回 `Ok(())`；冗余比例非法、目标路径位于数据目录内或打包失败时返回对应的 `ErrorCode`。
pub fn preprocess(
    redundancy_ratio: f32,
    target_path: PathBuf,
    on_progress: &dyn Fn(Phase, f32),
) -> Result<(), ErrorCode> {
    validate_redundancy_ratio(redundancy_ratio)?;
    crate::state::path().ensure_outside_data_directory(
        target_path.to_string_lossy().as_ref(),
    )?;
    let resolved = ensure_backup_extension(target_path);
    service::pack(&resolved, redundancy_ratio, on_progress)
}

/// 校验冗余比例是否落在 `(0, 1)` 开区间。
///
/// # 参数
/// - `redundancy_ratio`：冗余比例。
///
/// # 返回值
/// 成功时返回 `Ok(())`；比例非法时返回 `ErrorCode::InvalidBackupFile`。
pub(super) fn validate_redundancy_ratio(redundancy_ratio: f32) -> Result<(), ErrorCode> {
    if !(redundancy_ratio > 0.0 && redundancy_ratio < 1.0) {
        return Err(ErrorCode::InvalidBackupFile {
            detail: format!(
                "redundancy_ratio must be in (0, 1), got {}",
                redundancy_ratio
            ),
        });
    }
    Ok(())
}

/// 若路径没有 `.ibackup` 扩展名则自动补上；已有其他扩展名时替换为 `.ibackup`，
/// 避免出现 `backup.tar.ibackup` 之类的双扩展名。
///
/// # 参数
/// - `path`：用户选定的路径。
///
/// # 返回值
/// - 已有 `.ibackup` 扩展名（不区分大小写）则原样返回。
/// - 否则把文件名替换为 `<file_stem>.ibackup`。
fn ensure_backup_extension(path: PathBuf) -> PathBuf {
    let already_correct = path
        .extension()
        .map(|ext| ext.eq_ignore_ascii_case(BACKUP_EXTENSION))
        .unwrap_or(false);
    if already_correct {
        return path;
    }
    let stem = match path.file_stem() {
        Some(stem) if !stem.is_empty() => stem.to_os_string(),
        _ => return path,
    };
    let mut new_name = stem;
    new_name.push(".");
    new_name.push(BACKUP_EXTENSION);
    path.with_file_name(new_name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// 提取路径中的文件名部分，便于跨平台断言。
    fn file_name(p: &std::path::Path) -> std::ffi::OsString {
        p.file_name()
            .map(|n| n.to_os_string())
            .unwrap_or_default()
    }

    /// 覆盖 ensure_backup_extension：无扩展名时自动追加。
    #[test]
    fn test_ensure_backup_extension_adds_when_missing() {
        let p = ensure_backup_extension(PathBuf::from("/tmp/backup"));
        assert_eq!(file_name(&p), "backup.ibackup");
    }

    /// 覆盖 ensure_backup_extension：已有 `.ibackup` 扩展名时原样返回。
    #[test]
    fn test_ensure_backup_extension_keeps_existing() {
        let p = ensure_backup_extension(PathBuf::from("/tmp/backup.ibackup"));
        assert_eq!(file_name(&p), "backup.ibackup");
    }

    /// 覆盖 ensure_backup_extension：其他扩展名时替换为 `.ibackup`（避免双扩展名）。
    #[test]
    fn test_ensure_backup_extension_replaces_other() {
        let p = ensure_backup_extension(PathBuf::from("/tmp/backup.tar"));
        assert_eq!(file_name(&p), "backup.ibackup");
    }

    /// 覆盖 ensure_backup_extension：大写扩展名仍视为有效（不区分大小写）。
    #[test]
    fn test_ensure_backup_extension_case_insensitive() {
        let p = ensure_backup_extension(PathBuf::from("/tmp/backup.IBACKUP"));
        assert_eq!(file_name(&p), "backup.IBACKUP");
    }

    /// 覆盖 ensure_backup_extension：路径无文件名时原样返回（不 panic）。
    #[test]
    fn test_ensure_backup_extension_handles_empty_file_name() {
        let p = ensure_backup_extension(PathBuf::from("/"));
        assert_eq!(file_name(&p), std::ffi::OsString::new());
    }

    /// 覆盖 ensure_backup_extension：多扩展名（如 `a.bak`）只剥掉最后一层。
    #[test]
    fn test_ensure_backup_extension_handles_multi_extension() {
        let p = ensure_backup_extension(PathBuf::from("/tmp/backup.2024.bak"));
        assert_eq!(file_name(&p), "backup.2024.ibackup");
    }
}