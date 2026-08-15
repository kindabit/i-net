use serde::{Deserialize, Serialize};

use crate::business::user_database::field_type::FieldValue;

/// 节点字段值对象，节点字段在前后端之间传输的载体。
/// 字段顺序由 Vec 中的位置表达，不包含独立的排序字段。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NodeFieldVO {
    /// 字段名称。
    pub name: String,
    /// 字段类型 key（字段类型 schema 中的顶层类型）。
    pub field_type: String,
    /// 字段类型配置，无配置为 None。
    pub type_config: Option<serde_json::Value>,
    /// 字段值。
    pub value: FieldValue,
    /// 引用的字典条目 id，不引用为 None。
    pub dictionary_id: Option<String>,
}
