use serde::{Deserialize, Serialize};

/// 根画布名称常量，根画布以此名称创建，且没有父画布。
pub const ROOT_CANVAS_NAME: &str = "root";

/// 画布实体类，画布内含节点和边，所有画布通过 parent_id 构成一棵树。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Canvas {
    /// 画布 id（uuid），主键。
    pub id: String,
    /// 父画布的 id（uuid），根画布为 null。
    pub parent_id: Option<String>,
    /// 画布名称，唯一键。
    pub name: String,
    /// 画布在画布宇宙中的位置（x 坐标）。
    pub x: f64,
    /// 画布在画布宇宙中的位置（y 坐标）。
    pub y: f64,
    /// 是否逻辑删除。
    pub deleted: bool,
    /// 存储前端序列化的自定义颜色，空串表示使用默认色，后端不理解其内容。
    pub color: String,
}
