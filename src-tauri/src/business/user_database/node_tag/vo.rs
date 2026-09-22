use serde::Serialize;

/// 标签值对象：标签名称及其关联的未删除数据节点数量。
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct NodeTagVO {
    /// 标签名称。
    pub name: String,
    /// 携带该标签的未删除数据节点数量。
    pub node_count: i64,
}
