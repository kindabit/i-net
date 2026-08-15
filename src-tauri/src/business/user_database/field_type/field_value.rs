use serde::{Deserialize, Serialize};

/// 字段值，节点字段在前后端之间传输的值载体。变体集合与字段类型 schema 中的
/// 底层数据类型（valueKind）一一对应：serde 序列化后的 variant 名（camelCase）
/// 即 valueKind 的 key。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "variant", content = "data", rename_all = "camelCase")]
pub enum FieldValue {
    /// 字符串值。
    String(Option<String>),
    /// 任意精度十进制实数值，以十进制字符串传输。
    Decimal(Option<String>),
    /// 时间点值，UTC 毫秒时间戳。
    Instant(Option<i64>),
    /// 时间区间值，起点和终点均为 UTC 毫秒时间戳。
    InstantRange(Option<(i64, i64)>),
}

impl FieldValue {
    /// 返回该值对应的底层数据类型 key（与 serde variant 名一致）。
    pub fn value_kind(&self) -> &'static str {
        match self {
            FieldValue::String(_) => "string",
            FieldValue::Decimal(_) => "decimal",
            FieldValue::Instant(_) => "instant",
            FieldValue::InstantRange(_) => "instantRange",
        }
    }
}
