use crate::business::user_database::edge::dao;
use crate::business::user_database::entity::Action;
use crate::business::user_database::{log, node, state};
use crate::error_code::ErrorCode;

/// 物理删除指定边（边没有逻辑删除字段）。
///
/// 影子节点联动：边被删除后，其产生的影子节点经 node.shadow_id 外键级联删除；
/// 影子的相连边经 edge.source_id/target_id 外键级联删除；这些相连边若也是产生影子节点的边，
/// 其影子递归级联删除（嵌套影子沿外键链递归坍塌），应用层不再手动删除影子。
/// 受影响节点的收集通过 [`node::service::collect_edge_disconnected`] 沿同一外键链
/// 预收集（须在删除边之前完成）：影子方向必然可推导，
/// 邻居必然是非影子节点，任何不一致都返回 DataCorruption* 错误。存在受影响节点且
/// 调用方未确认时返回 `ErrorCode::EdgeDeleteDisconnectsNodes`，由前端向用户确认后
/// 以 `confirmed = true` 重调。
///
/// 产生 EdgePhysicalDelete 日志，载荷为源节点的标题和目标节点的标题。
///
/// # 参数
/// - `id`: 边 id。
/// - `confirmed`: 调用方已确认影子节点删除带来的连接断开影响。
///
/// # 返回值
/// 成功时返回 `Ok(())`；边不存在时返回 `ErrorCode::NoEdgeWithSuchId`，
/// 端点节点不存在时返回 `ErrorCode::DataCorruptionEdgeEndpointMissing`，
/// 删除会使子画布内的节点失去连接且未确认时返回 `ErrorCode::EdgeDeleteDisconnectsNodes`，
/// 发生其他错误时返回对应的 `ErrorCode`。
pub fn delete(id: &str, confirmed: bool) -> Result<(), ErrorCode> {
    let connection = state::lock_connection();
    let edge = dao::select_by_id(&connection, id)?
        .ok_or_else(|| ErrorCode::NoEdgeWithSuchId { id: id.to_string() })?;
    // 节点物理删除会连带删除边，因此此处两端节点必然仍然存在；
    // 查不到节点只可能是数据污染或程序缺陷，返回 DataCorruptionEdgeEndpointMissing
    // 由前端受控崩溃；该路径构造脏数据需绕过外键约束，按设计不单元测试，由代码审查保证。
    let source = node::dao::select_by_id(&connection, &edge.source_id)?.ok_or_else(|| {
        ErrorCode::DataCorruptionEdgeEndpointMissing {
            edge_id: id.to_string(),
            node_id: edge.source_id.clone(),
        }
    })?;
    let target = node::dao::select_by_id(&connection, &edge.target_id)?.ok_or_else(|| {
        ErrorCode::DataCorruptionEdgeEndpointMissing {
            edge_id: id.to_string(),
            node_id: edge.target_id.clone(),
        }
    })?;
    // 断连预收集须在删除边之前完成（收集依赖产生边链完好）；
    // 受影响节点非空且未确认时返回 EdgeDeleteDisconnectsNodes，由前端确认后重调。
    let affected = node::service::collect_edge_disconnected(&connection, &edge)?;
    if !affected.is_empty() && !confirmed {
        return Err(ErrorCode::EdgeDeleteDisconnectsNodes { nodes: affected });
    }
    // 删除边：其产生的影子经 node.shadow_id 外键级联删除，影子的相连边经
    // edge.source_id/target_id 外键级联删除，下游嵌套影子沿外键链递归坍塌，
    // 应用层禁止手写递归删除。
    dao::delete_by_id(&connection, id)?;
    log::service::create(
        id,
        Action::EdgePhysicalDelete {
            source_title: source.title,
            target_title: target.title,
        },
    )?;
    Ok(())
}
