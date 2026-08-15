use crate::business::user_database::attachment::dao;
use crate::business::user_database::state;
use crate::error_code::ErrorCode;

/// 交换两个附件的排序位置。
/// 校验两个附件存在、属于同一节点、且未逻辑删除，然后交换它们的 sort_order。
///
/// # 参数
/// - `id1`: 附件1 id。
/// - `id2`: 附件2 id。
///
/// # 返回值
/// 成功返回 Ok。附件不存在、不属于同一节点、或已逻辑删除时返回对应 ErrorCode。
pub fn swap_sort_order(id1: &str, id2: &str) -> Result<(), ErrorCode> {
    let connection = state::lock_connection();

    let attachment1 = dao::select_by_id(&connection, id1)?.ok_or_else(|| {
        ErrorCode::NoAttachmentWithSuchId { id: id1.to_string() }
    })?;
    let attachment2 = dao::select_by_id(&connection, id2)?.ok_or_else(|| {
        ErrorCode::NoAttachmentWithSuchId { id: id2.to_string() }
    })?;

    if attachment1.node_id != attachment2.node_id {
        return Err(ErrorCode::AttachmentNotInSameNode);
    }

    if attachment1.deleted || attachment2.deleted {
        return Err(ErrorCode::AttachmentDeleted);
    }

    dao::swap_sort_order(
        &connection,
        id1,
        id2,
        attachment1.sort_order,
        attachment2.sort_order,
    )?;

    Ok(())
}
