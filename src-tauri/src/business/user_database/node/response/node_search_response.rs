use serde::Serialize;

/// 节点全局搜索结果项。除节点自身字段外，附带其所在画布的名称。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct NodeSearchResponse {
    /// 节点 id
    pub id: String,
    /// 所属画布 id
    pub canvas_id: String,
    /// 节点在画布中的 x 坐标
    pub x: f64,
    /// 节点在画布中的 y 坐标
    pub y: f64,
    /// 节点标题
    pub title: String,
    /// 节点副标题
    pub sub_title: String,
    /// 节点引用的子画布 id，仅画布节点有值
    pub canvas_ref_id: Option<String>,
    /// 节点所在画布的名称
    pub canvas_name: String,
}
