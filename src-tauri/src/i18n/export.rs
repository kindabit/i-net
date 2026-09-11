//! 导出 markdown 的固定文案：导出文件中的标签与提示语。
//!
//! 用户数据（数据库名、画布名、节点标题、字段名与值、边标题与详情）一律原样输出，
//! 不属于本组文案；本组只覆盖导出格式自带的固定文案。
//!
//! 导出语言由前端经 command 参数传入（前端充当语言 gate），经受支持语言解析后
//! 取得本组文案。

/// 导出 markdown 的固定文案集合。
pub struct ExportTexts {
    /// 文件头「导出时间」的标签（含冒号及尾随空格）。
    pub export_time: &'static str,
    /// 文件头「导出模式」的标签（含冒号及尾随空格）。
    pub export_mode: &'static str,
    /// 文件头明文警示的完整文本。
    pub warning: &'static str,
    /// 不含字段模式的名字。
    pub mode_exclude_fields: &'static str,
    /// 包含字段且字段值打码模式的名字。
    pub mode_mask_values: &'static str,
    /// 包含字段且字段值为明文模式的名字。
    pub mode_include_values: &'static str,
    /// 画布分区标题的标签（含冒号及尾随空格）。
    pub canvas: &'static str,
    /// 节点分区标题的标签（含冒号及尾随空格）。
    pub node: &'static str,
    /// 副标题行的标签（含冒号及尾随空格；与节点标题共同构成节点标题行）。
    pub sub_title: &'static str,
    /// 关系小节的标题。
    pub relationships: &'static str,
    /// 字段表格「字段名」的列表头。
    pub field_name: &'static str,
    /// 字段表格「值」的列表头。
    pub field_value: &'static str,
    /// 关系行内边详情前的分隔符（中文用全角冒号，英文用半角冒号加空格）。
    pub edge_desc_sep: &'static str,
}

/// 中文导出固定文案。
pub(crate) const CHINESE: ExportTexts = ExportTexts {
    export_time: "导出时间：",
    export_mode: "导出模式：",
    warning: "警告：本文件为明文导出，请妥善保管。",
    mode_exclude_fields: "不包含字段",
    mode_mask_values: "包含字段（字段值已打码）",
    mode_include_values: "包含字段（字段值为明文）",
    canvas: "画布：",
    node: "节点：",
    sub_title: "副标题：",
    relationships: "关系",
    field_name: "字段名",
    field_value: "值",
    edge_desc_sep: "：",
};

/// 英文导出固定文案。
pub(crate) const ENGLISH: ExportTexts = ExportTexts {
    export_time: "Export Time: ",
    export_mode: "Export Mode: ",
    warning: "Warning: this file is exported in plaintext. Keep it safe.",
    mode_exclude_fields: "Exclude fields",
    mode_mask_values: "Include fields (values masked)",
    mode_include_values: "Include fields (plaintext values)",
    canvas: "Canvas: ",
    node: "Node: ",
    sub_title: "Subtitle: ",
    relationships: "Relationships",
    field_name: "Name",
    field_value: "Value",
    edge_desc_sep: ": ",
};
