use std::collections::HashSet;

use crate::business::user_database::entity::{Action, Canvas, Edge, Node, NodeField};
use crate::business::user_database::migration::vo::{ImportedEdgeVO, ImportedNodeVO};
use crate::business::user_database::{canvas, dictionary, edge, log, node, node_field, state};
use crate::error_code::ErrorCode;

/// 数据迁移聚合导入（KeePass 2.0）：接收前端已构造好的节点与边数据，在根画布内创建一个
/// 画布节点，其引用的新画布内批量写入全部普通节点、字段与父子边。全部写库操作聚合为一条
/// NodesImport 日志（不逐节点/逐字段/逐边产生日志，因为被导入数据库的条目可能非常多）。
///
/// 先校验后写库（两阶段）：校验阶段任何失败都不写库。
///
/// 字段类型与字段值的内容对后端不透明，此处仅校验字段名唯一性与字典引用存在性。
/// 边以节点列表下标表达父子关系，校验下标不越界且前向（source_index < target_index）：
/// 前端按深度优先先序产出节点，父节点下标恒小于子节点，强制前向即保证导入图无环。
///
/// # 参数
/// - `canvas_name`: 新画布的名称（画布节点标题与其保持一致），重名时自动追加 " 2"、" 3"…。
/// - `canvas_node_x`: 画布节点在根画布中的 x 坐标。
/// - `canvas_node_y`: 画布节点在根画布中的 y 坐标。
/// - `nodes`: 前端构造好的导入节点列表（第一个节点表示数据库本身，为树的根）。
/// - `edges`: 前端构造好的父子边列表（下标引用 `nodes`）。
///
/// # 返回值
/// 成功时返回 `Ok(新画布的 id)`（前端凭此跳转至新画布）；根画布不存在时返回
/// `ErrorCode::NoCanvasWithSuchId`，
/// 字段名重复时返回 `ErrorCode::DuplicateNodeFieldName`，
/// 字典引用悬空时返回 `ErrorCode::NoDictionaryEntryWithSuchId`，
/// 边下标无效时返回 `ErrorCode::InvalidImportedEdgeIndex`，
/// 发生其他错误时返回对应的 `ErrorCode`。
pub fn import_keepass2(
    canvas_name: &str,
    canvas_node_x: f64,
    canvas_node_y: f64,
    nodes: &[ImportedNodeVO],
    edges: &[ImportedEdgeVO],
) -> Result<String, ErrorCode> {
    let connection = state::lock_connection();

    let root =
        canvas::dao::select_root(&connection)?.ok_or_else(|| ErrorCode::NoCanvasWithSuchId {
            id: "root".to_string(),
        })?;

    // ===== 校验阶段：任何失败都不得写库 =====
    for imported in nodes {
        let mut seen = HashSet::new();
        for field in &imported.fields {
            if !seen.insert(field.name.as_str()) {
                return Err(ErrorCode::DuplicateNodeFieldName {
                    name: field.name.clone(),
                });
            }
        }
        for field in &imported.fields {
            if let Some(ref dict_id) = field.dictionary_id {
                if !dictionary::dao::exist_by_id(&connection, dict_id)? {
                    return Err(ErrorCode::NoDictionaryEntryWithSuchId {
                        id: dict_id.clone(),
                    });
                }
            }
        }
    }
    let node_count = nodes.len() as u64;
    for imported_edge in edges {
        if imported_edge.source_index >= imported_edge.target_index
            || imported_edge.target_index >= node_count
        {
            return Err(ErrorCode::InvalidImportedEdgeIndex {
                source_index: imported_edge.source_index,
                target_index: imported_edge.target_index,
            });
        }
    }

    // ===== 写库阶段 =====
    // 画布名去重：从 canvas_name 开始，重名时追加 " 2"、" 3"…（与画布节点创建的去重逻辑语义一致）。
    let mut final_name = canvas_name.to_string();
    let mut suffix = 2u32;
    while canvas::dao::select_by_name(&connection, &final_name)?.is_some() {
        final_name = format!("{canvas_name} {suffix}");
        suffix += 1;
    }

    // 用与 canvas::service::create 相同的布局算法计算新画布实体的坐标，
    // 但直接 insert 以绕过其逐条 CanvasCreate 日志。
    let all = canvas::dao::select_all(&connection)?;
    let (x, y) = canvas::service::layout(&root, &all);
    let canvas = Canvas {
        id: uuid::Uuid::new_v4().to_string(),
        parent_id: Some(root.id.clone()),
        name: final_name.clone(),
        x,
        y,
        deleted: false,
        color: String::new(),
    };
    canvas::dao::insert(&connection, &canvas)?;

    // 创建根画布内的画布节点（标题与画布名保持一致）。
    let canvas_node = Node {
        id: uuid::Uuid::new_v4().to_string(),
        canvas_id: root.id.clone(),
        x: canvas_node_x,
        y: canvas_node_y,
        title: final_name.clone(),
        sub_title: String::new(),
        canvas_ref_id: Some(canvas.id.clone()),
        deleted: false,
        color: String::new(),
        shadow_id: None,
    };
    node::dao::insert(&connection, &canvas_node)?;

    // 逐个创建普通节点并写入字段（order 为字段在数组中的索引），不逐条产生日志；
    // 按产出顺序记录节点 id，供边按下标回填端点。
    let key = state::key();
    let mut node_ids = Vec::with_capacity(nodes.len());
    for imported in nodes {
        let node = Node {
            id: uuid::Uuid::new_v4().to_string(),
            canvas_id: canvas.id.clone(),
            x: imported.x,
            y: imported.y,
            title: imported.title.clone(),
            sub_title: imported.sub_title.clone(),
            canvas_ref_id: None,
            deleted: false,
            color: String::new(),
            shadow_id: None,
        };
        node::dao::insert(&connection, &node)?;
        node_ids.push(node.id.clone());

        for (i, field) in imported.fields.iter().enumerate() {
            let field_value = match &field.value {
                Some(s) => Some(crate::security::aes::encrypt(s.as_bytes().to_vec(), key)?),
                None => None,
            };
            let node_field = NodeField {
                node_id: node.id.clone(),
                name: field.name.clone(),
                field_type: field.field_type.clone(),
                field_value,
                order: i as i64,
                dictionary_id: field.dictionary_id.clone(),
            };
            node_field::dao::insert(&connection, &node_field)?;
        }
    }

    // 批量写入父子边：树形布局父左子右，连接桩固定为 right → left，不逐条产生日志。
    // 导入的均为普通节点，不触发影子机制，直接 dao 写入。
    for imported_edge in edges {
        let new_edge = Edge {
            id: uuid::Uuid::new_v4().to_string(),
            canvas_id: canvas.id.clone(),
            source_id: node_ids[imported_edge.source_index as usize].clone(),
            source_port: "right".to_string(),
            target_id: node_ids[imported_edge.target_index as usize].clone(),
            target_port: "left".to_string(),
            title: String::new(),
            description: String::new(),
        };
        edge::dao::insert(&connection, &new_edge)?;
    }

    // 全部写库操作聚合为一条日志。
    log::service::create(Action::NodesImport {
        canvas_name: final_name,
        node_count: nodes.len() as i64,
    })?;

    Ok(canvas.id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::business::metadata;
    use crate::business::user_database::entity;
    use crate::business::user_database::lifecycle;
    use crate::business::user_database::log::service::LogFilter;
    use crate::business::user_database::node_field::vo::NodeFieldVO;
    use crate::test;

    /// 数据迁移聚合导入（service 层）：失败路径（字段名重复、悬空字典引用、非前向边与越界边，
    /// 均不写库）与成功路径（画布与画布节点创建、节点/字段/父子边写入、全部操作聚合为恰好一条
    /// 日志），以及同名画布的去重导入。
    #[test]
    fn test_import_keepass2() {
        let _guard = test::acquire_test_lock();

        // ===== 会话一：失败路径与成功路径 =====
        // 初始化测试数据目录、metadata 数据库并打开一个全新的用户数据库。
        let path = test::create_test_path();
        crate::state::set_path(path.clone());
        metadata::service::initialize().unwrap();
        let registered = metadata::service::register("migration-keepass2-test-db".to_string()).unwrap();
        lifecycle::service::initialize(&registered.id, test::test_key()).unwrap();
        let root = canvas::service::list(false).unwrap()[0].clone();

        // 构造字段的辅助闭包：dictionary_id 恒为 None（悬空引用场景再单独修改）。
        let field = |name: &str, field_type: &str, value: Option<&str>| NodeFieldVO {
            name: name.to_string(),
            field_type: field_type.to_string(),
            value: value.map(str::to_string),
            dictionary_id: None,
        };
        // 两个导入节点：第一个含 3 个字段（密码/访问链接有值、备注无值），第二个无字段。
        let nodes = vec![
            ImportedNodeVO {
                title: "User Name".to_string(),
                sub_title: "Sample Entry".to_string(),
                x: 0.0,
                y: 0.0,
                fields: vec![
                    field("密码", "string:password", Some("Password")),
                    field("访问链接", "string:url", Some("http://keepass.info/")),
                    field("备注", "string:multiple-line", None),
                ],
            },
            ImportedNodeVO {
                title: "General".to_string(),
                sub_title: String::new(),
                x: 240.0,
                y: 160.0,
                fields: Vec::new(),
            },
        ];
        // 一条父子边：节点 0（父）→ 节点 1（子）。
        let edges = vec![ImportedEdgeVO {
            source_index: 0,
            target_index: 1,
        }];

        // ===== 失败路径：字段名重复返回 DuplicateNodeFieldName，且不写库 =====
        let mut duplicated = nodes.clone();
        duplicated[0].fields.push(field("密码", "string:password", Some("dup")));
        assert!(matches!(
            import_keepass2("test", 0.0, 240.0, &duplicated, &edges),
            Err(ErrorCode::DuplicateNodeFieldName { name }) if name == "密码"
        ));
        assert_eq!(canvas::service::list(false).unwrap().len(), 1);
        assert!(node::service::list(&root.id, false).unwrap().is_empty());
        assert_eq!(log::service::list(0, 1, LogFilter::default()).unwrap().total, 0);

        // ===== 失败路径：悬空字典引用返回 NoDictionaryEntryWithSuchId，且不写库 =====
        let mut dangling = nodes.clone();
        dangling[0].fields[1].dictionary_id = Some(uuid::Uuid::new_v4().to_string());
        assert!(matches!(
            import_keepass2("test", 0.0, 240.0, &dangling, &edges),
            Err(ErrorCode::NoDictionaryEntryWithSuchId { .. })
        ));
        assert_eq!(canvas::service::list(false).unwrap().len(), 1);
        assert_eq!(log::service::list(0, 1, LogFilter::default()).unwrap().total, 0);

        // ===== 失败路径：非前向边（source_index >= target_index，含自环）与越界边
        // （target_index 超出节点数量）返回 InvalidImportedEdgeIndex，且不写库 =====
        let backward_edge = vec![ImportedEdgeVO {
            source_index: 1,
            target_index: 0,
        }];
        assert!(matches!(
            import_keepass2("test", 0.0, 240.0, &nodes, &backward_edge),
            Err(ErrorCode::InvalidImportedEdgeIndex { source_index: 1, target_index: 0 })
        ));
        let out_of_range_edge = vec![ImportedEdgeVO {
            source_index: 0,
            target_index: 2,
        }];
        assert!(matches!(
            import_keepass2("test", 0.0, 240.0, &nodes, &out_of_range_edge),
            Err(ErrorCode::InvalidImportedEdgeIndex { source_index: 0, target_index: 2 })
        ));
        assert_eq!(canvas::service::list(false).unwrap().len(), 1);
        assert_eq!(log::service::list(0, 1, LogFilter::default()).unwrap().total, 0);

        // ===== 成功路径：service 层导入，画布名 "test"，画布节点坐标 (0, 240) =====
        let imported_id =
            import_keepass2("test", 0.0, 240.0, &nodes, &edges).unwrap();

        // 画布列表新增名为 "test" 的画布，其 id 与接口返回值一致。
        let canvases = canvas::service::list(false).unwrap();
        let imported = canvases
            .iter()
            .find(|c| c.name == "test")
            .expect("canvas 'test' should exist after import")
            .clone();
        assert_eq!(imported_id, imported.id);

        // 根画布内新增引用新画布的画布节点，标题与画布名一致。
        let root_nodes = node::service::list(&root.id, false).unwrap();
        let canvas_node = root_nodes
            .iter()
            .find(|n| n.canvas_ref_id.as_deref() == Some(imported.id.as_str()))
            .expect("canvas node referencing imported canvas should exist in root canvas");
        assert_eq!(canvas_node.title, "test");
        assert_eq!(canvas_node.sub_title, "");
        assert_eq!((canvas_node.x, canvas_node.y), (0.0, 240.0));

        // 新画布内恰好 2 个节点，标题/副标题/坐标与传入一致。
        let imported_nodes = node::service::list(&imported.id, false).unwrap();
        assert_eq!(imported_nodes.len(), 2);
        let first = imported_nodes
            .iter()
            .find(|n| n.title == "User Name")
            .expect("imported node 'User Name' should exist")
            .clone();
        assert_eq!(first.sub_title, "Sample Entry");
        assert_eq!((first.x, first.y), (0.0, 0.0));
        let second = imported_nodes
            .iter()
            .find(|n| n.title == "General")
            .expect("imported node 'General' should exist")
            .clone();
        assert_eq!(second.sub_title, "");
        assert_eq!((second.x, second.y), (240.0, 160.0));

        // 字段解密读回：name/field_type/value 与传入一致，返回顺序即存储 order 与传入一致，
        // 无值字段的 value 为 None；第二个节点无字段。
        let fields = node_field::service::get(&first.id).unwrap();
        assert_eq!(fields.len(), 3);
        assert_eq!(fields[0].name, "密码");
        assert_eq!(fields[0].field_type, "string:password");
        assert_eq!(fields[0].value.as_deref(), Some("Password"));
        assert_eq!(fields[1].name, "访问链接");
        assert_eq!(fields[1].field_type, "string:url");
        assert_eq!(fields[1].value.as_deref(), Some("http://keepass.info/"));
        assert_eq!(fields[2].name, "备注");
        assert_eq!(fields[2].field_type, "string:multiple-line");
        assert_eq!(fields[2].value, None);
        assert!(fields.iter().all(|f| f.dictionary_id.is_none()));
        assert!(node_field::service::get(&second.id).unwrap().is_empty());

        // 父子边恰好 1 条：节点 0（父）→ 节点 1（子），连接桩 right → left，标题与详情为空。
        let imported_edges = edge::service::list(&imported.id).unwrap();
        assert_eq!(imported_edges.len(), 1);
        assert_eq!(imported_edges[0].source_id, first.id);
        assert_eq!(imported_edges[0].target_id, second.id);
        assert_eq!(imported_edges[0].source_port, "right");
        assert_eq!(imported_edges[0].target_port, "left");
        assert_eq!(imported_edges[0].title, "");
        assert_eq!(imported_edges[0].description, "");

        // 日志恰好 1 条且 variant 为 NodesImport，data 的 canvas_name 与 node_count 正确。
        let logs = log::service::list(0, 10, LogFilter::default()).unwrap();
        assert_eq!(logs.total, 1);
        assert_eq!(logs.items.len(), 1);
        assert!(matches!(
            &logs.items[0].action,
            entity::Action::NodesImport { canvas_name, node_count }
                if canvas_name == "test" && *node_count == 2
        ));

        // 保存并关闭数据库。
        lifecycle::service::save().unwrap();
        lifecycle::service::close().unwrap();

        // ===== 会话二：画布重名去重 =====
        let registered_2 =
            metadata::service::register("migration-keepass2-test-db-2".to_string()).unwrap();
        lifecycle::service::initialize(&registered_2.id, test::test_key()).unwrap();
        let root_2 = canvas::service::list(false).unwrap()[0].clone();

        // 先通过 canvas::service::create 建同名画布，再导入同名画布。
        canvas::service::create(&root_2.id, "test".to_string()).unwrap();
        let imported_id_2 =
            import_keepass2("test", 0.0, 240.0, &nodes, &edges).unwrap();

        // 新画布名为 "test 2"（去重逻辑与画布节点创建语义一致），画布节点标题同步为 "test 2"，
        // 日志里的 canvas_name 也是 "test 2"。
        let canvases = canvas::service::list(false).unwrap();
        let imported_2 = canvases
            .iter()
            .find(|c| c.name == "test 2")
            .expect("canvas 'test 2' should exist after dedup import")
            .clone();
        // 去重导入的返回值同样为实际新建画布的 id。
        assert_eq!(imported_id_2, imported_2.id);
        let root_nodes_2 = node::service::list(&root_2.id, false).unwrap();
        assert!(root_nodes_2
            .iter()
            .any(|n| n.title == "test 2" && n.canvas_ref_id.as_deref() == Some(imported_2.id.as_str())));
        let logs = log::service::list(0, 10, LogFilter::default()).unwrap();
        // 1 条 CanvasCreate（canvas::service::create）+ 1 条 NodesImport（聚合导入）。
        assert_eq!(logs.total, 2);
        assert!(logs.items.iter().any(|entry| matches!(
            &entry.action,
            entity::Action::NodesImport { canvas_name, node_count }
                if canvas_name == "test 2" && *node_count == 2
        )));

        // 保存并关闭数据库，清理测试数据目录。
        lifecycle::service::save().unwrap();
        lifecycle::service::close().unwrap();
        test::cleanup(&path);
    }
}
