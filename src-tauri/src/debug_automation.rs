//! 仅用于调试的自动化模块。
//!
//! 当 `debug-automation` feature 被启用时（仅在调试构建中；参见该 feature
//! 附加的 `debug_assertions` 门控），本模块会在回环地址上暴露一个 HTTP 服务器，
//! 使外部 LLM 代理能够检查 UI 树、捕获窗口并合成原生输入。本模块完全自包含，
//! 并且是其可选依赖的唯一使用方，因此发布构建中不包含它的任何痕迹。

pub mod capture;
pub mod input;
pub mod model;
pub mod platform;
pub mod server;
pub mod ui_tree;

use std::sync::Arc;

use tauri::{AppHandle, Manager};

/// 调试自动化服务器绑定的固定回环端口。
const PORT: u16 = 17432;

/// 传递给每个 HTTP 处理器的共享应用程序状态。
pub struct AppState {
    pub app_handle: AppHandle,
    /// 以原始 isize 表示的主窗口 HWND，与 Tauri 自身依赖的 windows crate
    /// 版本解耦；在不支持原生句柄的平台上为 `0`。
    pub main_hwnd: isize,
}

/// 初始化并启动调试自动化服务器。
///
/// 该函数由 Tauri 的 setup 钩子调用。所有失败模式都以英文错误消息返回，
/// 以便调用方记录日志；服务器失败不得中止应用程序启动。
///
/// # 参数
/// - `app_handle`：正在运行的 Tauri 应用程序的句柄。
///
/// # 返回值
/// 当服务器正在固定的回环端口上监听时返回 `Ok(())`。
pub fn initialize(app_handle: AppHandle) -> Result<(), String> {
    let main_window = app_handle
        .get_webview_window("main")
        .ok_or("main webview window not found")?;

    // 原生句柄仅在 Windows 上有意义；其它平台保留哨兵值，
    // 因为其截图后端尚未实现。
    #[cfg(windows)]
    let main_hwnd = main_window
        .hwnd()
        .map(|hwnd| hwnd.0 as isize)
        .map_err(|error| format!("failed to get main window hwnd: {error}"))?;
    #[cfg(not(windows))]
    let main_hwnd = 0isize;

    let state = Arc::new(AppState {
        app_handle: app_handle.clone(),
        main_hwnd,
    });

    server::start(state, PORT)
        .map_err(|error| format!("failed to start automation http server: {error}"))?;

    let base_url = format!("http://127.0.0.1:{PORT}");
    tracing::warn!(
        "DEBUG AUTOMATION SERVER IS RUNNING at {base_url}; \
         it exposes screenshots, the UI tree and native input injection and must never be \
         shipped in release builds"
    );
    Ok(())
}
