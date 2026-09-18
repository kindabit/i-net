use std::collections::HashSet;

use crate::business::user_database::entity::{Action, Canvas, Node, NodeField};
use crate::business::user_database::migration::vo::ImportedCanvasVO;
use crate::business::user_database::{canvas, dictionary, log, node, node_field, state};
use crate::error_code::ErrorCode;

/// 数据迁移多画布聚合导入（KeePass 2.0）：接收前端按迁移源 group 层级构造好的画布列表，
/// 在根画布下批量创建整棵画布子树——每个画布按其 parent_index 挂到父画布（None 挂到根画布），
/// 画布内批量写入数据节点（含字段）与引用直接子画布的画布数据节点。全部写库操作聚合为一条
/// CanvasesImport 日志（不逐画布/逐节点/逐字段产生日志，因为被导入数据库的条目可能非常多）。
///
/// 先校验后写库（两阶段）：校验阶段任何失败都不写库。
///
/// 字段类型与字段值的内容对后端不透明，此处仅校验字段名唯一性与字典引用存在性。
/// 画布层级以列表下标表达：parent_index 必须前向（父画布下标恒小于子画布，前端按深度优先
/// 先序产出画布），强制前向即保证画布层级无环；画布数据节点的 ref_index 必须指向自身画布的
/// 直接子画布（被引用画布的 parent_index 必须等于自身画布下标），保证画布宇宙中的层级关系
/// 与画布内的画布数据节点引用一致。
///
/// 写库分两遍：第一遍按列表顺序创建全部画布实体（下标前向保证父画布 id 可解析），
/// 第二遍写入各画布的内容（此时子画布 id 已生成，画布数据节点可回填引用）。
///
/// # 参数
/// - `canvases`: 前端构造好的导入画布列表（第一个元素为迁移源根 group 对应的画布；
///   画布名称已在 command 层预处理；宇宙坐标与画布内矩阵坐标均已由前端计算）。
///
/// # 返回值
/// 成功时返回 `Ok(根 group 画布的 id)`（前端凭此跳转至顶层新画布）；根画布不存在时返回
/// `ErrorCode::NoCanvasWithSuchId`，
/// 画布列表为空时返回 `ErrorCode::EmptyImportedCanvasList`，
/// 字段名重复时返回 `ErrorCode::DuplicateNodeFieldName`，
/// 字典引用悬空时返回 `ErrorCode::NoDictionaryWithSuchId`，
/// 父画布下标无效时返回 `ErrorCode::InvalidImportedCanvasIndex`，
/// 画布数据节点引用下标无效时返回 `ErrorCode::InvalidImportedCanvasRefIndex`，
/// 发生其他错误时返回对应的 `ErrorCode`。
pub fn import_keepass2_canvases(canvases: &[ImportedCanvasVO]) -> Result<String, ErrorCode> {
    let connection = state::lock_connection();

    let root =
        canvas::dao::select_root(&connection)?.ok_or_else(|| ErrorCode::NoCanvasWithSuchId {
            id: "root".to_string(),
        })?;

    // ===== 校验阶段：任何失败都不得写库 =====
    if canvases.is_empty() {
        return Err(ErrorCode::EmptyImportedCanvasList);
    }
    for imported_canvas in canvases {
        for imported_node in &imported_canvas.nodes {
            let mut seen = HashSet::new();
            for field in &imported_node.fields {
                if !seen.insert(field.name.as_str()) {
                    return Err(ErrorCode::DuplicateNodeFieldName {
                        name: field.name.clone(),
                    });
                }
            }
            for field in &imported_node.fields {
                if let Some(ref dict_id) = field.dictionary_id {
                    if !dictionary::dao::exist_by_id(&connection, dict_id)? {
                        return Err(ErrorCode::NoDictionaryWithSuchId {
                            id: dict_id.clone(),
                        });
                    }
                }
            }
        }
    }
    // 父画布下标必须前向（parent_index < 自身下标）：前端按深度优先先序产出画布，
    // 强制前向即保证画布层级无环，同时使第一遍建画布时父画布 id 必然已生成。
    for (index, imported_canvas) in canvases.iter().enumerate() {
        if let Some(parent_index) = imported_canvas.parent_index {
            if parent_index >= index as u64 {
                return Err(ErrorCode::InvalidImportedCanvasIndex {
                    canvas_index: index as u64,
                    parent_index,
                });
            }
        }
    }
    // 画布数据节点必须引用自身画布的直接子画布（被引用画布的 parent_index 必须指回自身画布），
    // 保证画布宇宙的层级关系与画布内的引用一致；下标越界同样在此拦截。
    for (index, imported_canvas) in canvases.iter().enumerate() {
        for canvas_node in &imported_canvas.canvas_nodes {
            let ref_index = canvas_node.ref_index as usize;
            if ref_index >= canvases.len()
                || canvases[ref_index].parent_index != Some(index as u64)
            {
                return Err(ErrorCode::InvalidImportedCanvasRefIndex {
                    canvas_index: index as u64,
                    ref_index: canvas_node.ref_index,
                });
            }
        }
    }

    // ===== 写库阶段 =====
    // 第一遍：按列表顺序创建全部画布实体。画布名去重：从传入名称开始，重名时追加 " 2"、" 3"…
    // （与单画布聚合导入的去重逻辑语义一致；画布逐个 insert，后建画布的去重能看到先建画布）。
    // 画布坐标原样采用前端算好的树形布局坐标，不使用 canvas::service::layout 的环形布局。
    let mut canvas_ids: Vec<String> = Vec::with_capacity(canvases.len());
    let mut canvas_names: Vec<String> = Vec::with_capacity(canvases.len());
    for imported_canvas in canvases {
        let mut final_name = imported_canvas.name.clone();
        let mut suffix = 2u32;
        while canvas::dao::select_by_name(&connection, &final_name)?.is_some() {
            final_name = format!("{} {suffix}", imported_canvas.name);
            suffix += 1;
        }
        let parent_id = match imported_canvas.parent_index {
            None => root.id.clone(),
            Some(parent_index) => canvas_ids[parent_index as usize].clone(),
        };
        let canvas = Canvas {
            id: uuid::Uuid::new_v4().to_string(),
            parent_id: Some(parent_id),
            name: final_name.clone(),
            x: imported_canvas.x,
            y: imported_canvas.y,
            deleted: false,
            color: String::new(),
        };
        canvas::dao::insert(&connection, &canvas)?;
        canvas_ids.push(canvas.id);
        canvas_names.push(final_name);
    }

    // 第二遍：写入各画布的内容。数据节点逐个创建并写入字段（sort_order 为字段在数组中的索引）；
    // 画布数据节点的标题取被引用画布去重后的最终名称（标题与引用画布名称保持一致），
    // 副标题为空。均不逐条产生日志，直接 dao 写入；导入的节点均无边相连，不触发影子机制。
    let key = state::key();
    let mut node_count = 0i64;
    for (index, imported_canvas) in canvases.iter().enumerate() {
        for imported_node in &imported_canvas.nodes {
            let node = Node {
                id: uuid::Uuid::new_v4().to_string(),
                canvas_id: canvas_ids[index].clone(),
                x: imported_node.x,
                y: imported_node.y,
                title: imported_node.title.clone(),
                subtitle: imported_node.subtitle.clone(),
                canvas_ref_id: None,
                deleted: false,
                color: String::new(),
                shadow_producing_edge_id: None,
            };
            node::dao::insert(&connection, &node)?;
            node_count += 1;

            for (i, field) in imported_node.fields.iter().enumerate() {
                let value = match &field.value {
                    Some(s) => Some(crate::security::aes::encrypt(s.as_bytes().to_vec(), key)?),
                    None => None,
                };
                let node_field = NodeField {
                    node_id: node.id.clone(),
                    name: field.name.clone(),
                    field_type: field.field_type.clone(),
                    value,
                    sort_order: i as i64,
                    dictionary_id: field.dictionary_id.clone(),
                };
                node_field::dao::insert(&connection, &node_field)?;
            }
        }

        for canvas_node in &imported_canvas.canvas_nodes {
            let ref_index = canvas_node.ref_index as usize;
            let node = Node {
                id: uuid::Uuid::new_v4().to_string(),
                canvas_id: canvas_ids[index].clone(),
                x: canvas_node.x,
                y: canvas_node.y,
                title: canvas_names[ref_index].clone(),
                subtitle: String::new(),
                canvas_ref_id: Some(canvas_ids[ref_index].clone()),
                deleted: false,
                color: String::new(),
                shadow_producing_edge_id: None,
            };
            node::dao::insert(&connection, &node)?;
        }
    }

    // 全部写库操作聚合为一条日志。
    log::service::create(Action::CanvasesImport {
        canvas_name: canvas_names[0].clone(),
        canvas_count: canvases.len() as i64,
        node_count,
    })?;

    Ok(canvas_ids[0].clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::business::metadata;
    use crate::business::user_database::entity;
    use crate::business::user_database::lifecycle;
    use crate::business::user_database::log::service::LogFilter;
    use crate::business::user_database::migration::vo::{ImportedCanvasNodeVO, ImportedNodeVO};
    use crate::business::user_database::node_field::vo::NodeFieldVO;
    use crate::business::user_database::edge;
    use crate::test;

    /// 构造字段的辅助闭包：dictionary_id 恒为 None（悬空引用场景再单独修改）。
    fn field(name: &str, field_type: &str, value: Option<&str>) -> NodeFieldVO {
        NodeFieldVO {
            name: name.to_string(),
            field_type: field_type.to_string(),
            value: value.map(str::to_string),
            dictionary_id: None,
        }
    }

    /// 构造无字段数据节点的辅助闭包。
    fn blank_node(x: f64, y: f64) -> ImportedNodeVO {
        ImportedNodeVO {
            title: format!("node-{x}-{y}"),
            subtitle: String::new(),
            x,
            y,
            fields: Vec::new(),
        }
    }

    /// 构造三层画布列表（根 group 画布 → 子 group 画布 → 孙 group 画布）的辅助函数：
    /// 根 group 画布含 2 个带字段的数据节点与 1 个引用子画布的画布数据节点，
    /// 子 group 画布含 1 个数据节点与 1 个引用孙画布的画布数据节点，孙 group 画布含 1 个数据节点。
    fn three_level_canvases() -> Vec<ImportedCanvasVO> {
        vec![
            ImportedCanvasVO {
                name: "root-group".to_string(),
                x: 240.0,
                y: 0.0,
                parent_index: None,
                nodes: vec![
                    ImportedNodeVO {
                        title: "User Name".to_string(),
                        subtitle: "Sample Entry".to_string(),
                        x: 0.0,
                        y: 0.0,
                        fields: vec![
                            field("密码", "string:password", Some("Password")),
                            field("访问链接", "string:url", Some("http://keepass.info/")),
                            field("备注", "string:multiple-line", None),
                        ],
                    },
                    blank_node(240.0, 0.0),
                ],
                canvas_nodes: vec![ImportedCanvasNodeVO {
                    x: 0.0,
                    y: 160.0,
                    ref_index: 1,
                }],
            },
            ImportedCanvasVO {
                name: "sub-group".to_string(),
                x: 480.0,
                y: 0.0,
                parent_index: Some(0),
                nodes: vec![blank_node(0.0, 0.0)],
                canvas_nodes: vec![ImportedCanvasNodeVO {
                    x: 0.0,
                    y: 160.0,
                    ref_index: 2,
                }],
            },
            ImportedCanvasVO {
                name: "grand-group".to_string(),
                x: 720.0,
                y: 0.0,
                parent_index: Some(1),
                nodes: vec![blank_node(0.0, 0.0)],
                canvas_nodes: Vec::new(),
            },
        ]
    }

    /// 数据迁移多画布聚合导入（service 层）：失败路径（空列表、字段名重复、悬空字典引用、
    /// 非前向/越界父画布下标、越界或指错父画布的画布数据节点引用，均不写库）与成功路径
    /// （三层画布子树的创建、画布内数据节点与字段写入、画布数据节点标题与引用画布最终名称一致、
    /// 全部操作聚合为恰好一条 CanvasesImport 日志），以及同名画布（含导入列表内部同名）的去重导入。
    #[test]
    fn test_import_keepass2_canvases() {
        let _guard = test::acquire_test_lock();

        // ===== 会话一：失败路径与成功路径 =====
        // 初始化测试数据目录、metadata 数据库并打开一个全新的用户数据库。
        let path = test::create_test_path();
        crate::state::set_path(path.clone());
        metadata::service::initialize().unwrap();
        let registered =
            metadata::service::register("migration-keepass2-canvases-test-db".to_string()).unwrap();
        lifecycle::service::initialize(&registered.id, test::test_key()).unwrap();
        let root = canvas::service::list(false).unwrap()[0].clone();

        // ===== 失败路径：空画布列表返回 EmptyImportedCanvasList，且不写库 =====
        assert!(matches!(
            import_keepass2_canvases(&[]),
            Err(ErrorCode::EmptyImportedCanvasList)
        ));
        assert_eq!(canvas::service::list(false).unwrap().len(), 1);

        // ===== 失败路径：字段名重复返回 DuplicateNodeFieldName，且不写库 =====
        let mut duplicated = three_level_canvases();
        duplicated[0].nodes[0]
            .fields
            .push(field("密码", "string:password", Some("dup")));
        assert!(matches!(
            import_keepass2_canvases(&duplicated),
            Err(ErrorCode::DuplicateNodeFieldName { name }) if name == "密码"
        ));
        assert_eq!(canvas::service::list(false).unwrap().len(), 1);
        assert_eq!(log::service::list(0, 1, LogFilter::default()).unwrap().total, 0);

        // ===== 失败路径：悬空字典引用返回 NoDictionaryWithSuchId，且不写库 =====
        let mut dangling = three_level_canvases();
        dangling[2].nodes[0].fields = vec![NodeFieldVO {
            dictionary_id: Some(uuid::Uuid::new_v4().to_string()),
            ..field("字典字段", "string:single-line", Some("value"))
        }];
        assert!(matches!(
            import_keepass2_canvases(&dangling),
            Err(ErrorCode::NoDictionaryWithSuchId { .. })
        ));
        assert_eq!(canvas::service::list(false).unwrap().len(), 1);

        // ===== 失败路径：非前向父画布下标（含自指）与越界父画布下标
        // 返回 InvalidImportedCanvasIndex，且不写库 =====
        let mut backward_parent = three_level_canvases();
        backward_parent[1].parent_index = Some(1);
        assert!(matches!(
            import_keepass2_canvases(&backward_parent),
            Err(ErrorCode::InvalidImportedCanvasIndex { canvas_index: 1, parent_index: 1 })
        ));
        let mut out_of_range_parent = three_level_canvases();
        out_of_range_parent[2].parent_index = Some(3);
        assert!(matches!(
            import_keepass2_canvases(&out_of_range_parent),
            Err(ErrorCode::InvalidImportedCanvasIndex { canvas_index: 2, parent_index: 3 })
        ));
        assert_eq!(canvas::service::list(false).unwrap().len(), 1);

        // ===== 失败路径：画布数据节点引用越界、引用非直接子画布（引用画布的 parent_index
        // 未指回自身画布）返回 InvalidImportedCanvasRefIndex，且不写库 =====
        let mut out_of_range_ref = three_level_canvases();
        out_of_range_ref[0].canvas_nodes[0].ref_index = 3;
        assert!(matches!(
            import_keepass2_canvases(&out_of_range_ref),
            Err(ErrorCode::InvalidImportedCanvasRefIndex { canvas_index: 0, ref_index: 3 })
        ));
        // 画布 0 的画布数据节点引用画布 2：画布 2 的父画布是画布 1 而非画布 0，引用无效。
        let mut wrong_parent_ref = three_level_canvases();
        wrong_parent_ref[0].canvas_nodes[0].ref_index = 2;
        assert!(matches!(
            import_keepass2_canvases(&wrong_parent_ref),
            Err(ErrorCode::InvalidImportedCanvasRefIndex { canvas_index: 0, ref_index: 2 })
        ));
        assert_eq!(canvas::service::list(false).unwrap().len(), 1);
        assert_eq!(log::service::list(0, 1, LogFilter::default()).unwrap().total, 0);

        // ===== 成功路径：三层画布子树导入 =====
        let canvases = three_level_canvases();
        let imported_root_id = import_keepass2_canvases(&canvases).unwrap();

        // 画布列表新增 3 个画布（原仅根画布），id 与名称按列表顺序一一对应。
        let all_canvases = canvas::service::list(false).unwrap();
        assert_eq!(all_canvases.len(), 4);
        let root_group = all_canvases
            .iter()
            .find(|c| c.name == "root-group")
            .expect("canvas 'root-group' should exist after import")
            .clone();
        let sub_group = all_canvases
            .iter()
            .find(|c| c.name == "sub-group")
            .expect("canvas 'sub-group' should exist after import")
            .clone();
        let grand_group = all_canvases
            .iter()
            .find(|c| c.name == "grand-group")
            .expect("canvas 'grand-group' should exist after import")
            .clone();
        // 接口返回根 group 画布的 id。
        assert_eq!(imported_root_id, root_group.id);
        // 画布层级：根 group 画布挂到根画布，子/孙 group 画布分别挂到其父 group 画布。
        assert_eq!(root_group.parent_id.as_deref(), Some(root.id.as_str()));
        assert_eq!(sub_group.parent_id.as_deref(), Some(root_group.id.as_str()));
        assert_eq!(grand_group.parent_id.as_deref(), Some(sub_group.id.as_str()));
        // 宇宙坐标原样写入（前端算好的树形布局坐标）。
        assert_eq!((root_group.x, root_group.y), (240.0, 0.0));
        assert_eq!((sub_group.x, sub_group.y), (480.0, 0.0));
        assert_eq!((grand_group.x, grand_group.y), (720.0, 0.0));

        // 根 group 画布内：2 个数据节点 + 1 个引用子 group 画布的画布数据节点。
        let root_group_nodes = node::service::list(&root_group.id, false).unwrap();
        assert_eq!(root_group_nodes.len(), 3);
        let first = root_group_nodes
            .iter()
            .find(|n| n.title == "User Name")
            .expect("imported node 'User Name' should exist")
            .clone();
        assert_eq!(first.subtitle, "Sample Entry");
        assert_eq!((first.x, first.y), (0.0, 0.0));
        assert!(first.canvas_ref_id.is_none());
        let sub_canvas_node = root_group_nodes
            .iter()
            .find(|n| n.canvas_ref_id.as_deref() == Some(sub_group.id.as_str()))
            .expect("canvas node referencing 'sub-group' should exist in root group canvas")
            .clone();
        // 画布数据节点标题与被引用画布的最终名称一致，副标题为空，坐标原样写入。
        assert_eq!(sub_canvas_node.title, "sub-group");
        assert_eq!(sub_canvas_node.subtitle, "");
        assert_eq!((sub_canvas_node.x, sub_canvas_node.y), (0.0, 160.0));

        // 子 group 画布内：1 个数据节点 + 1 个引用孙 group 画布的画布数据节点。
        let sub_group_nodes = node::service::list(&sub_group.id, false).unwrap();
        assert_eq!(sub_group_nodes.len(), 2);
        assert!(sub_group_nodes
            .iter()
            .any(|n| n.canvas_ref_id.as_deref() == Some(grand_group.id.as_str())
                && n.title == "grand-group"));

        // 孙 group 画布内：仅 1 个数据节点，无画布数据节点。
        let grand_group_nodes = node::service::list(&grand_group.id, false).unwrap();
        assert_eq!(grand_group_nodes.len(), 1);
        assert!(grand_group_nodes.iter().all(|n| n.canvas_ref_id.is_none()));

        // 多画布路线不创建任何画布内边。
        for canvas_id in [&root_group.id, &sub_group.id, &grand_group.id] {
            assert!(edge::service::list(canvas_id).unwrap().is_empty());
        }

        // 字段解密读回：name/field_type/value 与传入一致，无值字段的 value 为 None。
        let fields = node_field::service::get(&first.id).unwrap();
        assert_eq!(fields.len(), 3);
        assert_eq!(fields[0].name, "密码");
        assert_eq!(fields[0].value.as_deref(), Some("Password"));
        assert_eq!(fields[1].name, "访问链接");
        assert_eq!(fields[1].value.as_deref(), Some("http://keepass.info/"));
        assert_eq!(fields[2].name, "备注");
        assert_eq!(fields[2].value, None);

        // 日志恰好 1 条且 variant 为 CanvasesImport，data 的 canvas_name、canvas_count
        // 与 node_count（数据节点总数，不含画布数据节点）正确。
        let logs = log::service::list(0, 10, LogFilter::default()).unwrap();
        assert_eq!(logs.total, 1);
        assert!(matches!(
            &logs.items[0].action,
            entity::Action::CanvasesImport { canvas_name, canvas_count, node_count }
                if canvas_name == "root-group" && *canvas_count == 3 && *node_count == 4
        ));

        // 保存并关闭数据库。
        lifecycle::service::save().unwrap();
        lifecycle::service::close().unwrap();

        // ===== 会话二：画布重名去重（含导入列表内部同名） =====
        let registered_2 =
            metadata::service::register("migration-keepass2-canvases-test-db-2".to_string())
                .unwrap();
        lifecycle::service::initialize(&registered_2.id, test::test_key()).unwrap();
        let root_2 = canvas::service::list(false).unwrap()[0].clone();

        // 先通过 canvas::service::create 建同名画布 "root-group"，再导入：
        // 根 group 画布去重为 "root-group 2"；导入列表内部两个同名 "same-name" 画布
        // 依次去重为 "same-name" 与 "same-name 2"。
        canvas::service::create(&root_2.id, "root-group".to_string()).unwrap();
        let mut dedup_canvases = three_level_canvases();
        dedup_canvases[1].name = "same-name".to_string();
        dedup_canvases[2].name = "same-name".to_string();
        let imported_root_id_2 = import_keepass2_canvases(&dedup_canvases).unwrap();

        let all_canvases_2 = canvas::service::list(false).unwrap();
        let root_group_2 = all_canvases_2
            .iter()
            .find(|c| c.name == "root-group 2")
            .expect("canvas 'root-group 2' should exist after dedup import")
            .clone();
        assert_eq!(imported_root_id_2, root_group_2.id);
        let same_name_1 = all_canvases_2
            .iter()
            .find(|c| c.name == "same-name")
            .expect("canvas 'same-name' should exist after dedup import")
            .clone();
        let same_name_2 = all_canvases_2
            .iter()
            .find(|c| c.name == "same-name 2")
            .expect("canvas 'same-name 2' should exist after dedup import")
            .clone();
        // 导入列表内部同名的子/孙画布按创建顺序去重，层级关系保持不变。
        assert_eq!(same_name_1.parent_id.as_deref(), Some(root_group_2.id.as_str()));
        assert_eq!(same_name_2.parent_id.as_deref(), Some(same_name_1.id.as_str()));
        // 画布数据节点标题与被引用画布去重后的最终名称一致。
        let root_group_2_nodes = node::service::list(&root_group_2.id, false).unwrap();
        assert!(root_group_2_nodes
            .iter()
            .any(|n| n.canvas_ref_id.as_deref() == Some(same_name_1.id.as_str())
                && n.title == "same-name"));
        let same_name_1_nodes = node::service::list(&same_name_1.id, false).unwrap();
        assert!(same_name_1_nodes
            .iter()
            .any(|n| n.canvas_ref_id.as_deref() == Some(same_name_2.id.as_str())
                && n.title == "same-name 2"));
        // 日志中的 canvas_name 为去重后的顶层画布名称。
        let logs_2 = log::service::list(0, 10, LogFilter::default()).unwrap();
        assert!(logs_2.items.iter().any(|entry| matches!(
            &entry.action,
            entity::Action::CanvasesImport { canvas_name, canvas_count, node_count }
                if canvas_name == "root-group 2" && *canvas_count == 3 && *node_count == 4
        )));

        // 保存并关闭数据库，清理测试数据目录。
        lifecycle::service::save().unwrap();
        lifecycle::service::close().unwrap();
        test::cleanup(&path);
    }
}
