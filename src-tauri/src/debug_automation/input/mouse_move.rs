//! `mouse_move` 命令：把光标平滑移动到目标位置。

use enigo::{Enigo, Mouse};

use super::{
    ensure_foreground, input_error, move_smooth, platform, resolve_screen_point, AppState,
    AutomationResult, MouseMoveCommand,
};

/// 执行 `mouse_move` 命令：把光标平滑移动到目标位置。
///
/// # 参数
/// - `state`：共享应用程序状态。
/// - `enigo`：整个队列共用的原生输入句柄。
/// - `command`：目标位置（以主窗口物理像素表示）。
/// - `speed`：本次移动使用的物理像素每秒速度。
///
/// # 返回值
/// 移动完成后返回 `Ok(())`；坐标无效、移动超时或原生注入失败时返回英文错误。
pub(super) fn execute(
    state: &AppState,
    enigo: &mut Enigo,
    command: &MouseMoveCommand,
    speed: f64,
) -> AutomationResult<()> {
    let main = platform::main_raw_window(state)?;
    let target = resolve_screen_point(&main, command.x, command.y)?;

    ensure_foreground(state)?;
    let current = enigo
        .location()
        .map_err(|error| input_error("failed to query the cursor position", error))?;

    move_smooth(enigo, current, target, speed)
}
