use super::collect_subtree_ids;
use crate::business::user_database::canvas::dao;
use crate::business::user_database::entity::Action;
use crate::business::user_database::{log, node, state};
use crate::error_code::ErrorCode;

/// 逻辑删除指定画布以及它的全部子孙画布：
/// 对子树内每个尚未逻辑删除的画布置上逻辑删除标志；
/// 同时逻辑删除所有引用这些画布的画布节点。
///
/// 每个被逻辑删除的画布产生一条 CanvasLogicalDelete 日志，载荷内记录画布名称。
/// 每个被逻辑删除的画布节点产生一条 NodeLogicalDelete 日志。
///
/// # 参数
/// - `id`: 画布 id。
///
/// # 返回值
/// 成功时返回 `Ok(())`；画布不存在时返回 `ErrorCode::NoCanvasWithSuchId`，
/// 目标是根画布时返回 `ErrorCode::RootCanvasCannotBeDeleted`，
/// 发生其他错误时返回对应的 `ErrorCode`。
pub fn logical_delete(id: &str) -> Result<(), ErrorCode> {
    let connection = state::lock_connection();
    let all = dao::select_all(&connection)?;
    let target = all
        .iter()
        .find(|canvas| canvas.id == id)
        .ok_or_else(|| ErrorCode::NoCanvasWithSuchId { id: id.to_string() })?;
    // 根画布是整棵画布树的根，删除它会摧毁整个数据库的内容，因此直接拒绝。
    if target.parent_id.is_none() {
        return Err(ErrorCode::RootCanvasCannotBeDeleted);
    }
    let subtree_ids = collect_subtree_ids(&all, id);
    let mut deleted = Vec::new();
    let mut node_deleted = Vec::new();
    for canvas in all
        .iter()
        .filter(|canvas| subtree_ids.contains(&canvas.id) && !canvas.deleted)
    {
        let mut canvas = canvas.clone();
        canvas.deleted = true;
        dao::update(&connection, &canvas)?;
        deleted.push((canvas.id.clone(), canvas.name));
        // 逻辑删除引用该画布的画布节点（若存在且未删除）
        if let Some(mut ref_node) =
            node::dao::select_by_canvas_ref_id(&connection, &canvas.id)?
        {
            if !ref_node.deleted {
                ref_node.deleted = true;
                node::dao::update(&connection, &ref_node)?;
                node_deleted.push((ref_node.id, ref_node.title));
            }
        }
    }
    for (id, name) in deleted {
        log::service::create(&id, Action::CanvasLogicalDelete { name })?;
    }
    for (id, title) in node_deleted {
        log::service::create(&id, Action::NodeLogicalDelete { title })?;
    }
    Ok(())
}
