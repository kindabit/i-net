use std::path::Path;

use crate::business::user_database::attachment::dao;
use crate::business::user_database::entity::Action;
use crate::business::user_database::node::dao as node_dao;
use crate::business::user_database::{log, state};
use crate::error_code::ErrorCode;
use crate::util::file_system_util;

/// 导出附件：将附件明文写入目标文件。明文读取复用 load 的逻辑。
/// 产生 AttachmentExport 日志，载荷为节点标题与文件名；
/// 查不到所属节点（数据异常场景）时不记日志，不影响主流程。
///
/// # 参数
/// - `id`: 附件 id。
/// - `target_path`: 导出目标文件路径。
///
/// # 返回值
/// 成功时返回 `Ok(())`；附件不存在时返回 `ErrorCode::NoAttachmentWithSuchId`，
/// 发生其他错误时返回对应的 `ErrorCode`。
pub fn export(id: &str, target_path: &str) -> Result<(), ErrorCode> {
    let connection = state::lock_connection();
    let attachment = dao::select_by_id(&connection, id)?.ok_or_else(|| {
        ErrorCode::NoAttachmentWithSuchId { id: id.to_string() }
    })?;
    let plaintext = super::load(id)?;
    file_system_util::write(Path::new(target_path), &plaintext)?;
    if let Some(node) = node_dao::select_by_id(&connection, &attachment.node_id)? {
        log::service::create(
            &attachment.node_id,
            Action::AttachmentExport {
                node_title: node.title,
                file_name: attachment.file_name,
            },
        )?;
    }
    Ok(())
}
