use crate::business::user_database::edge::service;
use crate::error_code::ErrorCode;
use crate::util::preprocess_util;

/// 更新指定边的标题和详情。
///
/// # 参数
/// - `id`: 边 id。
/// - `title`: 新标题。
/// - `description`: 新详情。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_edge_update(
    id: String,
    title: String,
    description: String,
) -> Result<(), ErrorCode> {
    preprocess(id, title, description)
}

/// `user_database_edge_update` 的 preprocess 函数：校验参数后接入 service 层的 update 函数。
pub fn preprocess(
    id: String,
    title: String,
    description: String,
) -> Result<(), ErrorCode> {
    let id = preprocess_util::preprocess_edge_id(id)?;
    let title = preprocess_util::preprocess_edge_title(title)?;
    let description = preprocess_util::preprocess_edge_description(description)?;
    service::update(&id, title, description)
}
