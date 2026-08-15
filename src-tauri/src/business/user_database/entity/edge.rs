use serde::{Deserialize, Serialize};

/// 边实体类，边连接同一画布内的两个节点，
/// 源节点 id 和目标节点 id 构成联合唯一键。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    /// 边 id（uuid），主键。
    pub id: String,
    /// 边所在画布的 id（uuid）。
    pub canvas_id: String,
    /// 源节点 id（uuid）。
    pub source_id: String,
    /// 源节点连接桩（"top" / "right" / "bottom" / "left"）。
    pub source_port: String,
    /// 目标节点 id（uuid）。
    pub target_id: String,
    /// 目标节点连接桩（"top" / "right" / "bottom" / "left"）。
    pub target_port: String,
    /// 边的标题，始终显示在边上。
    pub title: String,
    /// 边的详情，鼠标悬浮时显示。
    pub description: String,
}
