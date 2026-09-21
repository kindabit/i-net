//! `type` 命令：输入一段 Unicode 文本。

use enigo::{Enigo, Keyboard};

use super::{
    ensure_foreground, input_error, AppState, AutomationError, AutomationResult, TypeCommand,
};

/// 执行 `type` 命令：把 Unicode 文本键入到当前活动目标。
///
/// # 参数
/// - `state`：共享应用程序状态。
/// - `enigo`：整个队列共用的原生输入句柄。
/// - `command`：要键入的文本。
///
/// # 返回值
/// 注入文本后返回 `Ok(())`；文本包含空字节或原生注入失败时返回英文错误。
pub(super) fn execute(
    state: &AppState,
    enigo: &mut Enigo,
    command: &TypeCommand,
) -> AutomationResult<()> {
    if command.text.contains('\0') {
        return Err(AutomationError::bad_request(
            "text must not contain null bytes",
        ));
    }

    ensure_foreground(state)?;
    enigo
        .text(&command.text)
        .map_err(|error| input_error("failed to inject text", error))
}
