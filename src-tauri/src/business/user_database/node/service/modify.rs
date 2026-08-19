use crate::business::user_database::entity::Action;
use crate::business::user_database::node::dao;
use crate::business::user_database::{log, state};
use crate::error_code::ErrorCode;

/// 修改指定节点的标题和副标题。
///
/// 产生 NodeModify 日志，载荷为旧标题、副标题和新标题、副标题。
///
/// # 参数
/// - `id`: 节点 id。
/// - `title`: 新标题。
/// - `sub_title`: 新副标题。
///
/// # 返回值
/// 成功时返回 `Ok(())`；节点不存在时返回 `ErrorCode::NoNodeWithSuchId`，影子节点时返回 `ErrorCode::NodeIsShadow`，
/// 发生其他错误时返回对应的 `ErrorCode`。
pub fn modify(id: &str, title: String, sub_title: String) -> Result<(), ErrorCode> {
    let connection = state::lock_connection();
    let mut node = dao::select_by_id(&connection, id)?
        .ok_or_else(|| ErrorCode::NoNodeWithSuchId { id: id.to_string() })?;
    // 影子节点不允许此操作（展示数据从原始节点拉取，生命周期由边管理）。
    if node.shadow_id.is_some() {
        return Err(ErrorCode::NodeIsShadow);
    }
    let old_title = std::mem::replace(&mut node.title, title);
    let old_sub_title = std::mem::replace(&mut node.sub_title, sub_title);
    dao::update(&connection, &node)?;
    log::service::create(
        id,
        Action::NodeModify {
            old_title,
            old_sub_title,
            new_title: node.title,
            new_sub_title: node.sub_title,
        },
    )?;
    Ok(())
}
