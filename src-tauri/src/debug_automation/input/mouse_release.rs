//! `mouse_release` 命令：在当前光标位置松开鼠标按键。

use enigo::{Direction, Enigo, Mouse};

use super::{
    ensure_foreground, input_error, map_button, AppState, AutomationResult, MouseReleaseCommand,
    PressedState,
};

/// 执行 `mouse_release` 命令：在当前光标位置松开鼠标按键。
///
/// 不检查按键是否已经处于按下状态，直接注入。
///
/// # 参数
/// - `state`：共享应用程序状态。
/// - `enigo`：整个队列共用的原生输入句柄。
/// - `pressed`：本次请求的按下状态追踪。
/// - `command`：命令参数。
///
/// # 返回值
/// 注入松开后返回 `Ok(())`；原生注入失败时返回英文错误。
pub(super) fn execute(
    state: &AppState,
    enigo: &mut Enigo,
    pressed: &mut PressedState,
    command: &MouseReleaseCommand,
) -> AutomationResult<()> {
    let button = map_button(command.button);

    ensure_foreground(state)?;
    enigo
        .button(button, Direction::Release)
        .map_err(|error| input_error("failed to release the mouse button", error))?;
    pressed.release_mouse(button);
    Ok(())
}
