use crate::business::user_database::attachment::dao;
use crate::business::user_database::entity::Action;
use crate::business::user_database::node::dao as node_dao;
use crate::business::user_database::{log, state};
use crate::error_code::ErrorCode;

/// 重命名附件：更新附件元数据中的文件名，附件文件本身（以附件 id 命名）不受影响。
/// 产生 AttachmentRename 日志，载荷为节点标题、旧文件名与新文件名；
/// 查不到所属节点（数据异常场景）时不记日志，不影响主流程。
///
/// # 参数
/// - `id`: 附件 id。
/// - `new_file_name`: 新文件名。
///
/// # 返回值
/// 成功时返回 `Ok(())`；附件不存在时返回 `ErrorCode::NoAttachmentWithSuchId`，
/// 发生其他错误时返回对应的 `ErrorCode`。
pub fn rename(id: &str, new_file_name: String) -> Result<(), ErrorCode> {
    let connection = state::lock_connection();
    let mut attachment = dao::select_by_id(&connection, id)?.ok_or_else(|| {
        ErrorCode::NoAttachmentWithSuchId { id: id.to_string() }
    })?;
    let old_file_name = std::mem::replace(&mut attachment.file_name, new_file_name);
    dao::update(&connection, &attachment)?;
    if let Some(node) = node_dao::select_by_id(&connection, &attachment.node_id)? {
        log::service::create(
            &attachment.node_id,
            Action::AttachmentRename {
                node_title: node.title,
                old_file_name,
                new_file_name: attachment.file_name,
            },
        )?;
    }
    Ok(())
}