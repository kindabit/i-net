//! `mouse_press` 命令：在当前光标位置按下鼠标按键。

use enigo::{Direction, Enigo, Mouse};

use super::{
    ensure_foreground, input_error, map_button, AppState, AutomationResult, MousePressCommand,
    PressedState,
};

/// 执行 `mouse_press` 命令：在当前光标位置按下鼠标按键。
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
/// 注入按下后返回 `Ok(())`；原生注入失败时返回英文错误。
pub(super) fn execute(
    state: &AppState,
    enigo: &mut Enigo,
    pressed: &mut PressedState,
    command: &MousePressCommand,
) -> AutomationResult<()> {
    let button = map_button(command.button);

    ensure_foreground(state)?;
    enigo
        .button(button, Direction::Press)
        .map_err(|error| input_error("failed to press the mouse button", error))?;
    pressed.press_mouse(button);
    Ok(())
}
