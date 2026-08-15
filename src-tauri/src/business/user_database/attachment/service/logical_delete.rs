use crate::business::user_database::attachment::dao;
use crate::business::user_database::entity::Action;
use crate::business::user_database::node::dao as node_dao;
use crate::business::user_database::{log, state};
use crate::error_code::ErrorCode;

/// 逻辑删除附件：将附件标记为已删除（进入回收站），附件文件保留不动。
/// 产生 AttachmentLogicalDelete 日志，载荷为节点标题与文件名；
/// 查不到所属节点（数据异常场景）时不记日志，不影响主流程。
///
/// # 参数
/// - `id`: 附件 id。
///
/// # 返回值
/// 成功时返回 `Ok(())`；附件不存在时返回 `ErrorCode::NoAttachmentWithSuchId`，
/// 发生其他错误时返回对应的 `ErrorCode`。
pub fn logical_delete(id: &str) -> Result<(), ErrorCode> {
    let connection = state::lock_connection();
    let attachment = dao::select_by_id(&connection, id)?.ok_or_else(|| {
        ErrorCode::NoAttachmentWithSuchId { id: id.to_string() }
    })?;
    dao::update_deleted(&connection, id, true)?;
    if let Some(node) = node_dao::select_by_id(&connection, &attachment.node_id)? {
        log::service::create(
            &attachment.node_id,
            Action::AttachmentLogicalDelete {
                node_title: node.title,
                file_name: attachment.file_name,
            },
        )?;
    }
    Ok(())
}
