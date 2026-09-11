use rusqlite::types::Value as RusqliteValue;
use sea_query::Value as SeaValue;

/// 将 sea-query 的参数值列表转换为 rusqlite 的参数值列表，配合 `rusqlite::params_from_iter` 使用。
///
/// # 参数
/// - `values`: sea-query 语句构建产物（`build(SqliteQueryBuilder)` 返回的第二项）。
///
/// # 返回值
/// 与输入等长的 rusqlite 值列表；sea-query 各变体内部的 None 统一映射为 NULL。
pub fn values_to_params(values: sea_query::Values) -> Vec<RusqliteValue> {
    values.into_iter().map(convert_value).collect()
}

/// 将单个 sea-query 值转换为 rusqlite 值。仅支持本项目实际产生的变体，其余变体属编程错误，直接 panic。
fn convert_value(value: SeaValue) -> RusqliteValue {
    match value {
        SeaValue::Bool(v) => v
            .map(|b| RusqliteValue::Integer(b as i64))
            .unwrap_or(RusqliteValue::Null),
        SeaValue::TinyInt(v) => v
            .map(|x| RusqliteValue::Integer(x as i64))
            .unwrap_or(RusqliteValue::Null),
        SeaValue::SmallInt(v) => v
            .map(|x| RusqliteValue::Integer(x as i64))
            .unwrap_or(RusqliteValue::Null),
        SeaValue::Int(v) => v
            .map(|x| RusqliteValue::Integer(x as i64))
            .unwrap_or(RusqliteValue::Null),
        SeaValue::BigInt(v) => v.map(RusqliteValue::Integer).unwrap_or(RusqliteValue::Null),
        SeaValue::TinyUnsigned(v) => v
            .map(|x| RusqliteValue::Integer(x as i64))
            .unwrap_or(RusqliteValue::Null),
        SeaValue::SmallUnsigned(v) => v
            .map(|x| RusqliteValue::Integer(x as i64))
            .unwrap_or(RusqliteValue::Null),
        SeaValue::Unsigned(v) => v
            .map(|x| RusqliteValue::Integer(x as i64))
            .unwrap_or(RusqliteValue::Null),
        SeaValue::BigUnsigned(v) => v
            .map(|x| {
                RusqliteValue::Integer(
                    TryInto::<i64>::try_into(x).expect("u64 parameter out of i64 range"),
                )
            })
            .unwrap_or(RusqliteValue::Null),
        SeaValue::Float(v) => v
            .map(|x| RusqliteValue::Real(x as f64))
            .unwrap_or(RusqliteValue::Null),
        SeaValue::Double(v) => v.map(RusqliteValue::Real).unwrap_or(RusqliteValue::Null),
        SeaValue::String(v) => v.map(RusqliteValue::Text).unwrap_or(RusqliteValue::Null),
        SeaValue::Char(v) => v
            .map(|c| RusqliteValue::Text(c.to_string()))
            .unwrap_or(RusqliteValue::Null),
        SeaValue::Bytes(v) => v.map(RusqliteValue::Blob).unwrap_or(RusqliteValue::Null),
        SeaValue::Enum(_) => unreachable!("unsupported sea_query value variant: Enum"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 布尔与有符号整数变体的 Some 值统一映射为 rusqlite Integer。
    #[test]
    fn test_bool_and_signed_integer_some() {
        assert_eq!(
            convert_value(SeaValue::Bool(Some(true))),
            RusqliteValue::Integer(1)
        );
        assert_eq!(
            convert_value(SeaValue::TinyInt(Some(-1))),
            RusqliteValue::Integer(-1)
        );
        assert_eq!(
            convert_value(SeaValue::SmallInt(Some(-2))),
            RusqliteValue::Integer(-2)
        );
        assert_eq!(
            convert_value(SeaValue::Int(Some(-3))),
            RusqliteValue::Integer(-3)
        );
        assert_eq!(
            convert_value(SeaValue::BigInt(Some(-4))),
            RusqliteValue::Integer(-4)
        );
    }

    /// 无符号整数变体的 Some 值统一映射为 rusqlite Integer。
    #[test]
    fn test_unsigned_integer_some() {
        assert_eq!(
            convert_value(SeaValue::TinyUnsigned(Some(1))),
            RusqliteValue::Integer(1)
        );
        assert_eq!(
            convert_value(SeaValue::SmallUnsigned(Some(2))),
            RusqliteValue::Integer(2)
        );
        assert_eq!(
            convert_value(SeaValue::Unsigned(Some(3))),
            RusqliteValue::Integer(3)
        );
        assert_eq!(
            convert_value(SeaValue::BigUnsigned(Some(4))),
            RusqliteValue::Integer(4)
        );
    }

    /// 浮点变体的 Some 值统一映射为 rusqlite Real。
    #[test]
    fn test_float_some() {
        assert_eq!(
            convert_value(SeaValue::Float(Some(1.5))),
            RusqliteValue::Real(1.5)
        );
        assert_eq!(
            convert_value(SeaValue::Double(Some(2.5))),
            RusqliteValue::Real(2.5)
        );
    }

    /// 字符串、字符与字节变体的 Some 值分别映射为 rusqlite Text 与 Blob。
    #[test]
    fn test_text_and_blob_some() {
        assert_eq!(
            convert_value(SeaValue::String(Some("abc".to_string()))),
            RusqliteValue::Text("abc".to_string())
        );
        assert_eq!(
            convert_value(SeaValue::Char(Some('x'))),
            RusqliteValue::Text("x".to_string())
        );
        assert_eq!(
            convert_value(SeaValue::Bytes(Some(vec![1u8, 2u8]))),
            RusqliteValue::Blob(vec![1u8, 2u8])
        );
    }

    /// 所有变体的 None 值统一映射为 rusqlite Null。
    #[test]
    fn test_all_none_maps_to_null() {
        assert_eq!(convert_value(SeaValue::Bool(None)), RusqliteValue::Null);
        assert_eq!(convert_value(SeaValue::TinyInt(None)), RusqliteValue::Null);
        assert_eq!(convert_value(SeaValue::SmallInt(None)), RusqliteValue::Null);
        assert_eq!(convert_value(SeaValue::Int(None)), RusqliteValue::Null);
        assert_eq!(convert_value(SeaValue::BigInt(None)), RusqliteValue::Null);
        assert_eq!(convert_value(SeaValue::TinyUnsigned(None)), RusqliteValue::Null);
        assert_eq!(convert_value(SeaValue::SmallUnsigned(None)), RusqliteValue::Null);
        assert_eq!(convert_value(SeaValue::Unsigned(None)), RusqliteValue::Null);
        assert_eq!(convert_value(SeaValue::BigUnsigned(None)), RusqliteValue::Null);
        assert_eq!(convert_value(SeaValue::Float(None)), RusqliteValue::Null);
        assert_eq!(convert_value(SeaValue::Double(None)), RusqliteValue::Null);
        assert_eq!(convert_value(SeaValue::String(None)), RusqliteValue::Null);
        assert_eq!(convert_value(SeaValue::Char(None)), RusqliteValue::Null);
        assert_eq!(convert_value(SeaValue::Bytes(None)), RusqliteValue::Null);
    }

    /// values_to_params 输出与输入等长，且逐项映射正确。
    #[test]
    fn test_values_to_params_length_and_mapping() {
        let values = sea_query::Values(vec![
            SeaValue::BigInt(Some(1)),
            SeaValue::String(None),
            SeaValue::Bytes(Some(vec![9u8])),
        ]);
        assert_eq!(
            values_to_params(values),
            vec![
                RusqliteValue::Integer(1),
                RusqliteValue::Null,
                RusqliteValue::Blob(vec![9u8]),
            ]
        );
    }

    /// 失败路径：u64 超出 i64 表示范围时直接 panic。
    #[test]
    #[should_panic]
    fn test_big_unsigned_out_of_i64_range_panics() {
        convert_value(SeaValue::BigUnsigned(Some(u64::MAX)));
    }

    /// 失败路径：遇到本项目不会产生的 Enum 变体时直接 panic。
    #[test]
    #[should_panic]
    fn test_enum_variant_panics() {
        convert_value(SeaValue::Enum(sea_query::OptionEnum::None("".into())));
    }
}