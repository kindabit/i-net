/// 导出 markdown 的固定文案集合，随导出语言切换。
///
/// 语言 gate 在前端：前端传入当前 locale，后端只做文案渲染，
/// 非法或未识别的 locale 一律回退英文。
pub struct ExportText {
    /// 文件头"导出时间"标签（含冒号及尾随空格）。
    pub export_time: &'static str,
    /// 文件头"导出模式"标签（含冒号及尾随空格）。
    pub export_mode: &'static str,
    /// 文件头明文警示完整文本。
    pub warning: &'static str,
    /// ExcludeFields 模式名。
    pub mode_exclude_fields: &'static str,
    /// MaskValues 模式名。
    pub mode_mask_values: &'static str,
    /// IncludeValues 模式名。
    pub mode_include_values: &'static str,
    /// 画布分区标题标签（含冒号及尾随空格）。
    pub canvas: &'static str,
    /// 节点分区标题标签（含冒号及尾随空格）。
    pub node: &'static str,
    /// 副标题行标签（含冒号及尾随空格）。
    pub sub_title: &'static str,
    /// 关系小节标题。
    pub relationships: &'static str,
    /// 字段表格"字段名"列表头。
    pub field_name: &'static str,
    /// 字段表格"值"列表头。
    pub field_value: &'static str,
    /// 边行内详情前的分隔符（中文用全角冒号，英文用半角冒号加空格）。
    pub edge_desc_sep: &'static str,
}

/// 按 locale 选择导出文案：以 zh 开头（不区分大小写）取中文表，其余一律取英文表。
///
/// # 参数
/// - `locale`: 前端传入的当前语言代码（如 "zh-CN"、"en-US"）。
///
/// # 返回值
/// 返回对应语言的固定文案集合。
pub fn text_for(locale: &str) -> ExportText {
    if locale.to_lowercase().starts_with("zh") {
        ExportText {
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
        }
    } else {
        ExportText {
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
        }
    }
}
