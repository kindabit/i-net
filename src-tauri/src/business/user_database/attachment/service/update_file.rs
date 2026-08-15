use crate::business::user_database::attachment::dao;
use crate::business::user_database::attachment::service::MAX_ATTACHMENT_SIZE_MB;
use crate::business::user_database::entity::Action;
use crate::business::user_database::node::dao as node_dao;
use crate::business::user_database::{log, state};
use crate::error_code::ErrorCode;
use crate::security::aes;
use crate::util::compress;
use crate::util::file_system_util;

/// 覆盖保存附件内容：用新的明文经压缩 guard 后重新加密并写入附件文件，再同步更新附件元数据中的大小与压缩标记。
/// 先写文件成功再更新元数据，与 import 的"先写文件后改元数据"顺序一致，失败中止保持一致性。
/// 产生 AttachmentUpdate 日志，载荷为节点标题与文件名；
/// 查不到所属节点（数据异常场景）时不记日志，不影响主流程。
///
/// # 参数
/// - `id`: 附件 id。
/// - `plaintext`: 新的附件明文内容。
///
/// # 返回值
/// 成功时返回 `Ok(())`；附件不存在时返回 `ErrorCode::NoAttachmentWithSuchId`，
/// 明文大小超过上限时返回 `ErrorCode::AttachmentTooLarge`，
/// 发生其他错误时返回对应的 `ErrorCode`。
pub fn update_file(id: &str, plaintext: &[u8]) -> Result<(), ErrorCode> {
    let connection = state::lock_connection();
    let attachment = dao::select_by_id(&connection, id)?.ok_or_else(|| {
        ErrorCode::NoAttachmentWithSuchId { id: id.to_string() }
    })?;
    if plaintext.len() as u64 > MAX_ATTACHMENT_SIZE_MB * 1024 * 1024 {
        return Err(ErrorCode::AttachmentTooLarge {
            max: MAX_ATTACHMENT_SIZE_MB,
        });
    }
    let guard_output: compress::GuardOutput = compress::compress(&attachment.file_name, plaintext.to_vec())?;
    let ciphertext = aes::encrypt(guard_output.data, state::key())?;
    let path = crate::state::path();
    let file = path.user_attachment_file(&state::metadata().id, id);
    file_system_util::write(&file, &ciphertext)?;
    let mut updated = attachment.clone();
    updated.size = plaintext.len() as i64;
    updated.compressed = guard_output.compressed;
    updated.compress_param = guard_output.compress_param;
    dao::update(&connection, &updated)?;
    if let Some(node) = node_dao::select_by_id(&connection, &attachment.node_id)? {
        log::service::create(
            &attachment.node_id,
            Action::AttachmentUpdate {
                node_title: node.title,
                file_name: attachment.file_name,
            },
        )?;
    }
    Ok(())
}
