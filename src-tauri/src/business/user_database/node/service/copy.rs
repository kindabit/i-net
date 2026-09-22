use crate::business::user_database::entity::{Action, Node};
use crate::business::user_database::entity::NodeField;
use crate::business::user_database::node::dao;
use crate::business::user_database::{canvas, log, node_field, state};
use crate::error_code::ErrorCode;

/// 在指定画布内的指定位置创建指定节点的副本。
///
/// 目标画布仅校验存在，不过滤逻辑删除的画布。
/// 副本继承源节点的标题、副标题、颜色和字段结构（value 为 None），
/// 不复制附件和边；副本始终是数据节点（canvas_ref_id 与 shadow_producing_edge_id 均为 None）。
/// 影子节点与画布数据节点不允许复制。
///
/// 产生 NodeCreate 日志，载荷为副本节点的标题和副标题。
///
/// # 参数
/// - `id`: 被复制的节点 id。
/// - `canvas_id`: 副本归属的目标画布 id。
/// - `x`: 副本节点在画布中的 x 坐标。
/// - `y`: 副本节点在画布中的 y 坐标。
///
/// # 返回值
/// 返回新建的副本节点；目标画布不存在时返回 `ErrorCode::NoCanvasWithSuchId`，
/// 源节点不存在或已逻辑删除时返回 `ErrorCode::NoNodeWithSuchId`，
/// 源节点是影子节点时返回 `ErrorCode::NodeIsShadow`，
/// 源节点是画布数据节点时返回 `ErrorCode::NodeIsCanvasDataNode`，
/// 发生其他错误时返回对应的 `ErrorCode`。
pub fn copy(id: &str, canvas_id: &str, x: f64, y: f64) -> Result<Node, ErrorCode> {
    let connection = state::lock_connection();

    canvas::dao::select_by_id(&connection, canvas_id)?
        .ok_or_else(|| ErrorCode::NoCanvasWithSuchId {
            id: canvas_id.to_string(),
        })?;

    let source = dao::select_by_id(&connection, id)?
        .filter(|n| !n.deleted)
        .ok_or_else(|| ErrorCode::NoNodeWithSuchId { id: id.to_string() })?;
    // 影子节点不允许此操作（展示数据从本体节点拉取，生命周期由边管理）。
    if source.shadow_producing_edge_id.is_some() {
        return Err(ErrorCode::NodeIsShadow);
    }
    // 画布数据节点不允许复制（复制子画布引用的语义不明确，避免产生空子画布）。
    if source.canvas_ref_id.is_some() {
        return Err(ErrorCode::NodeIsCanvasDataNode);
    }

    let node = Node {
        id: uuid::Uuid::new_v4().to_string(),
        canvas_id: canvas_id.to_string(),
        x,
        y,
        title: source.title.clone(),
        subtitle: source.subtitle.clone(),
        canvas_ref_id: None,
        deleted: false,
        color: source.color.clone(),
        shadow_producing_edge_id: None,
    };
    dao::insert(&connection, &node)?;

    let source_fields = node_field::dao::select_by_node_id(&connection, &source.id)?;
    for source_field in source_fields {
        let node_field = NodeField {
            node_id: node.id.clone(),
            name: source_field.name,
            field_type: source_field.field_type,
            value: None,
            sort_order: source_field.sort_order,
            dictionary_id: source_field.dictionary_id,
        };
        node_field::dao::insert(&connection, &node_field)?;
    }

    log::service::create(Action::NodeCreate {
        title: node.title.clone(),
        subtitle: node.subtitle.clone(),
    })?;
    Ok(node)
}
