use serde::{Deserialize, Serialize};

/// 节点字段值对象，节点字段在前后端之间传输的载体。
/// 字段顺序由 Vec 中的位置表达，不包含独立的排序字段。
/// field_type 与 value 的内容对后端不透明，后端不解析也不校验。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NodeFieldVO {
    /// 字段名称。
    pub name: String,
    /// 字段类型 key。
    pub field_type: String,
    /// 字段值字符串（格式由前端定义），无值为 None。
    pub value: Option<String>,
    /// 引用的字典条目 id，不引用为 None。
    pub dictionary_id: Option<String>,
}
