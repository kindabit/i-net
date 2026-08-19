use crate::business::user_database::entity::Action;
use crate::business::user_database::node::dao;
use crate::business::user_database::{attachment, canvas, log, state};
use crate::error_code::ErrorCode;
use crate::util::file_system_util;

/// 物理删除指定节点；与它相连的全部边、该节点的全部字段和附件元数据由外键
/// ON DELETE CASCADE 随节点行的删除一并删除，附件的磁盘文件在删行之前逐个清理。
/// 如果是画布节点，同时物理删除其引用的子画布。
///
/// 产生 NodePhysicalDelete 日志，载荷为节点的标题。级联删除（边、字段、附件）不记日志。
///
/// # 参数
/// - `id`: 节点 id。
///
/// # 返回值
/// 成功时返回 `Ok(())`；节点不存在时返回 `ErrorCode::NoNodeWithSuchId`，影子节点时返回 `ErrorCode::NodeIsShadow`，
/// 发生其他错误时返回对应的 `ErrorCode`。
pub fn physical_delete(id: &str) -> Result<(), ErrorCode> {
    let connection = state::lock_connection();
    let node = dao::select_by_id(&connection, id)?
        .ok_or_else(|| ErrorCode::NoNodeWithSuchId { id: id.to_string() })?;
    // 影子节点不允许此操作（展示数据从原始节点拉取，生命周期由边管理）。
    if node.shadow_id.is_some() {
        return Err(ErrorCode::NodeIsShadow);
    }
    // 清理附件磁盘文件：先删文件再删节点行，附件元数据由外键随节点行级联删除。
    let attachments = attachment::dao::select_by_node_ids(&connection, &[id.to_string()])?;
    let path = crate::state::path();
    let user_uuid = state::metadata().id;
    for a in &attachments {
        let file = path.user_attachment_file(&user_uuid, &a.id);
        if file_system_util::try_exists(&file)? {
            file_system_util::remove_file(&file)?;
        }
    }
    // 删除节点行：相连的边（edge.source_id/target_id）、节点字段（node_field.node_id）、
    // 附件元数据（attachment.node_id）由外键级联删除。
    dao::delete_by_id(&connection, id)?;
    let canvas_ref_id = node.canvas_ref_id.clone();
    log::service::create(id, Action::NodePhysicalDelete { title: node.title })?;
    // 级联物理删除引用的子画布
    if let Some(ref_id) = canvas_ref_id {
        canvas::service::physical_delete(&ref_id)?;
    }
    Ok(())
}
