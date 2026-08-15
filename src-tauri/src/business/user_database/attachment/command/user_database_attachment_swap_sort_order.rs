use crate::business::user_database::attachment::service;
use crate::error_code::ErrorCode;
use crate::util::preprocess_util;

/// 交换两个附件的排序位置。
///
/// # 参数
/// - `id1`: 附件1 id。
/// - `id2`: 附件2 id。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_attachment_swap_sort_order(
    id1: String,
    id2: String,
) -> Result<(), ErrorCode> {
    preprocess(id1, id2)
}

/// `user_database_attachment_swap_sort_order` 的 preprocess 函数：校验两个 id 后接入 service 层的 swap_sort_order 函数。
pub fn preprocess(id1: String, id2: String) -> Result<(), ErrorCode> {
    let id1 = preprocess_util::preprocess_attachment_id(id1)?;
    let id2 = preprocess_util::preprocess_attachment_id(id2)?;
    service::swap_sort_order(&id1, &id2)
}
