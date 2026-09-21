//! `mouse_scroll` 命令：在当前光标位置滚动鼠标滚轮。

use enigo::{Axis, Enigo, Mouse};

use super::{
    ensure_foreground, input_error, validate_scroll_amount, AppState, AutomationResult,
    MouseScrollCommand, ScrollDirection, SCROLL_NOTCH_DELAY,
};

/// 执行 `mouse_scroll` 命令：在当前光标位置滚动鼠标滚轮。
///
/// # 参数
/// - `state`：共享应用程序状态。
/// - `enigo`：整个队列共用的原生输入句柄。
/// - `command`：滚动方向与滚轮刻度数。
///
/// # 返回值
/// 全部刻度注入后返回 `Ok(())`；刻度数越界或原生注入失败时返回英文错误。
pub(super) fn execute(
    state: &AppState,
    enigo: &mut Enigo,
    command: &MouseScrollCommand,
) -> AutomationResult<()> {
    validate_scroll_amount(command.amount)?;
    ensure_foreground(state)?;

    // 正值表示向右或向下，与滚轮增量约定一致。
    let (axis, direction) = match command.direction {
        ScrollDirection::Up => (Axis::Vertical, -1),
        ScrollDirection::Down => (Axis::Vertical, 1),
        ScrollDirection::Left => (Axis::Horizontal, -1),
        ScrollDirection::Right => (Axis::Horizontal, 1),
    };

    for _ in 0..command.amount {
        enigo
            .scroll(direction, axis)
            .map_err(|error| input_error("failed to inject scroll", error))?;
        std::thread::sleep(SCROLL_NOTCH_DELAY);
    }
    Ok(())
}
