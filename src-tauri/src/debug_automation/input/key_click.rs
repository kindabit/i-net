//! `key_click` 命令：把整个按键序列快速点击若干轮。

use enigo::{Direction, Enigo, Keyboard};

use super::{
    ensure_foreground, ensure_unique_keys, input_error, parse_key_names, validate_click_count,
    AppState, AutomationResult, CommandConfig, KeyClickCommand, PressedState,
};

/// 执行 `key_click` 命令：把整个按键序列快速点击若干轮。
///
/// 每一轮内按键按数组顺序按下，按数组顺序以 FIFO 方式松开；
/// 轮与轮之间以及同一阶段内相邻按键之间都插入配置指定的间隔。
///
/// # 参数
/// - `state`：共享应用程序状态。
/// - `enigo`：整个队列共用的原生输入句柄。
/// - `pressed`：本次请求的按下状态追踪。
/// - `command`：按键名数组与重复轮数。
/// - `config`：本次请求的运行期配置。
///
/// # 返回值
/// 全部轮次注入后返回 `Ok(())`；数组含重复按键、轮数越界或原生注入失败时
/// 返回英文错误。
pub(super) fn execute(
    state: &AppState,
    enigo: &mut Enigo,
    pressed: &mut PressedState,
    command: &KeyClickCommand,
    config: &CommandConfig,
) -> AutomationResult<()> {
    let keys = parse_key_names(&command.keys)?;
    ensure_unique_keys(&keys)?;
    validate_click_count(command.count)?;

    ensure_foreground(state)?;
    for turn in 0..command.count {
        if turn > 0 {
            std::thread::sleep(config.key_click_cross_turn_interval);
        }

        for (index, key) in keys.iter().enumerate() {
            if index > 0 {
                std::thread::sleep(config.key_click_in_turn_interval);
            }
            enigo
                .key(*key, Direction::Press)
                .map_err(|error| input_error("failed to press a key", error))?;
            pressed.press_key(*key);
        }

        std::thread::sleep(config.key_click_span);

        for (index, key) in keys.iter().enumerate() {
            if index > 0 {
                std::thread::sleep(config.key_click_in_turn_interval);
            }
            enigo
                .key(*key, Direction::Release)
                .map_err(|error| input_error("failed to release a key", error))?;
            pressed.release_key(*key);
        }
    }
    Ok(())
}
