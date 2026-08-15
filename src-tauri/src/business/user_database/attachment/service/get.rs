use crate::business::user_database::attachment::dao;
use crate::business::user_database::entity::Attachment;
use crate::business::user_database::state;
use crate::error_code::ErrorCode;

/// 按 id 获取附件元数据（不读取附件文件内容）。
///
/// # 参数
/// - `id`: 附件 id。
///
/// # 返回值
/// 返回附件实体；附件不存在时返回 `ErrorCode::NoAttachmentWithSuchId`，
/// 发生其他错误时返回对应的 `ErrorCode`。
pub fn get(id: &str) -> Result<Attachment, ErrorCode> {
    let connection = state::lock_connection();
    dao::select_by_id(&connection, id)?.ok_or_else(|| ErrorCode::NoAttachmentWithSuchId {
        id: id.to_string(),
    })
}
