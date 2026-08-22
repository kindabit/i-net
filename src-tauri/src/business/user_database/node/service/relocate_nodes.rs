use std::collections::HashSet;

use crate::business::user_database::entity::Action;
use crate::business::user_database::node::dao;
use crate::business::user_database::node::vo::MoveNodeVO;
use crate::business::user_database::{canvas, edge, log, state};
use crate::error_code::ErrorCode;

/// 批量跨画布迁移节点。坐标由前端算好（含目标画布视口中心定位与网格吸附），本函数只负责校验与落库。
///
/// 校验规则（全部通过后才写库）：
/// - 目标画布存在且未逻辑删除；
/// - 每个节点都存在、不是影子节点、不是画布节点；
/// - 所有节点属于同一个源画布（源画布 == 目标画布时视为无操作，直接返回 Ok，不写库不产日志）；
/// - 源画布内不存在"恰有一个端点在集合内"的边（外部边）。此校验连带保证影子一致性：
///   节点成为影子 origin 的充要条件是它与本画布的画布节点有边，这类边必触发上述拒绝，
///   故合法迁移集永远不会有影子引用残留。
///
/// 写库：更新节点 canvas_id/x/y，两端都在集合内的内部边随迁至目标画布。
/// 产生一条 NodeRelocate 日志，object_id 为目标画布 id，载荷为节点数量与源/目标画布名称。
///
/// # 参数
/// - `items`: 要迁移的节点列表（含最终坐标）。
/// - `target_canvas_id`: 目标画布 id。
///
/// # 返回值
/// 成功时返回 `Ok(())`；目标画布不存在或已删除时返回 `ErrorCode::NoCanvasWithSuchId`，
/// 任一节点不存在时返回 `ErrorCode::NoNodeWithSuchId`，
/// 含影子节点时返回 `ErrorCode::NodeIsShadow`，含画布节点时返回 `ErrorCode::NodeIsCanvasNode`，
/// 节点分属不同画布时返回 `ErrorCode::NodeNotInSameCanvas`，
/// 存在外部边时返回 `ErrorCode::NodeSetHasExternalEdges`，
/// 发生其他错误时返回对应的 `ErrorCode`。
pub fn relocate_nodes(items: &[MoveNodeVO], target_canvas_id: &str) -> Result<(), ErrorCode> {
    // 空列表：no-op（对齐 move_nodes 的空列表语义）
    if items.is_empty() {
        return Ok(());
    }
    let connection = state::lock_connection();
    let target_canvas = canvas::dao::select_by_id(&connection, target_canvas_id)?
        .filter(|c| !c.deleted)
        .ok_or_else(|| ErrorCode::NoCanvasWithSuchId {
            id: target_canvas_id.to_string(),
        })?;
    // 逐节点加载并校验节点类型
    let mut nodes = Vec::with_capacity(items.len());
    for item in items {
        let node = dao::select_by_id(&connection, &item.id)?
            .ok_or_else(|| ErrorCode::NoNodeWithSuchId {
                id: item.id.clone(),
            })?;
        if node.shadow_id.is_some() {
            return Err(ErrorCode::NodeIsShadow);
        }
        if node.canvas_ref_id.is_some() {
            return Err(ErrorCode::NodeIsCanvasNode);
        }
        nodes.push(node);
    }
    // 所有节点必须同属一个源画布
    let source_canvas_id = nodes[0].canvas_id.clone();
    if nodes.iter().any(|n| n.canvas_id != source_canvas_id) {
        return Err(ErrorCode::NodeNotInSameCanvas);
    }
    // 源画布与目标画布相同：无操作
    if source_canvas_id == target_canvas_id {
        return Ok(());
    }
    // 外部边校验：恰有一个端点在集合内的边即外部边
    let id_set: HashSet<&str> = items.iter().map(|i| i.id.as_str()).collect();
    let edges = edge::dao::select_by_canvas_id(&connection, &source_canvas_id)?;
    for e in &edges {
        if id_set.contains(e.source_id.as_str()) != id_set.contains(e.target_id.as_str()) {
            return Err(ErrorCode::NodeSetHasExternalEdges);
        }
    }
    // 迁移节点
    let dao_items: Vec<(String, f64, f64)> = items
        .iter()
        .map(|i| (i.id.clone(), i.x, i.y))
        .collect();
    dao::batch_relocate(&connection, target_canvas_id, &dao_items)?;
    // 内部边随迁
    let internal_edge_ids: Vec<String> = edges
        .iter()
        .filter(|e| {
            id_set.contains(e.source_id.as_str()) && id_set.contains(e.target_id.as_str())
        })
        .map(|e| e.id.clone())
        .collect();
    edge::dao::batch_update_canvas_id(&connection, &internal_edge_ids, target_canvas_id)?;
    // 日志
    let source_canvas_name = canvas::dao::select_by_id(&connection, &source_canvas_id)?
        .map(|c| c.name)
        .unwrap_or_default();
    log::service::create(
        target_canvas_id,
        Action::NodeRelocate {
            node_count: items.len() as i64,
            source_canvas_name,
            target_canvas_name: target_canvas.name,
        },
    )?;
    Ok(())
}