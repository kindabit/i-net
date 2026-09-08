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
    log::service::create(
        &canvas.id,
        Action::NodesImport {
            canvas_name: final_name,
            node_count: nodes.len() as i64,
        },
    )?;

    Ok(canvas.id)
}
