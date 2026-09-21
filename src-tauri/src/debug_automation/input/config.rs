//! `config` 命令：临时修改本次请求中后续命令的执行配置。

use super::{
    validate_millis, AutomationError, AutomationResult, CommandConfig, ConfigCommand,
    MAX_COMMAND_INTERVAL_MS, MAX_KEY_CLICK_CROSS_TURN_MS, MAX_KEY_CLICK_IN_TURN_MS,
    MAX_KEY_CLICK_SPAN_MS, MAX_MOUSE_CLICK_SPAN_MS, MAX_MOVE_SPEED, MIN_MOVE_SPEED,
};

/// 用 `config` 命令提供的字段覆盖当前配置，未提供的字段保持原值。
///
/// # 参数
/// - `command`：本次请求中 `config` 命令携带的可选覆盖值。
///
/// # 返回值
/// 全部提供的字段均合法并完成覆盖时返回 `Ok(())`；
/// 任一字段越界时返回英文 bad-request 错误。
pub(super) fn execute(
    config: &mut CommandConfig,
    command: &ConfigCommand,
) -> AutomationResult<()> {
    if let Some(span) = command.mouse_click_span {
        config.mouse_click_span =
            validate_millis(span, MAX_MOUSE_CLICK_SPAN_MS, "mouseClickSpan")?;
    }
    if let Some(speed) = command.mouse_move_speed {
        if !speed.is_finite() || !(MIN_MOVE_SPEED..=MAX_MOVE_SPEED).contains(&speed) {
            return Err(AutomationError::bad_request(format!(
                "mouseMoveSpeed must be within {MIN_MOVE_SPEED}..={MAX_MOVE_SPEED} pixels per second"
            )));
        }
        config.mouse_move_speed = speed;
    }
    if let Some(interval) = command.key_click_in_turn_interval {
        config.key_click_in_turn_interval =
            validate_millis(interval, MAX_KEY_CLICK_IN_TURN_MS, "keyClickInTurnInterval")?;
    }
    if let Some(interval) = command.key_click_cross_turn_interval {
        config.key_click_cross_turn_interval = validate_millis(
            interval,
            MAX_KEY_CLICK_CROSS_TURN_MS,
            "keyClickCrossTurnInterval",
        )?;
    }
    if let Some(span) = command.key_click_span {
        config.key_click_span =
            validate_millis(span, MAX_KEY_CLICK_SPAN_MS, "keyClickSpan")?;
    }
    if let Some(interval) = command.command_interval {
        config.command_interval =
            validate_millis(interval, MAX_COMMAND_INTERVAL_MS, "commandInterval")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::*;

    #[test]
    fn command_config_defaults_match_the_documented_values() {
        let config = CommandConfig::default();
        // 六项配置的默认值必须与接口文档一致。
        assert_eq!(config.mouse_click_span, Duration::from_millis(50));
        assert_eq!(config.mouse_move_speed, DEFAULT_MOVE_SPEED);
        assert_eq!(config.key_click_in_turn_interval, Duration::from_millis(10));
        assert_eq!(
            config.key_click_cross_turn_interval,
            Duration::from_millis(50)
        );
        assert_eq!(config.key_click_span, Duration::from_millis(50));
        assert_eq!(config.command_interval, Duration::ZERO);
    }

    #[test]
    fn config_command_overrides_only_provided_fields() {
        let mut config = CommandConfig::default();

        // 只提供 mouseClickSpan 时，其余字段必须保持默认值。
        super::execute(
            &mut config,
            &ConfigCommand {
                mouse_click_span: Some(120),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(config.mouse_click_span, Duration::from_millis(120));
        assert_eq!(config.mouse_move_speed, DEFAULT_MOVE_SPEED);
        assert_eq!(config.key_click_span, Duration::from_millis(50));
        assert_eq!(config.command_interval, Duration::ZERO);

        // 第二次覆盖不提供 mouseClickSpan 时，上一次的覆盖值必须保留；0 是合法值。
        super::execute(
            &mut config,
            &ConfigCommand {
                mouse_move_speed: Some(2000.0),
                key_click_span: Some(0),
                command_interval: Some(30),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(config.mouse_move_speed, 2000.0);
        assert_eq!(config.key_click_span, Duration::ZERO);
        assert_eq!(config.mouse_click_span, Duration::from_millis(120));
        assert_eq!(config.command_interval, Duration::from_millis(30));
    }

    #[test]
    fn config_values_outside_the_allowed_ranges_are_rejected() {
        let mut config = CommandConfig::default();

        // 速度 0、NaN、低于下限 49 与高于上限 20001 都必须被拒绝。
        for speed in [0.0, f64::NAN, MIN_MOVE_SPEED - 1.0, MAX_MOVE_SPEED + 1.0] {
            let error = super::execute(
                &mut config,
                &ConfigCommand {
                    mouse_move_speed: Some(speed),
                    ..Default::default()
                },
            )
            .unwrap_err();
            assert_eq!(error.message(), format!("mouseMoveSpeed must be within {MIN_MOVE_SPEED}..={MAX_MOVE_SPEED} pixels per second"));
        }

        // 四个毫秒字段各自超过上限一个毫秒时必须被拒绝，并报告对应字段名。
        let rejected = [
            (
                ConfigCommand {
                    mouse_click_span: Some(MAX_MOUSE_CLICK_SPAN_MS + 1),
                    ..Default::default()
                },
                "mouseClickSpan must not exceed 5000 milliseconds",
            ),
            (
                ConfigCommand {
                    key_click_in_turn_interval: Some(MAX_KEY_CLICK_IN_TURN_MS + 1),
                    ..Default::default()
                },
                "keyClickInTurnInterval must not exceed 1000 milliseconds",
            ),
            (
                ConfigCommand {
                    key_click_cross_turn_interval: Some(MAX_KEY_CLICK_CROSS_TURN_MS + 1),
                    ..Default::default()
                },
                "keyClickCrossTurnInterval must not exceed 5000 milliseconds",
            ),
            (
                ConfigCommand {
                    key_click_span: Some(MAX_KEY_CLICK_SPAN_MS + 1),
                    ..Default::default()
                },
                "keyClickSpan must not exceed 5000 milliseconds",
            ),
            (
                ConfigCommand {
                    command_interval: Some(MAX_COMMAND_INTERVAL_MS + 1),
                    ..Default::default()
                },
                "commandInterval must not exceed 5000 milliseconds",
            ),
        ];
        for (command, expected) in rejected {
            let error = super::execute(&mut config, &command).unwrap_err();
            assert!(error.to_string().starts_with("BadRequest"));
            assert_eq!(error.message(), expected);
        }

        // 恰好等于各字段上限与下限的取值必须被接受。
        let accepted = [
            ConfigCommand {
                mouse_move_speed: Some(MIN_MOVE_SPEED),
                ..Default::default()
            },
            ConfigCommand {
                mouse_move_speed: Some(MAX_MOVE_SPEED),
                ..Default::default()
            },
            ConfigCommand {
                mouse_click_span: Some(MAX_MOUSE_CLICK_SPAN_MS),
                ..Default::default()
            },
            ConfigCommand {
                key_click_in_turn_interval: Some(MAX_KEY_CLICK_IN_TURN_MS),
                ..Default::default()
            },
            ConfigCommand {
                key_click_cross_turn_interval: Some(MAX_KEY_CLICK_CROSS_TURN_MS),
                ..Default::default()
            },
            ConfigCommand {
                key_click_span: Some(MAX_KEY_CLICK_SPAN_MS),
                ..Default::default()
            },
            ConfigCommand {
                command_interval: Some(MAX_COMMAND_INTERVAL_MS),
                ..Default::default()
            },
        ];
        for command in accepted {
            assert!(super::execute(&mut config, &command).is_ok());
        }
    }
}
