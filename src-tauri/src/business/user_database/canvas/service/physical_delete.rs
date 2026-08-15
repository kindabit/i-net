use super::collect_subtree_ids;
use crate::business::user_database::canvas::dao;
use crate::business::user_database::entity::Action;
use crate::business::user_database::{attachment, log, node, state, viewport};
use crate::error_code::ErrorCode;
use crate::util::file_system_util;

/// 物理删除指定画布以及它的全部子孙画布，
/// 包括这些画布中的全部节点、边、视口、节点字段和附件；
/// 同时物理删除引用目标画布自身的画布节点及其相连边、字段和附件；
/// 子树内其它画布的引用节点位于子树内部，随子树节点一并删除。
///
/// 画布、节点、边、节点字段、附件元数据的行删除由外键 ON DELETE CASCADE 随
/// 目标画布行的删除递归完成；视口无外键约束（画布宇宙视口使用特殊 canvas_id），
/// 逐个画布手工删除；附件的磁盘文件在删行之前逐个清理。
///
/// 每个被物理删除的画布产生一条 CanvasPhysicalDelete 日志，载荷内记录画布名称。
/// 每个被物理删除的画布节点产生一条 NodePhysicalDelete 日志。
/// 级联删除（边、字段、附件、视口）不记日志。
///
/// # 参数
/// - `id`: 画布 id。
///
/// # 返回值
/// 成功时返回 `Ok(())`；画布不存在时返回 `ErrorCode::NoCanvasWithSuchId`，
/// 目标是根画布时返回 `ErrorCode::RootCanvasCannotBeDeleted`，
/// 发生其他错误时返回对应的 `ErrorCode`。
pub fn physical_delete(id: &str) -> Result<(), ErrorCode> {
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
    let path = crate::state::path();
    let user_uuid = state::metadata().id;
    // 收集子树内全部节点 id（正常与回收站各查一次合并），
    // 用于在删行之前清理这些节点的附件磁盘文件。
    let mut node_ids: Vec<String> = Vec::new();
    for canvas_id in &subtree_ids {
        node_ids.extend(
            node::dao::select_by_canvas_id_and_deleted(&connection, canvas_id, false)?
                .into_iter()
                .map(|node| node.id),
        );
        node_ids.extend(
            node::dao::select_by_canvas_id_and_deleted(&connection, canvas_id, true)?
                .into_iter()
                .map(|node| node.id),
        );
    }
    let attachments = attachment::dao::select_by_node_ids(&connection, &node_ids)?;
    for a in &attachments {
        let file = path.user_attachment_file(&user_uuid, &a.id);
        if file_system_util::try_exists(&file)? {
            file_system_util::remove_file(&file)?;
        }
    }
    // 引用目标画布的画布节点只可能存在于目标画布的父画布中（创建入口保证
    // parent_id == 节点所在画布），其子树内其它画布的引用节点位于子树内部。
    // 引用节点的行以及相连边、字段、附件元数据由 node.canvas_ref_id 外键随目标
    // 画布行的删除级联删除；此处先清理其附件磁盘文件并记录日志载荷。
    let mut node_deleted: Option<(String, String)> = None;
    if let Some(ref_node) = node::dao::select_by_canvas_ref_id(&connection, id)? {
        let attachments =
            attachment::dao::select_by_node_ids(&connection, &[ref_node.id.clone()])?;
        for a in &attachments {
            let file = path.user_attachment_file(&user_uuid, &a.id);
            if file_system_util::try_exists(&file)? {
                file_system_util::remove_file(&file)?;
            }
        }
        node_deleted = Some((ref_node.id.clone(), ref_node.title.clone()));
    }
    // 视口无外键约束，逐个画布手工删除。
    for canvas_id in &subtree_ids {
        viewport::dao::delete_by_canvas_id(&connection, canvas_id)?;
    }
    // 删除目标画布行：子画布（canvas.parent_id）、子树内节点（node.canvas_id）、
    // 引用节点（node.canvas_ref_id）、边（edge.canvas_id/source_id/target_id）、
    // 节点字段（node_field.node_id）、附件元数据（attachment.node_id）
    // 全部由外键递归级联删除。
    dao::delete_by_id(&connection, id)?;
    let mut deleted = Vec::new();
    for canvas_id in &subtree_ids {
        if let Some(canvas) = all.iter().find(|canvas| &canvas.id == canvas_id) {
            deleted.push((canvas.id.clone(), canvas.name.clone()));
        }
    }
    for (id, name) in deleted {
        log::service::create(&id, Action::CanvasPhysicalDelete { name })?;
    }
    if let Some((id, title)) = node_deleted {
        log::service::create(&id, Action::NodePhysicalDelete { title })?;
    }
    Ok(())
}
