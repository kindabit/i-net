use serde::Serialize;

/// 画布颜色历史条目，记录用户曾经使用过的画布名称与颜色组合。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CanvasColorEntry {
    /// 画布名称
    pub name: String,
    /// 父画布的 id，根画布为 null（前端据此本地化根画布显示名）
    pub parent_id: Option<String>,
    /// 前端序列化的自定义颜色
    pub color: String,
}
