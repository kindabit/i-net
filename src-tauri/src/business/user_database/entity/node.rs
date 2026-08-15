use serde::{Deserialize, Serialize};

/// 节点实体类，节点位于某个画布内。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    /// 节点 id（uuid），主键。
    pub id: String,
    /// 节点所在画布的 id（uuid）。
    pub canvas_id: String,
    /// 节点在画布中的位置（x 坐标）。
    pub x: f64,
    /// 节点在画布中的位置（y 坐标）。
    pub y: f64,
    /// 节点的标题。
    pub title: String,
    /// 节点的副标题。
    pub sub_title: String,
    /// 节点引用的子画布 id，仅画布节点有值，普通数据节点为 None。
    pub canvas_ref_id: Option<String>,
    /// 是否逻辑删除。
    pub deleted: bool,
    /// 存储前端序列化的自定义颜色，空串表示使用默认色，后端不理解其内容。
    pub color: String,
}
