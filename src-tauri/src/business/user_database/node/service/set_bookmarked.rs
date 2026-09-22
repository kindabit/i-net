use crate::business::user_database::entity::Action;
use crate::business::user_database::node::dao;
use crate::business::user_database::{log, state};
use crate::error_code::ErrorCode;

/// 设置指定节点的书签状态。
///
/// 无变化（当前状态与目标状态相同）时直接返回，不写库不写日志；
/// 否则更新书签状态并产生 NodeBookmarkModify 日志。
///
/// # 参数
/// - `id`: 节点 id。
/// - `bookmarked`: 目标书签状态，true 表示收藏，false 表示取消收藏。
///
/// # 返回值
/// 成功时返回 `Ok(())`；节点不存在时返回 `ErrorCode::NoNodeWithSuchId`，
/// 影子节点时返回 `ErrorCode::NodeIsShadow`，发生其他错误时返回对应的 `ErrorCode`。
pub fn set_bookmarked(id: &str, bookmarked: bool) -> Result<(), ErrorCode> {
    let connection = state::lock_connection();
    let mut node = dao::select_by_id(&connection, id)?
        .ok_or_else(|| ErrorCode::NoNodeWithSuchId { id: id.to_string() })?;
    // 影子节点不允许此操作（展示数据从本体节点拉取，生命周期由边管理）。
    if node.shadow_producing_edge_id.is_some() {
        return Err(ErrorCode::NodeIsShadow);
    }
    if node.bookmarked == bookmarked {
        return Ok(());
    }
    node.bookmarked = bookmarked;
    dao::update(&connection, &node)?;
    log::service::create(Action::NodeBookmarkModify {
        node_title: node.title.clone(),
        bookmarked,
    })?;
    Ok(())
}
