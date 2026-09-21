//! `mouse_click` 命令：在当前光标位置快速按下并松开鼠标按键若干次。

use std::time::Duration;

use enigo::{Direction, Enigo, Mouse};

use super::{
    ensure_foreground, input_error, map_button, validate_click_count, AppState, AutomationResult,
    MouseClickCommand, PressedState, MOUSE_CLICK_GAP,
};

/// 执行 `mouse_click` 命令：在当前光标位置快速点击鼠标按键若干次。
///
/// # 参数
/// - `state`：共享应用程序状态。
/// - `enigo`：整个队列共用的原生输入句柄。
/// - `pressed`：本次请求的按下状态追踪。
/// - `command`：鼠标按键与点击次数。
/// - `span`：每次点击按下与松开之间保持的时长。
///
/// # 返回值
/// 全部点击注入后返回 `Ok(())`；次数越界或原生注入失败时返回英文错误。
pub(super) fn execute(
    state: &AppState,
    enigo: &mut Enigo,
    pressed: &mut PressedState,
    command: &MouseClickCommand,
    span: Duration,
) -> AutomationResult<()> {
    validate_click_count(command.count)?;

    let button = map_button(command.button);
    ensure_foreground(state)?;

    for index in 0..command.count {
        if index > 0 {
            std::thread::sleep(MOUSE_CLICK_GAP);
        }
        enigo
            .button(button, Direction::Press)
            .map_err(|error| input_error("failed to press the mouse button", error))?;
        pressed.press_mouse(button);
        std::thread::sleep(span);
        enigo
            .button(button, Direction::Release)
            .map_err(|error| input_error("failed to release the mouse button", error))?;
        pressed.release_mouse(button);
    }
    Ok(())
}
