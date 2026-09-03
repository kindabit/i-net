use serde::{Deserialize, Serialize};

/// 模板字段实体类，模板字段只定义结构，不含值。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateField {
    /// 字段所属模板的 id。
    pub template_id: String,
    /// 字段名称，与模板 id 构成联合主键。
    pub name: String,
    /// 字段类型 key。后端不校验其合法性，仅作为不透明标签存取。
    pub field_type: String,
    /// 字段在模板内的排序序号。
    pub order: i64,
    /// 引用的字典条目 id，不引用为 None。
    pub dictionary_id: Option<String>,
}
