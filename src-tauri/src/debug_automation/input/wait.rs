//! `wait` 命令：等待一段时长。

use super::{wait_duration, AutomationResult, WaitCommand};

/// 执行 `wait` 命令：等待一段时长（不注入任何输入）。
///
/// # 参数
/// - `command`：时长的数值与单位。
///
/// # 返回值
/// 等待结束后返回 `Ok(())`；时长超过上限时返回英文 bad-request 错误。
pub(super) fn execute(command: &WaitCommand) -> AutomationResult<()> {
    let duration = wait_duration(command)?;
    std::thread::sleep(duration);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::*;

    #[test]
    fn wait_durations_are_converted_and_limited() {
        // 60000 毫秒与 60 秒是同一时长，都必须被接受并换算为 60 秒。
        assert_eq!(
            wait_duration(&WaitCommand {
                duration: 60000,
                unit: TimeUnit::Ms,
            })
            .unwrap(),
            Duration::from_secs(60)
        );
        assert_eq!(
            wait_duration(&WaitCommand {
                duration: 60,
                unit: TimeUnit::S,
            })
            .unwrap(),
            Duration::from_secs(60)
        );

        // 61000 毫秒与 61 秒都超过上限，必须被拒绝。
        for command in [
            WaitCommand {
                duration: 61000,
                unit: TimeUnit::Ms,
            },
            WaitCommand {
                duration: 61,
                unit: TimeUnit::S,
            },
        ] {
            let error = wait_duration(&command).unwrap_err();
            assert!(error.to_string().starts_with("BadRequest"));
            assert_eq!(error.message(), "wait must not exceed 60 seconds");
        }
    }
}
