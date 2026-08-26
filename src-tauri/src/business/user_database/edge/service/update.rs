use crate::business::user_database::edge::dao;
use crate::business::user_database::entity::Action;
use crate::business::user_database::{log, node, state};
use crate::error_code::ErrorCode;

/// 更新指定边的标题和详情。
///
/// 产生 EdgeUpdate 日志，载荷为源节点标题、目标节点标题、旧标题、旧详情、新标题和新详情。
///
/// # 参数
/// - `id`: 边 id。
/// - `title`: 新标题。
/// - `description`: 新详情。
///
/// # 返回值
/// 成功时返回 `Ok(())`；边不存在时返回 `ErrorCode::NoEdgeWithSuchId`，
/// 端点节点不存在时返回 `ErrorCode::DataCorruptionEdgeEndpointMissing`，
/// 发生其他错误时返回对应的 `ErrorCode`。
pub fn update(id: &str, title: String, description: String) -> Result<(), ErrorCode> {
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
    let old_title = edge.title.clone();
    let old_description = edge.description.clone();
    dao::update_title_and_description(&connection, id, &title, &description)?;
    log::service::create(
        id,
        Action::EdgeUpdate {
            source_title: source.title,
            target_title: target.title,
            old_title,
            old_description,
            new_title: title,
            new_description: description,
        },
    )?;
    Ok(())
}
