use serde::{Deserialize, Serialize};

/// 模板字段值对象，模板字段在前后端之间传输的载体。模板字段只定义结构，不含值。
/// 字段顺序由 Vec 中的位置表达，不包含独立的排序字段。
/// field_type 对后端不透明，后端不校验其合法性。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TemplateFieldVO {
    /// 字段名称。
    pub name: String,
    /// 字段类型 key。
    pub field_type: String,
    /// 引用的字典条目 id，不引用为 None。
    pub dictionary_id: Option<String>,
}
