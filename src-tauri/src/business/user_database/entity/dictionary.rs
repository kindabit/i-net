use serde::{Deserialize, Serialize};

/// 字典条目实体类，字典条目以树形组织，供文本类字段作为候选值来源。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dictionary {
    /// 字典条目 id（uuid），主键。
    pub id: String,
    /// 父条目 id，根条目为 None。
    pub parent_id: Option<String>,
    /// 条目的文本值。
    pub value: String,
    /// 条目在同级中的排序序号。
    pub order: i64,
}
