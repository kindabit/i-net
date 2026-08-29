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
    /// 影子节点指向产生它的边的 id；None 表示普通节点，Some 表示该节点是影子节点。
    /// 影子节点只有位置和 shadow_id 有意义，展示数据（标题、副标题、颜色等）沿产生边链从根本体节点拉取。
    /// 影子的生命周期由产生边控制：边被物理删除时，影子经该外键级联删除，下游级联随之自然坍塌。
    pub shadow_id: Option<String>,
}
