use serde::{Deserialize, Serialize};

/// 模板实体类，模板是可复用的节点字段结构。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    /// 模板 id（uuid），主键。
    pub id: String,
    /// 模板名称，唯一。
    pub name: String,
    /// 模板在列表中的排序序号。
    pub order: i64,
}
