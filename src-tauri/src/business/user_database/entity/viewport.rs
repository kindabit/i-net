use serde::{Deserialize, Serialize};

/// 画布宇宙视口的 canvas_id 特殊值，用于区分画布宇宙的视口和画布内的视口。
pub const CANVAS_UNIVERSE_VIEWPORT_ID: &str = "canvas_universe";

/// 视口实体类，记录画布宇宙或某个画布内的视口位置和缩放比例。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Viewport {
    /// 画布 id（uuid）或画布宇宙视口特殊值，主键。
    pub canvas_id: String,
    /// 视口中心的 x 坐标。
    pub x: f64,
    /// 视口中心的 y 坐标。
    pub y: f64,
    /// 当前的缩放比例。
    pub zoom: f64,
}
