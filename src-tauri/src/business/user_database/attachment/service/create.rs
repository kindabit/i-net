use crate::business::user_database::attachment::dao;
use crate::business::user_database::attachment::vo::AttachmentVO;
use crate::business::user_database::entity::{Action, Attachment};
use crate::business::user_database::node::dao as node_dao;
use crate::business::user_database::{log, state};
use crate::error_code::ErrorCode;
use crate::security::aes;
use crate::util::compress;
use crate::util::{file_system_util, time_util};

/// 新建文本附件：以指定文件名创建一个内容为空的附件，供用户随后在文本编辑器中写入内容。
/// 压缩参数按文件名路由（见 `compress::compress_forced`），空内容同样按压缩模式落盘，
/// 与后续写入文本内容时的压缩路由保持一致。
/// 先写文件成功再插行；行插入失败产生的孤儿文件由孤儿文件上报机制兜底，不自动清理。
/// 产生 AttachmentCreate 日志，载荷为节点标题与文件名。
///
/// # 参数
/// - `node_id`: 附件所属节点的 id。
/// - `file_name`: 附件文件名。
///
/// # 返回值
/// 返回新建的附件值对象；节点不存在时返回 `ErrorCode::NoNodeWithSuchId`，节点是影子节点时返回 `ErrorCode::NodeIsShadow`，
/// 发生其他错误时返回对应的 `ErrorCode`。
pub fn create(node_id: &str, file_name: &str) -> Result<AttachmentVO, ErrorCode> {
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
    let guard_output: compress::GuardOutput = compress::compress_forced(file_name, Vec::new())?;
    let attachment_id = uuid::Uuid::new_v4().to_string();
    let ciphertext = aes::encrypt(guard_output.data, state::key())?;
    let path = crate::state::path();
    let file = path.user_attachment_file(&state::metadata().id, &attachment_id);
    file_system_util::write(&file, &ciphertext)?;
    let max_sort_order = dao::select_max_sort_order(&connection, node_id)?;
    let attachment = Attachment {
        id: attachment_id,
        node_id: node_id.to_string(),
        file_name: file_name.to_string(),
        size: 0,
        create_time: time_util::now(),
        deleted: false,
        sort_order: max_sort_order + 1,
        compressed: guard_output.compressed,
        compress_param: guard_output.compress_param,
    };
    dao::insert(&connection, &attachment)?;
    log::service::create(
        node_id,
        Action::AttachmentCreate {
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
