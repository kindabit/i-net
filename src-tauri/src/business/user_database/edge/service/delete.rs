use crate::business::user_database::edge::dao;
use crate::business::user_database::entity::Action;
use crate::business::user_database::{log, node, state};
use crate::error_code::ErrorCode;

/// 物理删除指定边（边没有逻辑删除字段）。
///
/// 影子节点联动：如果边的某个端点是画布节点，则被引用子画布内另一端节点的影子节点
/// 随边一并物理删除（影子节点自身相连的边由外键级联删除）；两端都是画布节点时
/// 两个影子都会被删除。影子支持嵌套（影子的影子），嵌套影子会由 node.shadow_id
/// 自引用外键与 edge.source_id/target_id 外键在 SQLite 层逐层级联删除，应用层
/// 禁止手写递归删除。受影响节点的收集通过 [`node::service::collect_shadow_disconnected`]
/// 递归覆盖下游各级画布：影子方向必然可推导，邻居必然是非影子节点（数据不一致时
/// collect_shadow_disconnected 返回 DataCorruption* 错误）。存在受影响节点且调用方未确认时
/// 返回 `ErrorCode::EdgeDeleteDisconnectsNodes`，由前端向用户确认后以 `confirmed = true` 重调。
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
    // 找到这条边关联的影子节点：复用 edge::service::shadows_of_edge。
    let shadows = super::shadows_of_edge(&connection, &source, &target)?;
    // 收集受影响节点（须在删除边之前完成，影子方向推导依赖这条边）。
    // 嵌套影子的断连递归覆盖到下游各级画布。
    let mut affected: Vec<String> = Vec::new();
    for shadow in &shadows {
        affected.extend(node::service::collect_shadow_disconnected(
            &connection,
            shadow,
        )?);
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
