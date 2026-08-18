//! 备份/还原进度上报：把业务层上报的进度转换为 Tauri 事件推送给前端。
//!
//! 前端通过 `@tauri-apps/api/event` 的 `listen("backup-progress", ...)`
//! 或 `listen("restore-progress", ...)` 订阅对应事件。
//! payload 结构见 [`ProgressPayload`]。
//!
//! command 层通过 [`progress_emitter`] 构造进度回调闭包并注入 preprocess/service，
//! 使 preprocess/service 不依赖任何 Tauri 类型。

use serde::Serialize;
use tauri::{AppHandle, Emitter};

pub use crate::business::backup::progress::Phase;

/// 进度事件名（备份）。
pub const BACKUP_PROGRESS_EVENT: &str = "backup-progress";

/// 进度事件名（还原）。
pub const RESTORE_PROGRESS_EVENT: &str = "restore-progress";

/// 单次进度事件的 payload。
#[derive(Debug, Clone, Serialize)]
pub struct ProgressPayload {
    /// 当前阶段。
    pub phase: Phase,
    /// 当前阶段内的进度，范围 0.0~1.0。
    pub progress: f32,
}

/// 构造进度回调闭包：捕获 `app_handle`，每次被调用时向 `event` 通道发送 [`ProgressPayload`]。
///
/// # 参数
/// - `app_handle`：Tauri 应用句柄。
/// - `event`：事件名，使用 [`BACKUP_PROGRESS_EVENT`] 或 [`RESTORE_PROGRESS_EVENT`]。
///
/// # 返回值
/// 返回 `Fn(Phase, f32)` 闭包，作为进度回调注入 preprocess/service；
/// 发送失败时仅记录日志，不向调用方回传（进度上报不应中断业务流程）。
pub fn progress_emitter<'a>(app_handle: &'a AppHandle, event: &'static str) -> impl Fn(Phase, f32) + 'a {
    move |phase, progress| {
        if let Err(error) = app_handle.emit(event, ProgressPayload { phase, progress }) {
            tracing::warn!("failed to emit {} event: {:?}", event, error);
        }
    }
}
