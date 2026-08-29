use serde::{Deserialize, Serialize};

use crate::business::user_database::entity::Node;

/// 批量移动节点时单个条目的值对象。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MoveNodeVO {
    /// 节点 id。
    pub id: String,
    /// 新 x 坐标。
    pub x: f64,
    /// 新 y 坐标。
    pub y: f64,
}

/// 影子节点的方向。
/// Inflow：入向影子，普通节点的影子，产生边的源端不是画布节点，在画布内只能有出度（只能作为源）；
/// Outflow：出向影子，画布节点的影子，产生边的源端是画布节点，在画布内只能有入度（只能作为目标）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ShadowDirection {
    Inflow,
    Outflow,
}

/// 节点列表（node_list）的返回项：在 Node 基础上附带影子节点的展示信息。
/// 影子节点的 title / sub_title / color 沿产生边链合并自根本体节点；canvas_ref_id 恒为 None
/// （影子节点本身不能是画布节点；出向影子的根本体是画布节点，其 canvas_ref_id 不合并给影子，
/// 改由 shadow_origin_canvas_ref_id 单独携带）；普通节点的四个扩展字段均为 None。
#[derive(Debug, Clone, Serialize)]
pub struct NodeVO {
    /// 节点本体（serde flatten 展开）。
    #[serde(flatten)]
    pub node: Node,
    /// 影子节点根本体节点的 id，仅影子节点有值；供前端跳转定位。
    pub shadow_origin_id: Option<String>,
    /// 影子节点的原始节点是否已被逻辑删除；仅影子节点有值。
    pub shadow_origin_deleted: Option<bool>,
    /// 影子节点的方向；仅影子节点有值。
    pub shadow_direction: Option<ShadowDirection>,
    /// 影子节点根本体（画布节点）引用的子画布 id，仅出向影子有值；供前端双击影子节点时跳转定位。
    /// 普通节点与入向影子为 None。
    pub shadow_origin_canvas_ref_id: Option<String>,
}

impl std::ops::Deref for NodeVO {
    type Target = Node;
    /// 解引用到节点本体，使调用方可以直接访问 Node 的字段（如 n.id、n.title）。
    fn deref(&self) -> &Node {
        &self.node
    }
}
