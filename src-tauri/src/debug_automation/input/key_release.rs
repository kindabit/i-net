//! `key_release` 命令：按数组顺序依次松开键盘按键。

use enigo::{Direction, Enigo, Keyboard};

use super::{
    ensure_foreground, input_error, parse_key_names, AppState, AutomationResult, KeyReleaseCommand,
    PressedState,
};

/// 执行 `key_release` 命令：按数组顺序依次松开键盘按键。
///
/// # 参数
/// - `state`：共享应用程序状态。
/// - `enigo`：整个队列共用的原生输入句柄。
/// - `pressed`：本次请求的按下状态追踪。
/// - `command`：按键名数组，顺序即松开顺序。
///
/// # 返回值
/// 全部按键松开后返回 `Ok(())`；数组非法或原生注入失败时返回英文错误。
pub(super) fn execute(
    state: &AppState,
    enigo: &mut Enigo,
    pressed: &mut PressedState,
    command: &KeyReleaseCommand,
) -> AutomationResult<()> {
    let keys = parse_key_names(&command.keys)?;

    ensure_foreground(state)?;
    for key in keys {
        enigo
            .key(key, Direction::Release)
            .map_err(|error| input_error("failed to release a key", error))?;
        pressed.release_key(key);
    }
    Ok(())
}
