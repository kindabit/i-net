use std::collections::HashMap;
use std::str::FromStr;
use std::sync::OnceLock;

use bigdecimal::BigDecimal;
use regex::Regex;

use crate::error_code::ErrorCode;

use super::field_value::FieldValue;
use super::schema::{self, FieldTypeDef};

/// 预编译的正则校验器缓存，key 为字段类型 key。
static VALIDATION_REGEXES: OnceLock<HashMap<&'static str, Regex>> = OnceLock::new();

/// 初始化并返回预编译的正则校验器缓存。
fn validation_regexes() -> &'static HashMap<&'static str, Regex> {
    VALIDATION_REGEXES.get_or_init(|| {
        let mut map = HashMap::new();
        for ft in &schema::schema().field_types {
            if let Some(vd) = &ft.validation {
                let re = Regex::new(&vd.regex).unwrap_or_else(|e| {
                    panic!(
                        "Failed to compile validation regex for field type '{}': {}",
                        ft.key, e
                    )
                });
                map.insert(ft.key.as_str(), re);
            }
        }
        map
    })
}

/// 按 key 获取字段类型定义，不存在时返回 `ErrorCode::InvalidNodeFieldType`。
pub fn field_type_def(key: &str) -> Result<&'static FieldTypeDef, ErrorCode> {
    schema::schema()
        .field_types
        .iter()
        .find(|ft| ft.key == key)
        .ok_or_else(|| ErrorCode::InvalidNodeFieldType {
            field_type: key.to_string(),
        })
}

/// 判断字段类型是否存在。
#[cfg(test)]
pub fn field_type_exists(key: &str) -> bool {
    field_type_def(key).is_ok()
}

/// 获取字段类型的底层数据类型 key，类型不存在时返回 `ErrorCode::InvalidNodeFieldType`。
pub fn value_kind_of(key: &str) -> Result<&'static str, ErrorCode> {
    Ok(field_type_def(key)?.value_kind.as_str())
}

/// 校验字段类型配置（type_config JSON）是否符合该类型的 typeConfig 声明。
/// `def` 为调用方通过 `field_type_def` 取得的字段类型定义。
pub fn validate_type_config(
    def: &FieldTypeDef,
    type_config: &Option<serde_json::Value>,
) -> Result<(), ErrorCode> {
    let config = match type_config {
        None => return Ok(()),
        Some(c) => c,
    };

    let type_config_def = def.type_config.as_ref().ok_or_else(|| {
        ErrorCode::InvalidNodeFieldTypeConfig {
            field_type: def.key.clone(),
            detail: "this field type has no typeConfig declaration".to_string(),
        }
    })?;

    let obj = config.as_object().ok_or_else(|| ErrorCode::InvalidNodeFieldTypeConfig {
        field_type: def.key.clone(),
        detail: "type_config must be a JSON object".to_string(),
    })?;

    for (key, value) in obj {
        match key.as_str() {
            "precision" => {
                let precision_def = type_config_def.precision.as_ref().ok_or_else(|| {
                    ErrorCode::InvalidNodeFieldTypeConfig {
                        field_type: def.key.clone(),
                        detail: format!("unknown type_config key: {key}"),
                    }
                })?;
                let str_val = value.as_str().ok_or_else(|| {
                    ErrorCode::InvalidNodeFieldTypeConfig {
                        field_type: def.key.clone(),
                        detail: format!("type_config value for '{key}' must be a string"),
                    }
                })?;
                if !precision_def.options.contains(&str_val.to_string()) {
                    return Err(ErrorCode::InvalidNodeFieldTypeConfig {
                        field_type: def.key.clone(),
                        detail: format!(
                            "type_config precision value '{str_val}' is not in the allowed options"
                        ),
                    });
                }
            }
            _ => {
                return Err(ErrorCode::InvalidNodeFieldTypeConfig {
                    field_type: def.key.clone(),
                    detail: format!("unknown type_config key: {key}"),
                });
            }
        }
    }

    Ok(())
}

/// 校验字段值与字段类型的匹配性：底层数据类型匹配、
/// 十进制实数可解析、内置正则校验、时间区间起点不大于终点。
/// `def` 为调用方通过 `field_type_def` 取得的字段类型定义。
pub fn validate_field_value(
    def: &FieldTypeDef,
    name: &str,
    value: &FieldValue,
) -> Result<(), ErrorCode> {
    let actual = value.value_kind();
    let expected = def.value_kind.as_str();
    if actual != expected {
        return Err(ErrorCode::NodeFieldValueKindMismatch {
            field_type: def.key.clone(),
            expected: expected.to_string(),
            actual: actual.to_string(),
        });
    }

    match value {
        FieldValue::String(None) | FieldValue::Decimal(None) | FieldValue::Instant(None) | FieldValue::InstantRange(None) => {
            return Ok(());
        }
        _ => {}
    }

    match value {
        FieldValue::Decimal(Some(s)) => {
            BigDecimal::from_str(s).map_err(|_| {
                ErrorCode::NodeFieldValueValidationFailed {
                    name: name.to_string(),
                }
            })?;
        }
        FieldValue::InstantRange(Some((start, end))) => {
            if start > end {
                return Err(ErrorCode::NodeFieldValueValidationFailed {
                    name: name.to_string(),
                });
            }
        }
        _ => {}
    }

    if let Some(re) = validation_regexes().get(def.key.as_str()) {
        match value {
            FieldValue::String(Some(s)) => {
                if !re.is_match(s) {
                    return Err(ErrorCode::NodeFieldValueValidationFailed {
                        name: name.to_string(),
                    });
                }
            }
            FieldValue::Decimal(Some(s)) => {
                if !re.is_match(s) {
                    return Err(ErrorCode::NodeFieldValueValidationFailed {
                        name: name.to_string(),
                    });
                }
            }
            _ => {}
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 覆盖 catalog 模块全部功能：
    /// field_type_def / field_type_exists / value_kind_of 的存在与不存在路径，
    /// validate_type_config 的合法/非法配置，validate_field_value 的各变体成功与失败路径。
    #[test]
    fn test_field_type_catalog() {
        // == field_type_def 成功路径：存在的类型返回定义 ==
        assert!(field_type_def("Email").is_ok());
        assert!(field_type_def("TextSingleLine").is_ok());
        assert!(field_type_def("Date").is_ok());
        assert!(field_type_def("DateRange").is_ok());

        // == field_type_def 失败路径：不存在的类型报 InvalidNodeFieldType ==
        assert!(matches!(
            field_type_def("NoSuchType"),
            Err(ErrorCode::InvalidNodeFieldType { .. })
        ));

        // == field_type_exists：存在的类型返回 true，不存在返回 false ==
        assert!(field_type_exists("Email"));
        assert!(!field_type_exists("NoSuchType"));

        // == value_kind_of 成功路径：返回正确的底层数据类型 ==
        assert_eq!(value_kind_of("Email").unwrap(), "string");
        assert_eq!(value_kind_of("Number").unwrap(), "decimal");
        assert_eq!(value_kind_of("Date").unwrap(), "instant");
        assert_eq!(value_kind_of("DateRange").unwrap(), "instantRange");

        // == value_kind_of 失败路径：不存在的类型报 InvalidNodeFieldType ==
        assert!(matches!(
            value_kind_of("NoSuchType"),
            Err(ErrorCode::InvalidNodeFieldType { .. })
        ));

        let email_def = field_type_def("Email").unwrap();
        let number_def = field_type_def("Number").unwrap();
        let date_def = field_type_def("Date").unwrap();
        let daterange_def = field_type_def("DateRange").unwrap();

        // == validate_type_config 成功路径：Date + 合法 precision 配置 ==
        assert!(
            validate_type_config(date_def, &Some(serde_json::json!({"precision": "month"}))).is_ok()
        );

        // == validate_type_config 成功路径：Date + None 配置 ==
        assert!(validate_type_config(date_def, &None).is_ok());

        // == validate_type_config 失败路径：precision 值不在 options 内 ==
        assert!(matches!(
            validate_type_config(date_def, &Some(serde_json::json!({"precision": "week"}))),
            Err(ErrorCode::InvalidNodeFieldTypeConfig { .. })
        ));

        // == validate_type_config 失败路径：未知 type_config key ==
        assert!(matches!(
            validate_type_config(date_def, &Some(serde_json::json!({"foo": "day"}))),
            Err(ErrorCode::InvalidNodeFieldTypeConfig { .. })
        ));

        // == validate_type_config 失败路径：配置不是 JSON object ==
        assert!(matches!(
            validate_type_config(date_def, &Some(serde_json::json!("not-an-object"))),
            Err(ErrorCode::InvalidNodeFieldTypeConfig { .. })
        ));

        // == validate_type_config 失败路径：Email 无 typeConfig 声明却传入配置 ==
        assert!(matches!(
            validate_type_config(email_def, &Some(serde_json::json!({"precision": "month"}))),
            Err(ErrorCode::InvalidNodeFieldTypeConfig { .. })
        ));

        // == validate_field_value 成功路径：合法 Email 值 ==
        assert!(
            validate_field_value(email_def, "test", &FieldValue::String(Some("a@b.com".to_string())))
                .is_ok()
        );

        // == validate_field_value 成功路径：合法 Number 值（普通小数/100 位大整数/科学记数法/负小数）==
        assert!(validate_field_value(
            number_def,
            "test",
            &FieldValue::Decimal(Some("123.456".to_string()))
        )
        .is_ok());
        assert!(validate_field_value(
            number_def,
            "test",
            &FieldValue::Decimal(Some("1".repeat(100)))
        )
        .is_ok());
        assert!(validate_field_value(
            number_def,
            "test",
            &FieldValue::Decimal(Some("1e-1000".to_string()))
        )
        .is_ok());
        assert!(validate_field_value(
            number_def,
            "test",
            &FieldValue::Decimal(Some("-0.5".to_string()))
        )
        .is_ok());

        // == validate_field_value 成功路径：任意类型 + None 值 ==
        assert!(validate_field_value(email_def, "test", &FieldValue::String(None)).is_ok());
        assert!(validate_field_value(number_def, "test", &FieldValue::Decimal(None)).is_ok());
        assert!(validate_field_value(date_def, "test", &FieldValue::Instant(None)).is_ok());
        assert!(
            validate_field_value(daterange_def, "test", &FieldValue::InstantRange(None)).is_ok()
        );

        // == validate_field_value 失败路径：非法 Email 值 ==
        assert!(matches!(
            validate_field_value(email_def, "test", &FieldValue::String(Some("not-an-email".to_string()))),
            Err(ErrorCode::NodeFieldValueValidationFailed { .. })
        ));

        // == validate_field_value 失败路径：非法 Number 值（abc / 多个小数点 / 空串）==
        assert!(matches!(
            validate_field_value(number_def, "test", &FieldValue::Decimal(Some("abc".to_string()))),
            Err(ErrorCode::NodeFieldValueValidationFailed { .. })
        ));
        assert!(matches!(
            validate_field_value(number_def, "test", &FieldValue::Decimal(Some("1.2.3".to_string()))),
            Err(ErrorCode::NodeFieldValueValidationFailed { .. })
        ));
        assert!(matches!(
            validate_field_value(number_def, "test", &FieldValue::Decimal(Some("".to_string()))),
            Err(ErrorCode::NodeFieldValueValidationFailed { .. })
        ));

        // == validate_field_value 失败路径：Date 类型 + String 值报 NodeFieldValueKindMismatch ==
        assert!(matches!(
            validate_field_value(date_def, "test", &FieldValue::String(Some("2024-01-01".to_string()))),
            Err(ErrorCode::NodeFieldValueKindMismatch { .. })
        ));

        // == validate_field_value 失败路径：DateRange 起点大于终点 ==
        assert!(matches!(
            validate_field_value(daterange_def, "test", &FieldValue::InstantRange(Some((2000, 1000)))),
            Err(ErrorCode::NodeFieldValueValidationFailed { .. })
        ));
    }
}
