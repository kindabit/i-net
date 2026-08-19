use std::path::Path;

use crate::business::user_database::attachment::dao;
use crate::business::user_database::attachment::service::MAX_ATTACHMENT_SIZE_MB;
use crate::business::user_database::attachment::vo::AttachmentVO;
use crate::business::user_database::entity::{Action, Attachment};
use crate::business::user_database::node::dao as node_dao;
use crate::business::user_database::{log, state};
use crate::error_code::ErrorCode;
use crate::security::aes;
use crate::util::compress;
use crate::util::{file_system_util, time_util};

/// 导入附件：读取源文件，大小校验后明文经压缩 guard（已是压缩格式的文件会直通）再加密写入附件目录，再插入附件元数据。
/// 先写文件成功再插行；行插入失败产生的孤儿文件由孤儿文件上报机制兜底，不自动清理。
/// 产生 AttachmentImport 日志，载荷为节点标题与文件名。
///
/// # 参数
/// - `node_id`: 附件所属节点的 id。
/// - `source_path`: 源文件路径。
///
/// # 返回值
/// 返回导入的附件值对象；节点不存在时返回 `ErrorCode::NoNodeWithSuchId`，节点是影子节点时返回 `ErrorCode::NodeIsShadow`，
/// 源文件路径取不到文件名时返回 `ErrorCode::EmptyFilePath`，
/// 明文大小超过上限时返回 `ErrorCode::AttachmentTooLarge`，
/// 发生其他错误时返回对应的 `ErrorCode`。
pub fn import(node_id: &str, source_path: &str) -> Result<AttachmentVO, ErrorCode> {
    let connection = state::lock_connection();
    let node = node_dao::select_by_id(&connection, node_id)?.ok_or_else(|| {
        ErrorCode::NoNodeWithSuchId {
            id: node_id.to_string(),
        }
    })?;
    // 影子节点不允许此操作（展示数据从原始节点拉取，生命周期由边管理）。
    if node.shadow_id.is_some() {
        return Err(ErrorCode::NodeIsShadow);
    }
    let file_name = Path::new(source_path)
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .filter(|name| !name.is_empty())
        .ok_or(ErrorCode::EmptyFilePath)?;
    let plaintext = file_system_util::read(Path::new(source_path))?;
    let size = plaintext.len();
    if size as u64 > MAX_ATTACHMENT_SIZE_MB * 1024 * 1024 {
        return Err(ErrorCode::AttachmentTooLarge {
            max: MAX_ATTACHMENT_SIZE_MB,
        });
    }
    let guard_output: compress::GuardOutput = compress::compress(&file_name, plaintext)?;
    let attachment_id = uuid::Uuid::new_v4().to_string();
    let ciphertext = aes::encrypt(guard_output.data, state::key())?;
    let path = crate::state::path();
    let file = path.user_attachment_file(&state::metadata().id, &attachment_id);
    file_system_util::write(&file, &ciphertext)?;
    let max_sort_order = dao::select_max_sort_order(&connection, node_id)?;
    let attachment = Attachment {
        id: attachment_id,
        node_id: node_id.to_string(),
        file_name,
        size: size as i64,
        create_time: time_util::now(),
        deleted: false,
        sort_order: max_sort_order + 1,
        compressed: guard_output.compressed,
        compress_param: guard_output.compress_param,
    };
    dao::insert(&connection, &attachment)?;
    log::service::create(
        node_id,
        Action::AttachmentImport {
            node_title: node.title,
            file_name: attachment.file_name.clone(),
        },
    )?;
    Ok(AttachmentVO {
        id: attachment.id,
        file_name: attachment.file_name,
        size: attachment.size,
        create_time: attachment.create_time,
        missing_file: false,
    })
}
