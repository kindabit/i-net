use serde::{Deserialize, Serialize};

use crate::business::user_database::field_type::FieldValue;

/// 节点字段变更，记录一次字段编辑中单个字段的变化。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "variant", content = "data")]
pub enum NodeFieldChange {
    /// 新增字段，包含字段名称、字段类型和当前值。
    Added {
        name: String,
        field_type: String,
        value: FieldValue,
    },
    /// 修改字段，包含字段名称、新旧字段类型和新旧值。
    Modified {
        name: String,
        old_field_type: String,
        new_field_type: String,
        old_value: FieldValue,
        new_value: FieldValue,
    },
    /// 删除字段，包含字段名称、字段类型和旧值。
    Removed {
        name: String,
        field_type: String,
        old_value: FieldValue,
    },
}

/// 日志记录的行为枚举，详细定义每一种日志事件及其数据。
/// serde 序列化为 { variant, data } 结构（与 ErrorCode 相同）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "variant", content = "data")]
pub enum Action {
    /// 对画布宇宙中的画布节点执行自动布局，包含实际位移的画布数量。
    AutoLayoutCanvasNodes { canvas_count: i64 },
    /// 对画布内节点执行自动布局，包含实际位移的节点数量。
    AutoLayoutDataNodes { node_count: i64 },
    /// 创建画布，包含画布名称。
    CanvasCreate { name: String },
    /// 移动画布，包含画布名称、旧坐标和新坐标。
    CanvasMove {
        name: String,
        old_x: f64,
        old_y: f64,
        new_x: f64,
        new_y: f64,
    },
    /// 逻辑删除画布，包含画布名称。
    CanvasLogicalDelete { name: String },
    /// 恢复被逻辑删除的画布，包含画布名称、旧坐标和新坐标。
    CanvasRestore {
        name: String,
        old_x: f64,
        old_y: f64,
        new_x: f64,
        new_y: f64,
    },
    /// 物理删除画布，包含画布名称。
    CanvasPhysicalDelete { name: String },
    /// 重命名画布，包含旧名称和新名称。
    CanvasRename { old_name: String, new_name: String },
    /// 创建节点，包含节点标题和副标题。
    NodeCreate { title: String, sub_title: String },
    /// 移动节点，包含节点标题、旧坐标和新坐标。
    NodeMove {
        title: String,
        old_x: f64,
        old_y: f64,
        new_x: f64,
        new_y: f64,
    },
    /// 修改节点的标题和副标题，包含旧标题、旧副标题、新标题和新副标题。
    NodeModify {
        old_title: String,
        old_sub_title: String,
        new_title: String,
        new_sub_title: String,
    },
    /// 逻辑删除节点，包含节点标题。
    NodeLogicalDelete { title: String },
    /// 恢复被逻辑删除的节点，包含节点标题、旧坐标和新坐标。
    NodeRestore {
        title: String,
        old_x: f64,
        old_y: f64,
        new_x: f64,
        new_y: f64,
    },
    /// 物理删除节点，包含节点标题。
    NodePhysicalDelete { title: String },
    /// 跨画布迁移节点，包含节点数量、源画布名称和目标画布名称。
    NodeRelocate {
        node_count: i64,
        source_canvas_name: String,
        target_canvas_name: String,
    },
    /// 修改节点的字段集合，包含节点标题和逐字段的变更列表。
    NodeFieldsModify {
        node_title: String,
        changes: Vec<NodeFieldChange>,
    },
    /// 导入附件，包含节点标题和附件文件名。
    AttachmentImport {
        node_title: String,
        file_name: String,
    },
    /// 导出附件，包含节点标题和附件文件名。
    AttachmentExport {
        node_title: String,
        file_name: String,
    },
    /// 更新附件文件内容，包含节点标题和附件文件名。
    AttachmentUpdate {
        node_title: String,
        file_name: String,
    },
    /// 逻辑删除附件，包含节点标题和附件文件名。
    AttachmentLogicalDelete {
        node_title: String,
        file_name: String,
    },
    /// 恢复被逻辑删除的附件，包含节点标题和附件文件名。
    AttachmentRestore {
        node_title: String,
        file_name: String,
    },
    /// 物理删除附件，包含节点标题和附件文件名。
    AttachmentPhysicalDelete {
        node_title: String,
        file_name: String,
    },
    /// 创建边，包含源节点标题和目标节点标题。
    EdgeCreate {
        source_title: String,
        target_title: String,
    },
    /// 物理删除边，包含源节点标题和目标节点标题。
    EdgePhysicalDelete {
        source_title: String,
        target_title: String,
    },
    /// 更新边的标题和详情，包含源节点标题、目标节点标题、旧标题、旧详情、新标题和新详情。
    EdgeUpdate {
        source_title: String,
        target_title: String,
        old_title: String,
        old_description: String,
        new_title: String,
        new_description: String,
    },
    /// 设置字典条目（全量覆盖），包含条目数量。
    DictionarySet { entry_count: i64 },
    /// 创建模板，包含模板名称。
    TemplateCreate { name: String },
    /// 从节点的字段结构创建模板，包含模板名称和节点标题。
    TemplateCreateFromNode { template_name: String, node_title: String },
    /// 物理删除模板，包含模板名称。
    TemplateDelete { name: String },
    /// 设置模板的字段集合，包含模板名称和字段名称列表。
    TemplateFieldsSet { template_name: String, field_names: Vec<String> },
    /// 重命名模板，包含旧名称和新名称。
    TemplateRename { old_name: String, new_name: String },
}

/// 日志实体类，记录创建对象和修改对象的行为，提供一定程度的追溯能力。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Log {
    /// 日志 id（uuid），主键。
    pub id: String,
    /// 被操作对象的 id。
    pub object_id: String,
    /// 行为，存储 Action 的 variant 名。
    pub action: String,
    /// 时间，毫秒时间戳。
    pub time: i64,
    /// 加密的行为数据（Action 序列化后的 data JSON 密文）。
    pub detail: Vec<u8>,
}
