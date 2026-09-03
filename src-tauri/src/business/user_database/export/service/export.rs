use std::collections::{HashMap, HashSet};
use std::path::Path;

use rusqlite::Connection;

use crate::business::user_database::canvas::dao as canvas_dao;
use crate::business::user_database::edge::dao as edge_dao;
use crate::business::user_database::entity::{Canvas, Edge, Node};
use crate::business::user_database::export::service::i18n::{text_for, ExportText};
use crate::business::user_database::node::dao as node_dao;
use crate::business::user_database::node_field;
use crate::business::user_database::state;
use crate::error_code::ErrorCode;
use crate::util::file_system_util;

/// 导出模式，决定字段及字段值的导出策略。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportMode {
    /// 不包含字段。
    ExcludeFields,
    /// 包含字段，但字段值打码。
    MaskValues,
    /// 包含字段且字段值为明文。
    IncludeValues,
}

/// 将模式字符串解析为 ExportMode 枚举。
///
/// # 参数
/// - `mode`: 模式字符串，"exclude-fields" / "mask-values" / "include-values"。
///
/// # 返回值
/// 成功时返回对应的 ExportMode；非法值时返回 `ErrorCode::InvalidExportMode`。
pub fn parse_mode(mode: &str) -> Result<ExportMode, ErrorCode> {
    match mode {
        "exclude-fields" => Ok(ExportMode::ExcludeFields),
        "mask-values" => Ok(ExportMode::MaskValues),
        "include-values" => Ok(ExportMode::IncludeValues),
        other => Err(ErrorCode::InvalidExportMode {
            mode: other.to_string(),
        }),
    }
}

/// 将整个用户数据库导出为单个 markdown 文件。
/// 分层处理：本函数只写文件头并遍历所有未删除的画布调用 handle_canvas，
/// 节点由 handle_canvas 内的 handle_node 处理。
///
/// # 参数
/// - `mode`: 导出模式，决定字段及字段值的导出策略。
/// - `locale`: 导出语言代码（如 "zh-CN"、"en-US"），决定固定文案的语言。
/// - `target_path`: 导出目标文件路径。
///
/// # 返回值
/// 成功时返回 `Ok(())`；发生错误时返回对应的 `ErrorCode`。
pub fn export(mode: ExportMode, locale: &str, target_path: &str) -> Result<(), ErrorCode> {
    let connection = state::lock_connection();
    let metadata = state::metadata();
    let text = text_for(locale);

    let mut md = String::new();

    // 文件头。
    let now = chrono::Local::now();
    let time_str = now.format("%Y-%m-%d %H:%M:%S").to_string();
    let mode_str = match mode {
        ExportMode::ExcludeFields => text.mode_exclude_fields,
        ExportMode::MaskValues => text.mode_mask_values,
        ExportMode::IncludeValues => text.mode_include_values,
    };
    md.push_str(&format!("# {}\n\n", metadata.name));
    // 行尾两个空格是 markdown 硬换行，否则 blockquote 内连续行会被合并为同一段落。
    md.push_str(&format!("> {}{}  \n", text.export_time, time_str));
    md.push_str(&format!("> {}{}  \n", text.export_mode, mode_str));
    md.push_str(&format!("> {}\n\n", text.warning));

    // 构建画布树：父 id 到子画布列表的映射，子画布按名称排序保证输出稳定。
    let canvases = canvas_dao::select_by_deleted(&connection, false)?;
    let mut children_map: HashMap<Option<&str>, Vec<&Canvas>> = HashMap::new();
    for canvas in &canvases {
        children_map
            .entry(canvas.parent_id.as_deref())
            .or_default()
            .push(canvas);
    }
    for children in children_map.values_mut() {
        children.sort_by(|a, b| a.name.cmp(&b.name));
    }

    // 从根画布开始逐画布处理，子画布在 handle_canvas 内递归（DFS 保证子画布
    // 紧随父画布输出，但所有画布分区的标题层级相同，互不嵌套）。
    if let Some(root_canvases) = children_map.get(&None) {
        for root_canvas in root_canvases {
            handle_canvas(&mut md, &connection, root_canvas, &children_map, mode, &text)?;
        }
    }

    // 写入文件。
    file_system_util::write(Path::new(target_path), md.as_bytes())?;
    Ok(())
}

/// 处理单个画布：输出画布标题，遍历画布内每个未删除节点调用 handle_node，
/// 然后输出本画布的关系小节，最后递归处理每个子画布。
/// 所有画布分区平级：标题层级固定为 h2 / h3 / h4，不随树深度变化。
///
/// # 参数
/// - `md`: 累积中的 markdown 内容。
/// - `connection`: 用户数据库连接。
/// - `canvas`: 待处理的画布。
/// - `children_map`: 父 id 到子画布列表的映射，用于递归子画布。
/// - `mode`: 导出模式。
/// - `text`: 当前语言的固定文案集合。
///
/// # 返回值
/// 成功时返回 `Ok(())`；发生错误时返回对应的 `ErrorCode`。
fn handle_canvas(
    md: &mut String,
    connection: &Connection,
    canvas: &Canvas,
    children_map: &HashMap<Option<&str>, Vec<&Canvas>>,
    mode: ExportMode,
    text: &ExportText,
) -> Result<(), ErrorCode> {
    md.push_str(&format!("## {}{}\n\n", text.canvas, canvas.name));

    // 取出画布内未删除的节点，按标题排序。
    let mut nodes = node_dao::select_by_canvas_id_and_deleted(connection, &canvas.id, false)?;
    // 影子节点只是原始节点在子画布内的引用展示，不作为独立数据导出；
    // 与影子相连的边会因两端节点集合过滤而自然排除。
    nodes.retain(|n| n.shadow_id.is_none());
    nodes.sort_by(|a, b| a.title.cmp(&b.title));

    // 节点 id 集合与标题映射，用于边的过滤与标题解析。
    let node_id_set: HashSet<&str> = nodes.iter().map(|n| n.id.as_str()).collect();
    let node_map: HashMap<&str, &Node> = nodes.iter().map(|n| (n.id.as_str(), n)).collect();

    // 逐节点处理。
    for node in &nodes {
        handle_node(md, node, mode, text)?;
    }

    // 关系小节：只保留两端节点都在本画布未删除节点集合中的边。
    let edges = edge_dao::select_by_canvas_id(connection, &canvas.id)?;
    let edges: Vec<&Edge> = edges
        .iter()
        .filter(|e| {
            node_id_set.contains(e.source_id.as_str())
                && node_id_set.contains(e.target_id.as_str())
        })
        .collect();
    if !edges.is_empty() {
        md.push_str(&format!("#### {}\n\n", text.relationships));
        for edge in edges {
            write_edge_line(md, edge, &node_map, text);
        }
        md.push('\n');
    }

    // 递归处理子画布。
    if let Some(children) = children_map.get(&Some(canvas.id.as_str())) {
        for child in children {
            handle_canvas(md, connection, child, children_map, mode, text)?;
        }
    }

    Ok(())
}

/// 处理单个节点：输出节点标题、副标题（为空则省略）和字段表格
/// （ExcludeFields 模式不输出字段表格）。
///
/// # 参数
/// - `md`: 累积中的 markdown 内容。
/// - `node`: 待处理的节点。
/// - `mode`: 导出模式。
/// - `text`: 当前语言的固定文案集合。
///
/// # 返回值
/// 成功时返回 `Ok(())`；发生错误时返回对应的 `ErrorCode`。
fn handle_node(
    md: &mut String,
    node: &Node,
    mode: ExportMode,
    text: &ExportText,
) -> Result<(), ErrorCode> {
    md.push_str(&format!("### {}{}\n\n", text.node, node.title));

    // 副标题为空则省略。
    if !node.sub_title.is_empty() {
        md.push_str(&format!("{}{}\n\n", text.sub_title, node.sub_title));
    }

    // 字段表格（ExcludeFields 模式不输出）。
    if mode != ExportMode::ExcludeFields {
        let fields = node_field::service::get(&node.id)?;
        if !fields.is_empty() {
            md.push_str(&format!("| {} | {} |\n", text.field_name, text.field_value));
            md.push_str("| --- | --- |\n");
            for field in &fields {
                let value_str = format_field_value(&field.value);
                let display_value = match mode {
                    ExportMode::MaskValues => mask(&value_str),
                    ExportMode::IncludeValues => value_str,
                    ExportMode::ExcludeFields => unreachable!(),
                };
                md.push_str(&format!(
                    "| {} | {} |\n",
                    escape_cell(&field.name),
                    escape_cell(&display_value),
                ));
            }
            md.push('\n');
        }
    }

    Ok(())
}

/// 输出单行关系：`<源节点标题> --[<边标题>]--> <目标节点标题><分隔符><详情>`。
///
/// # 参数
/// - `md`: 累积中的 markdown 内容。
/// - `edge`: 待输出的边。
/// - `node_map`: 节点 id 到节点的映射，用于解析端点标题；详情为空时省略分隔符和详情。
/// - `text`: 当前语言的固定文案集合。
fn write_edge_line(md: &mut String, edge: &Edge, node_map: &HashMap<&str, &Node>, text: &ExportText) {
    let source_title = node_map
        .get(edge.source_id.as_str())
        .map(|n| n.title.as_str())
        .unwrap_or("");
    let target_title = node_map
        .get(edge.target_id.as_str())
        .map(|n| n.title.as_str())
        .unwrap_or("");
    if edge.description.is_empty() {
        md.push_str(&format!(
            "- {} --[{}]--> {}\n",
            source_title, edge.title, target_title,
        ));
    } else {
        md.push_str(&format!(
            "- {} --[{}]--> {}{}{}\n",
            source_title, edge.title, target_title, text.edge_desc_sep, edge.description,
        ));
    }
}

/// 将字段值格式化为可读字符串：None 显示空字符串，Some 时原样输出。
/// 字段值内容对后端不透明，此处不做任何解析。
fn format_field_value(value: &Option<String>) -> String {
    value.clone().unwrap_or_default()
}

/// 对表格单元格内容转义：`|` 转义为 `\|`，换行替换为 `<br>`。
fn escape_cell(content: &str) -> String {
    content.replace('|', "\\|").replace('\n', "<br>")
}

/// 打码函数：按字符长度规则替换为 `*`，保留原始长度（兼容中文）。
///
/// - 长度 ≤6：全部替换为等长 `*`
/// - 7–12：保留前 3 个字符，其余替换为 `*`
/// - ≥13：保留前 3 个和后 3 个字符，中间替换为 `*`
/// - 空字符串返回空字符串
pub fn mask(value: &str) -> String {
    let len = value.chars().count();
    if len == 0 {
        return String::new();
    }
    if len <= 6 {
        return "*".repeat(len);
    }
    if len <= 12 {
        let first: String = value.chars().take(3).collect();
        return format!("{}{}", first, "*".repeat(len - 3));
    }
    // len >= 13
    let first: String = value.chars().take(3).collect();
    let last: String = value.chars().skip(len - 3).collect();
    format!("{}{}{}", first, "*".repeat(len - 6), last)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::business::metadata;
    use crate::business::user_database::node_field::vo::NodeFieldVO;
    use crate::test;
    use crate::util::file_system_util;

    /// 覆盖 mask 函数的各长度边界与中文场景。
    #[test]
    fn test_mask_all_cases() {
        // 空字符串返回空字符串。
        assert_eq!(mask(""), "");
        // 长度 ≤6：全部替换为等长 *。
        assert_eq!(mask("abc"), "***");
        assert_eq!(mask("abcdef"), "******");
        // 7–12：保留前 3 个字符，其余替换为 *。
        assert_eq!(mask("abcdefg"), "abc****");
        assert_eq!(mask("abcdefghijkl"), "abc*********");
        // ≥13：保留前 3 个和后 3 个字符，中间替换为 *。
        assert_eq!(mask("abcdefghijklm"), "abc*******klm");
        assert_eq!(mask("abcdefghijklmn"), "abc********lmn");
        // 纯中文串：按 chars().count() 计长度，保留原始长度。
        assert_eq!(mask("中文测试"), "****");
        // 8 个中文字符（7–12 区间）：保留前 3，其余 5 个 *。
        assert_eq!(mask("中文测试长字符串"), "中文测*****");
        // 15 个中文字符：保留前 3 和后 3，中间 9 个 *。
        assert_eq!(
            mask("中文测试长字符串啊吧的呢吗哦嗯"),
            "中文测*********吗哦嗯"
        );
    }

    /// 覆盖 export 函数三种导出模式的成功路径与断言。
    #[test]
    fn test_export_service_all_modes() {
        let _guard = test::acquire_test_lock();
        let path = test::create_test_path();
        crate::state::set_path(path.clone());
        metadata::service::initialize().unwrap();
        let registered = metadata::service::register("export-test-db".to_string()).unwrap();

        // 打开数据库。
        crate::business::user_database::lifecycle::service::initialize(
            &registered.id,
            test::test_key(),
        )
        .unwrap();

        // 获取根画布。
        let canvases =
            crate::business::user_database::canvas::service::list(false).unwrap();
        let root_id = canvases[0].id.clone();

        // 创建子画布。
        let child = crate::business::user_database::canvas::service::create(
            &root_id,
            "child-canvas".to_string(),
        )
        .unwrap();

        // 在根画布内创建节点 A 和 B。
        let node_a = crate::business::user_database::node::service::create(
            &root_id,
            "Node A".to_string(),
            "sub-a".to_string(),
            0.0,
            0.0,
            None,
            false,
        )
        .unwrap();
        let node_b = crate::business::user_database::node::service::create(
            &root_id,
            "Node B".to_string(),
            String::new(),
            10.0,
            0.0,
            None,
            false,
        )
        .unwrap();

        // 在子画布内创建节点 C 和 D。
        let node_c = crate::business::user_database::node::service::create(
            &child.id,
            "Node C".to_string(),
            "sub-c".to_string(),
            0.0,
            0.0,
            None,
            false,
        )
        .unwrap();
        let node_d = crate::business::user_database::node::service::create(
            &child.id,
            "Node D".to_string(),
            String::new(),
            10.0,
            0.0,
            None,
            false,
        )
        .unwrap();

        // 为 Node A 设置字段：明文字段值 + None 值。
        crate::business::user_database::node_field::service::set(
            &node_a.id,
            &[
                NodeFieldVO {
                    name: "secret".to_string(),
                    field_type: "string:password".to_string(),
                    value: Some("plaintext-secret".to_string()),
                    dictionary_id: None,
                },
                NodeFieldVO {
                    name: "empty-field".to_string(),
                    field_type: "string:single-line".to_string(),
                    value: None,
                    dictionary_id: None,
                },
            ],
        )
        .unwrap();

        // 为 Node C 设置字段。
        crate::business::user_database::node_field::service::set(
            &node_c.id,
            &[NodeFieldVO {
                name: "note".to_string(),
                field_type: "string:single-line".to_string(),
                value: Some("child-note".to_string()),
                dictionary_id: None,
            }],
        )
        .unwrap();

        // 创建边 A -> B（根画布内）。
        crate::business::user_database::edge::service::create(
            &root_id,
            &node_a.id,
            "right".to_string(),
            &node_b.id,
            "left".to_string(),
            false,
        )
        .unwrap();
        // 创建边 C -> D（子画布内）。
        crate::business::user_database::edge::service::create(
            &child.id,
            &node_c.id,
            "right".to_string(),
            &node_d.id,
            "left".to_string(),
            false,
        )
        .unwrap();

        // 导出到数据目录外的临时文件。
        let export_dir = path
            .data_directory
            .parent()
            .unwrap()
            .join("export-test-outside");
        file_system_util::create_dir_all(&export_dir).unwrap();

        // == ExcludeFields 模式 ==
        let exclude_path = export_dir.join("exclude.md");
        export(
            ExportMode::ExcludeFields,
            "zh-CN",
            &exclude_path.to_string_lossy(),
        )
        .unwrap();
        assert!(file_system_util::try_exists(&exclude_path).unwrap());
        let content =
            String::from_utf8(file_system_util::read(&exclude_path).unwrap()).unwrap();
        // 不包含字段表格。
        assert!(!content.contains("| 字段名 |"));
        // 包含画布与节点标题。
        assert!(content.contains("画布：root"));
        assert!(content.contains("画布：child-canvas"));
        // 所有画布分区平级：子画布标题同样是 h2，不随树深度嵌套。
        assert!(content.contains("## 画布：root"));
        assert!(content.contains("## 画布：child-canvas"));
        assert!(content.contains("节点：Node A"));
        assert!(content.contains("节点：Node C"));
        // 包含关系。
        assert!(content.contains("Node A --[]--> Node B"));
        // 子画布的关系在子画布分区内。
        assert!(content.contains("Node C --[]--> Node D"));

        // == MaskValues 模式 ==
        let mask_path = export_dir.join("mask.md");
        export(ExportMode::MaskValues, "zh-CN", &mask_path.to_string_lossy()).unwrap();
        assert!(file_system_util::try_exists(&mask_path).unwrap());
        let content =
            String::from_utf8(file_system_util::read(&mask_path).unwrap()).unwrap();
        // 包含字段表格（仅字段名与值两列，不含字段类型）。
        assert!(content.contains("| 字段名 | 值 |"));
        assert!(!content.contains("string:password"));
        assert!(!content.contains("string:single-line"));
        // 字段值已打码：plaintext-secret（16 字符）→ 保留前 3 后 3，中间 10 个 *。
        assert!(content.contains("pla**********ret"));
        // 不含原明文。
        assert!(!content.contains("plaintext-secret"));
        // child-note（10 字符）→ 保留前 3，其余 7 个 *。
        assert!(content.contains("chi*******"));
        assert!(!content.contains("child-note"));

        // == IncludeValues 模式 ==
        let include_path = export_dir.join("include.md");
        export(
            ExportMode::IncludeValues,
            "zh-CN",
            &include_path.to_string_lossy(),
        )
        .unwrap();
        assert!(file_system_util::try_exists(&include_path).unwrap());
        let content =
            String::from_utf8(file_system_util::read(&include_path).unwrap()).unwrap();
        // 包含原明文。
        assert!(content.contains("plaintext-secret"));
        assert!(content.contains("child-note"));

        // 边出现在正确画布分区：根画布的关系小节只包含 A->B 边。
        let root_section = content
            .split("画布：root")
            .nth(1)
            .unwrap()
            .split("画布：child-canvas")
            .next()
            .unwrap();
        assert!(root_section.contains("Node A --[]--> Node B"));
        assert!(!root_section.contains("Node C --[]--> Node D"));

        // 子画布的关系小节包含 C->D 边。
        let child_section = content
            .split("画布：child-canvas")
            .nth(1)
            .unwrap();
        assert!(child_section.contains("Node C --[]--> Node D"));

        // == 英文 locale 路径 ==
        let en_path = export_dir.join("en.md");
        export(ExportMode::IncludeValues, "en-US", &en_path.to_string_lossy()).unwrap();
        let en_content =
            String::from_utf8(file_system_util::read(&en_path).unwrap()).unwrap();
        // 固定文案为英文，用户数据原样输出。
        assert!(en_content.contains("Canvas: root"));
        assert!(en_content.contains("Node: Node A"));
        assert!(en_content.contains("Subtitle: sub-a"));
        assert!(en_content.contains("| Name | Value |"));
        assert!(!en_content.contains("string:password"));
        assert!(en_content.contains("Relationships"));
        assert!(en_content.contains("plaintext-secret"));
        assert!(en_content.contains("Node A --[]--> Node B"));
        // 英文导出中不含中文固定文案。
        assert!(!en_content.contains("画布："));

        // 未识别的 locale 回退英文。
        let fallback_path = export_dir.join("fallback.md");
        export(
            ExportMode::IncludeValues,
            "fr-FR",
            &fallback_path.to_string_lossy(),
        )
        .unwrap();
        let fallback_content =
            String::from_utf8(file_system_util::read(&fallback_path).unwrap()).unwrap();
        assert!(fallback_content.contains("Canvas: root"));

        // 清理。
        let _ = std::fs::remove_dir_all(&export_dir);
        crate::business::user_database::lifecycle::service::save().unwrap();
        crate::business::user_database::lifecycle::service::close().unwrap();
        test::cleanup(&path);
    }
}
