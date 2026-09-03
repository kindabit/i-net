use serde::{Deserialize, Serialize};

/// 节点字段实体类，节点字段是节点的具名数据项。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeField {
    /// 字段所属节点的 id。
    pub node_id: String,
    /// 字段名称，与节点 id 构成联合主键。
    pub name: String,
    /// 字段类型 key。后端不校验其合法性，仅作为不透明标签存取。
    pub field_type: String,
    /// 加密后的字段值（明文为字段值字符串，格式由前端定义，后端不解析其内容），无值为 None。
    pub field_value: Option<Vec<u8>>,
    /// 字段在节点内的排序序号。
    pub order: i64,
    /// 引用的字典条目 id，不引用为 None。
    pub dictionary_id: Option<String>,
}
