use serde::{Deserialize, Serialize};

/// 批量移动画布时单个条目的值对象。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MoveNodeVO {
    /// 画布 id。
    pub id: String,
    /// 新 x 坐标。
    pub x: f64,
    /// 新 y 坐标。
    pub y: f64,
}
