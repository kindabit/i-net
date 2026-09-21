//! `mouse_drag` 命令：从起点按下鼠标按键并沿直线拖动到终点后松开。

use enigo::{Direction, Enigo, Mouse};

use super::{
    ensure_foreground, input_error, map_button, move_pointer, move_smooth, platform,
    resolve_screen_point, AppState, AutomationResult, MouseDragCommand, PressedState,
    DRAG_JITTER_PIXELS, DRAG_SETTLE_DELAY,
};

/// 执行 `mouse_drag` 命令：从起点按下鼠标按键并沿直线拖动到终点后松开。
///
/// 光标在按下按键前先抖动，使浏览器登记到移动，然后以插值步向目标移动，
/// 并在释放前短暂停顿，这正是 HTML5 拖拽会话能够可靠启动的原因。
///
/// # 参数
/// - `state`：共享应用程序状态。
/// - `enigo`：整个队列共用的原生输入句柄。
/// - `pressed`：本次请求的按下状态追踪。
/// - `command`：起点、终点与鼠标按键。
/// - `speed`：本次移动使用的物理像素每秒速度。
///
/// # 返回值
/// 注入拖拽后返回 `Ok(())`；坐标或速度无效以及原生注入失败时返回英文错误。
pub(super) fn execute(
    state: &AppState,
    enigo: &mut Enigo,
    pressed: &mut PressedState,
    command: &MouseDragCommand,
    speed: f64,
) -> AutomationResult<()> {
    let main = platform::main_raw_window(state)?;
    let from = resolve_screen_point(&main, command.from.x, command.from.y)?;
    let to = resolve_screen_point(&main, command.to.x, command.to.y)?;

    ensure_foreground(state)?;
    let current = enigo
        .location()
        .map_err(|error| input_error("failed to query the cursor position", error))?;
    move_smooth(enigo, current, from, speed)?;

    // 先偏离起点再回到起点，使目标窗口登记到指针移动。
    move_pointer(enigo, (from.0.saturating_add(DRAG_JITTER_PIXELS), from.1))?;
    move_pointer(enigo, from)?;

    let button = map_button(command.button);
    enigo
        .button(button, Direction::Press)
        .map_err(|error| input_error("failed to press the drag button", error))?;
    pressed.press_mouse(button);

    // 移动失败时直接把错误交给外层，由外层统一释放按下的按键。
    move_smooth(enigo, from, to, speed)?;
    std::thread::sleep(DRAG_SETTLE_DELAY);
    enigo
        .button(button, Direction::Release)
        .map_err(|error| input_error("failed to release the drag button", error))?;
    pressed.release_mouse(button);
    Ok(())
}
