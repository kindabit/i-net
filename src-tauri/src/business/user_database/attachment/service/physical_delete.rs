use crate::business::user_database::attachment::dao;
use crate::business::user_database::entity::Action;
use crate::business::user_database::node::dao as node_dao;
use crate::business::user_database::{log, state};
use crate::error_code::ErrorCode;
use crate::util::file_system_util;

/// 物理删除附件：先删除附件文件再删除附件元数据（先文件后行，失败中止保持一致性），
/// 附件文件已缺失时不视为错误，继续删除元数据。
/// 产生 AttachmentPhysicalDelete 日志，载荷为节点标题与文件名；
/// 查不到所属节点（数据异常场景）时不记日志，不影响主流程。
///
/// # 参数
/// - `id`: 附件 id。
///
/// # 返回值
/// 成功时返回 `Ok(())`；附件不存在时返回 `ErrorCode::NoAttachmentWithSuchId`，
/// 发生其他错误时返回对应的 `ErrorCode`。
pub fn physical_delete(id: &str) -> Result<(), ErrorCode> {
    let connection = state::lock_connection();
    let attachment = dao::select_by_id(&connection, id)?.ok_or_else(|| {
        ErrorCode::NoAttachmentWithSuchId { id: id.to_string() }
    })?;
    let path = crate::state::path();
    let file = path.user_attachment_file(&state::metadata().id, id);
    if file_system_util::try_exists(&file)? {
        file_system_util::remove_file(&file)?;
    }
    dao::delete_by_id(&connection, id)?;
    if let Some(node) = node_dao::select_by_id(&connection, &attachment.node_id)? {
        log::service::create(
            &attachment.node_id,
            Action::AttachmentPhysicalDelete {
                node_title: node.title,
                file_name: attachment.file_name,
            },
        )?;
    }
    Ok(())
}
