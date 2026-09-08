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
    pub sub_title: String,
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
