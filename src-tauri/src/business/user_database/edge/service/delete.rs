use rusqlite::Connection;

use crate::business::user_database::edge::dao;
use crate::business::user_database::entity::{Action, Node};
use crate::business::user_database::node::vo::ShadowDirection;
use crate::business::user_database::{log, node, state};
use crate::error_code::ErrorCode;

/// 物理删除指定边（边没有逻辑删除字段）。
///
/// 影子节点联动：如果边的某个端点是画布节点，则被引用子画布内另一端节点的影子节点
/// 随边一并物理删除（影子节点自身相连的边由外键级联删除）；两端都是画布节点时
/// 两个影子都会被删除。删除前收集影子节点在子画布内相连的节点（入向影子的出边目标、
/// 出向影子的入边源）：存在受影响节点且调用方未确认时返回
/// `ErrorCode::EdgeDeleteDisconnectsNodes`，由前端向用户确认后以 `confirmed = true` 重调。
///
/// 产生 EdgePhysicalDelete 日志，载荷为源节点的标题和目标节点的标题。
///
/// # 参数
/// - `id`: 边 id。
/// - `confirmed`: 调用方已确认影子节点删除带来的连接断开影响。
///
/// # 返回值
/// 成功时返回 `Ok(())`；边不存在时返回 `ErrorCode::NoEdgeWithSuchId`，
/// 删除会使子画布内的节点失去连接且未确认时返回 `ErrorCode::EdgeDeleteDisconnectsNodes`，
/// 发生其他错误时返回对应的 `ErrorCode`。
pub fn delete(id: &str, confirmed: bool) -> Result<(), ErrorCode> {
    let connection = state::lock_connection();
    let edge = dao::select_by_id(&connection, id)?
        .ok_or_else(|| ErrorCode::NoEdgeWithSuchId { id: id.to_string() })?;
    // 节点物理删除会连带删除边，因此此处两端节点必然仍然存在；
    // 查不到节点只可能是数据污染或程序缺陷。为保护剩余的用户数据，
    // 此时记录完整上下文并立即退出进程，不在受损状态下继续运行或写盘。
    // 该路径按设计不可单元测试（会终止测试进程），由代码审查保证。
    let source = node::dao::select_by_id(&connection, &edge.source_id)?;
    let Some(source) = source else {
        tracing::error!(
            "deleting edge {id}: source node {} does not exist (data corruption or program defect), exiting process immediately",
            edge.source_id
        );
        std::process::exit(1);
    };
    let target = node::dao::select_by_id(&connection, &edge.target_id)?;
    let Some(target) = target else {
        tracing::error!(
            "deleting edge {id}: target node {} does not exist (data corruption or program defect), exiting process immediately",
            edge.target_id
        );
        std::process::exit(1);
    };
    // 找到这条边关联的影子节点：target 是画布节点时，其引用画布内 source 的入向影子；
    // source 是画布节点时，其引用画布内 target 的出向影子。
    let mut shadows: Vec<Node> = Vec::new();
    if let Some(ref_canvas_id) = &target.canvas_ref_id {
        if let Some(shadow) = node::dao::select_by_shadow_id_and_canvas_id(
            &connection,
            &edge.source_id,
            ref_canvas_id,
        )? {
            shadows.push(shadow);
        }
    }
    if let Some(ref_canvas_id) = &source.canvas_ref_id {
        if let Some(shadow) = node::dao::select_by_shadow_id_and_canvas_id(
            &connection,
            &edge.target_id,
            ref_canvas_id,
        )? {
            shadows.push(shadow);
        }
    }
    // 收集受影响节点（须在删除边之前完成，影子方向推导依赖这条边）。
    let mut affected: Vec<String> = Vec::new();
    for shadow in &shadows {
        affected.extend(collect_disconnected_nodes(&connection, shadow)?);
    }
    if !affected.is_empty() && !confirmed {
        return Err(ErrorCode::EdgeDeleteDisconnectsNodes { nodes: affected });
    }
    dao::delete_by_id(&connection, id)?;
    // 物理删除影子节点：影子自身相连的边由 edge 外键随节点行的删除级联删除。
    for shadow in &shadows {
        node::dao::delete_by_id(&connection, &shadow.id)?;
    }
    log::service::create(
        id,
        Action::EdgePhysicalDelete {
            source_title: source.title,
            target_title: target.title,
        },
    )?;
    Ok(())
}

/// 收集影子节点在其画布内相连的节点的展示标题：入向影子收集出边目标，
/// 出向影子收集入边源，方向推导不出（数据不一致）时两个方向都收集。
/// 邻居本身是影子节点时取其原始节点的标题（影子行自身的标题是空串）。标题去重。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `shadow`: 影子节点。
///
/// # 返回值
/// 返回受影响节点的标题列表；发生数据库错误时返回对应的 `ErrorCode`。
fn collect_disconnected_nodes(
    connection: &Connection,
    shadow: &Node,
) -> Result<Vec<String>, ErrorCode> {
    let direction = node::service::shadow_direction(connection, shadow)?;
    let edges = dao::select_by_canvas_id(connection, &shadow.canvas_id)?;
    let mut titles: Vec<String> = Vec::new();
    for edge in &edges {
        let neighbor_id = if edge.source_id == shadow.id && direction != Some(ShadowDirection::Outflow)
        {
            Some(edge.target_id.as_str())
        } else if edge.target_id == shadow.id && direction != Some(ShadowDirection::Inflow) {
            Some(edge.source_id.as_str())
        } else {
            None
        };
        let Some(neighbor_id) = neighbor_id else {
            continue;
        };
        let Some(neighbor) = node::dao::select_by_id(connection, neighbor_id)? else {
            continue;
        };
        // 邻居是影子节点时，其自身标题是空串，展示用原始节点的标题。
        let title = match &neighbor.shadow_id {
            Some(origin_id) => node::dao::select_by_id(connection, origin_id)?
                .map(|origin| origin.title)
                .unwrap_or(neighbor.title),
            None => neighbor.title,
        };
        if !titles.contains(&title) {
            titles.push(title);
        }
    }
    Ok(titles)
}
