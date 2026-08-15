use crate::business::user_database::edge::dao;
use crate::business::user_database::entity::Action;
use crate::business::user_database::{log, node, state};
use crate::error_code::ErrorCode;

/// 物理删除指定边（边没有逻辑删除字段）。
///
/// 产生 EdgePhysicalDelete 日志，载荷为源节点的标题和目标节点的标题。
///
/// # 参数
/// - `id`: 边 id。
///
/// # 返回值
/// 成功时返回 `Ok(())`；边不存在时返回 `ErrorCode::NoEdgeWithSuchId`，
/// 发生其他错误时返回对应的 `ErrorCode`。
pub fn delete(id: &str) -> Result<(), ErrorCode> {
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
