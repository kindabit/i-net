use serde::Serialize;

/// 节点颜色历史条目，记录用户曾经使用过的节点标题与颜色组合。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct NodeColorEntry {
    /// 节点标题
    pub title: String,
    /// 前端序列化的自定义颜色
    pub color: String,
}
