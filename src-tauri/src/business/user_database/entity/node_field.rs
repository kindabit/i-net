use serde::{Deserialize, Serialize};

/// 节点字段实体类，节点字段是节点的具名数据项。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeField {
    /// 字段所属节点的 id。
    pub node_id: String,
    /// 字段名称，与节点 id 构成联合主键。
    pub name: String,
    /// 字段类型 key（字段类型 schema 中的顶层类型）。
    pub field_type: String,
    /// 字段类型配置（JSON 文本，如 {"precision":"day"}），无配置为 None。
    pub type_config: Option<String>,
    /// 加密后的字段值，无值为 None。
    pub field_value: Option<Vec<u8>>,
    /// 字段在节点内的排序序号。
    pub order: i64,
    /// 引用的字典条目 id，不引用为 None。
    pub dictionary_id: Option<String>,
}
