use crate::error_code::ErrorCode;

use super::catalog;
use super::field_value::FieldValue;

/// 将字段值加密编码为可存储的 BLOB；无值（变体内为 None）时返回 None。
pub fn encode(value: &FieldValue, key: &[u8; 32]) -> Result<Option<Vec<u8>>, ErrorCode> {
    let plaintext: Option<Vec<u8>> = match value {
        FieldValue::String(Some(s)) => Some(s.as_bytes().to_vec()),
        FieldValue::Decimal(Some(s)) => Some(s.as_bytes().to_vec()),
        FieldValue::Instant(Some(ts)) => Some(ts.to_le_bytes().to_vec()),
        FieldValue::InstantRange(Some((start, end))) => {
            let mut bytes = Vec::with_capacity(16);
            bytes.extend_from_slice(&start.to_le_bytes());
            bytes.extend_from_slice(&end.to_le_bytes());
            Some(bytes)
        }
        _ => None,
    };

    match plaintext {
        Some(data) => {
            let encrypted = crate::security::aes::encrypt(data, *key)?;
            Ok(Some(encrypted))
        }
        None => Ok(None),
    }
}

/// 按字段类型将存储的 BLOB 解密还原为字段值；blob 为 None 时返回该类型的无值变体。
/// 字段类型不存在时返回 `ErrorCode::InvalidNodeFieldType`。
pub fn decode(
    field_type: &str,
    blob: Option<Vec<u8>>,
    key: &[u8; 32],
) -> Result<FieldValue, ErrorCode> {
    let kind = catalog::value_kind_of(field_type)?;

    let blob = match blob {
        Some(b) => b,
        None => {
            return match kind {
                "string" => Ok(FieldValue::String(None)),
                "decimal" => Ok(FieldValue::Decimal(None)),
                "instant" => Ok(FieldValue::Instant(None)),
                "instantRange" => Ok(FieldValue::InstantRange(None)),
                _ => unreachable!(),
            };
        }
    };

    let plaintext = crate::security::aes::decrypt(blob, *key)?;

    match kind {
        "string" => {
            let s = String::from_utf8(plaintext).map_err(|e| {
                ErrorCode::FailToDeserializeNodeFieldValue {
                    detail: format!("Failed to decode string field: {e}"),
                }
            })?;
            Ok(FieldValue::String(Some(s)))
        }
        "decimal" => {
            let s = String::from_utf8(plaintext).map_err(|e| {
                ErrorCode::FailToDeserializeNodeFieldValue {
                    detail: format!("Failed to decode decimal field: {e}"),
                }
            })?;
            Ok(FieldValue::Decimal(Some(s)))
        }
        "instant" => {
            let bytes: [u8; 8] =
                plaintext.as_slice().try_into().map_err(|_| {
                    ErrorCode::FailToDeserializeNodeFieldValue {
                        detail: format!(
                            "Failed to decode instant: expected 8 bytes, got {}",
                            plaintext.len()
                        ),
                    }
                })?;
            let ts = i64::from_le_bytes(bytes);
            Ok(FieldValue::Instant(Some(ts)))
        }
        "instantRange" => {
            let bytes: [u8; 16] =
                plaintext.as_slice().try_into().map_err(|_| {
                    ErrorCode::FailToDeserializeNodeFieldValue {
                        detail: format!(
                            "Failed to decode instantRange: expected 16 bytes, got {}",
                            plaintext.len()
                        ),
                    }
                })?;
            let start = i64::from_le_bytes(
                bytes[..8].try_into().unwrap(),
            );
            let end = i64::from_le_bytes(
                bytes[8..].try_into().unwrap(),
            );
            Ok(FieldValue::InstantRange(Some((start, end))))
        }
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 覆盖 codec 模块全部功能：
    /// 四种变体有值/无值的加解密往返，decode 的不存在类型、篡改密文、
    /// 错误长度明文、非 UTF-8 明文四类失败路径。
    #[test]
    fn test_field_type_codec() {
        let key = crate::test::test_key();

        // == 成功路径：String 有值加解密往返一致 ==
        let value = FieldValue::String(Some("hello world".to_string()));
        let encoded = encode(&value, &key).unwrap().unwrap();
        assert_eq!(decode("TextSingleLine", Some(encoded), &key).unwrap(), value);

        // == 成功路径：Decimal 100 位大数加解密往返一致 ==
        let digits = "1".repeat(100);
        let value = FieldValue::Decimal(Some(digits.clone()));
        let encoded = encode(&value, &key).unwrap().unwrap();
        assert_eq!(
            decode("Number", Some(encoded), &key).unwrap(),
            FieldValue::Decimal(Some(digits))
        );

        // == 成功路径：Instant 有值加解密往返一致 ==
        let value = FieldValue::Instant(Some(1712345678000));
        let encoded = encode(&value, &key).unwrap().unwrap();
        assert_eq!(decode("Date", Some(encoded), &key).unwrap(), value);

        // == 成功路径：InstantRange 有值加解密往返一致 ==
        let value = FieldValue::InstantRange(Some((1000, 2000)));
        let encoded = encode(&value, &key).unwrap().unwrap();
        assert_eq!(decode("DateRange", Some(encoded), &key).unwrap(), value);

        // == 成功路径：四种变体 None 值往返（encode 返回 None，decode 返回无值变体）==
        assert!(encode(&FieldValue::String(None), &key).unwrap().is_none());
        assert_eq!(
            decode("TextSingleLine", None, &key).unwrap(),
            FieldValue::String(None)
        );
        assert!(encode(&FieldValue::Decimal(None), &key).unwrap().is_none());
        assert_eq!(
            decode("Number", None, &key).unwrap(),
            FieldValue::Decimal(None)
        );
        assert!(encode(&FieldValue::Instant(None), &key).unwrap().is_none());
        assert_eq!(
            decode("Date", None, &key).unwrap(),
            FieldValue::Instant(None)
        );
        assert!(encode(&FieldValue::InstantRange(None), &key).unwrap().is_none());
        assert_eq!(
            decode("DateRange", None, &key).unwrap(),
            FieldValue::InstantRange(None)
        );

        // == 失败路径：decode 不存在的 field_type 报 InvalidNodeFieldType ==
        assert!(matches!(
            decode("NoSuchType", None, &key),
            Err(ErrorCode::InvalidNodeFieldType { .. })
        ));

        // == 失败路径：decode 被篡改的密文（翻转末尾字节）报解密失败 ==
        let mut encoded = encode(&FieldValue::String(Some("hello".to_string())), &key)
            .unwrap()
            .unwrap();
        let last = encoded.len() - 1;
        encoded[last] ^= 0xFF;
        assert!(decode("TextSingleLine", Some(encoded), &key).is_err());

        // == 失败路径：decode 长度错误的 instant 明文报 FailToDeserializeNodeFieldValue ==
        let encrypted = crate::security::aes::encrypt(vec![1u8, 2, 3, 4], key).unwrap();
        assert!(matches!(
            decode("Date", Some(encrypted), &key),
            Err(ErrorCode::FailToDeserializeNodeFieldValue { .. })
        ));

        // == 失败路径：decode 非 UTF-8 明文报 FailToDeserializeNodeFieldValue ==
        let encrypted = crate::security::aes::encrypt(vec![0xff, 0xfe], key).unwrap();
        assert!(matches!(
            decode("TextSingleLine", Some(encrypted), &key),
            Err(ErrorCode::FailToDeserializeNodeFieldValue { .. })
        ));
    }
}
