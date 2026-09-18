use serde::{Deserialize, Serialize};

use crate::business::user_database::node_field::vo::NodeFieldVO;

/// 数据迁移导入的节点值对象：前端解密解析迁移源文件后构造好的节点数据，
/// 经聚合导入接口传入后端。节点内容（标题、副标题、坐标与字段）对后端不透明，
/// 后端仅做数据完整性校验后原样写入。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImportedNodeVO {
    /// 节点标题。
    pub title: String,
    /// 节点副标题。
    pub subtitle: String,
    /// 节点在画布中的 x 坐标（布局坐标由前端计算）。
    pub x: f64,
    /// 节点在画布中的 y 坐标（布局坐标由前端计算）。
    pub y: f64,
    /// 节点的字段列表，顺序即存储顺序。
    pub fields: Vec<NodeFieldVO>,
}

/// 数据迁移导入的边值对象：以导入节点列表的下标表达父子关系
/// （source_index 指向父节点、target_index 指向子节点），随聚合导入接口传入后端。
/// 下标恒满足 source_index < target_index（前端按深度优先先序产出节点），
/// 后端据此以 O(边数) 校验保证导入图无环。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImportedEdgeVO {
    /// 父节点在导入节点列表中的下标。
    pub source_index: u64,
    /// 子节点在导入节点列表中的下标。
    pub target_index: u64,
}

/// 数据迁移多画布导入的画布数据节点值对象：表达"在父画布中创建一个引用子画布的
/// 画布数据节点"。被引用画布以导入画布列表的下标表达（画布 id 由后端在导入时生成），
/// 且必须引用自身画布的直接子画布（被引用画布的 parent_index 必须指回自身画布），
/// 后端据此校验画布层级与画布数据节点引用的一致性。标题不随数据传入：
/// 画布数据节点的标题与其引用画布的名称始终保持一致（见概念地图·数据节点与画布数据节点），
/// 后端写入时取被引用画布去重后的最终名称。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImportedCanvasNodeVO {
    /// 画布数据节点在其画布中的 x 坐标（布局坐标由前端计算）。
    pub x: f64,
    /// 画布数据节点在其画布中的 y 坐标（布局坐标由前端计算）。
    pub y: f64,
    /// 被引用子画布在导入画布列表中的下标。
    pub ref_index: u64,
}

/// 数据迁移多画布导入的画布值对象：前端按迁移源的 group 层级构造好的画布数据，
/// 经多画布聚合导入接口传入后端。画布名称、宇宙坐标与内容对后端不透明，
/// 后端仅做数据完整性校验后原样写入。
/// 列表按深度优先先序排列（父画布下标恒小于子画布），第一个元素为迁移源根 group
/// 对应的画布（parent_index 为 None，挂到根画布）；画布在画布宇宙中的坐标由前端
/// 按 group 层级以树形布局计算（见概念地图·数据迁移）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImportedCanvasVO {
    /// 画布名称，重名时后端自动追加 " 2"、" 3"…。
    pub name: String,
    /// 画布在画布宇宙中的 x 坐标（布局坐标由前端计算）。
    pub x: f64,
    /// 画布在画布宇宙中的 y 坐标（布局坐标由前端计算）。
    pub y: f64,
    /// 父画布在导入画布列表中的下标；None 表示挂到根画布（仅根 group 对应的画布）。
    pub parent_index: Option<u64>,
    /// 画布内的数据节点列表（矩阵布局坐标由前端计算）。
    pub nodes: Vec<ImportedNodeVO>,
    /// 画布内的画布数据节点列表（引用直接子画布，矩阵布局坐标由前端计算）。
    pub canvas_nodes: Vec<ImportedCanvasNodeVO>,
}
