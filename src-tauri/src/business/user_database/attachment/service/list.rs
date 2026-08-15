use crate::business::user_database::attachment::dao;
use crate::business::user_database::attachment::vo::AttachmentVO;
use crate::business::user_database::node::dao as node_dao;
use crate::business::user_database::state;
use crate::error_code::ErrorCode;
use crate::util::file_system_util;

/// 获取指定节点的附件列表（按逻辑删除标志过滤），按导入时间升序。
/// 每个附件的 missing_file 标记经文件存在性检查后填充。不产生日志。
///
/// # 参数
/// - `node_id`: 节点 id。
/// - `deleted`: 逻辑删除标志（true 查回收站，false 查正常附件）。
///
/// # 返回值
/// 返回附件值对象列表；节点不存在时返回 `ErrorCode::NoNodeWithSuchId`，
/// 发生其他错误时返回对应的 `ErrorCode`。
pub fn list(node_id: &str, deleted: bool) -> Result<Vec<AttachmentVO>, ErrorCode> {
    let connection = state::lock_connection();
    node_dao::select_by_id(&connection, node_id)?.ok_or_else(|| {
        ErrorCode::NoNodeWithSuchId {
            id: node_id.to_string(),
        }
    })?;
    let attachments = dao::select_by_node_id(&connection, node_id, deleted)?;
    let path = crate::state::path();
    let user_uuid = state::metadata().id;
    let mut vos = Vec::with_capacity(attachments.len());
    for attachment in attachments {
        let file = path.user_attachment_file(&user_uuid, &attachment.id);
        vos.push(AttachmentVO {
            id: attachment.id,
            file_name: attachment.file_name,
            size: attachment.size,
            create_time: attachment.create_time,
            missing_file: !file_system_util::try_exists(&file)?,
        });
    }
    Ok(vos)
}
