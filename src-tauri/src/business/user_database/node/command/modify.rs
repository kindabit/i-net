use crate::business::user_database::node::service;
use crate::error_code::ErrorCode;
use crate::util::preprocess_util;

/// 修改指定节点的标题和副标题。
///
/// # 参数
/// - `id`: 节点 id。
/// - `title`: 新标题。
/// - `sub_title`: 新副标题。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_node_modify(
    id: String,
    title: String,
    sub_title: String,
) -> Result<(), ErrorCode> {
    preprocess(id, title, sub_title)
}

/// `user_database_node_modify` 的 preprocess 函数：校验参数后接入 service 层的 modify 函数。
///
/// 标题和副标题只裁剪首尾空白字符，允许为空。
pub fn preprocess(id: String, title: String, sub_title: String) -> Result<(), ErrorCode> {
    let id = preprocess_util::preprocess_node_id(id)?;
    service::modify(&id, title.trim().to_string(), sub_title.trim().to_string())
}
