use crate::business::user_database::attachment::service;
use crate::business::user_database::attachment::vo::AttachmentVO;
use crate::error_code::ErrorCode;
use crate::util::preprocess_util;

/// 获取指定节点的附件列表（按逻辑删除标志过滤）。
///
/// # 参数
/// - `node_id`: 节点 id。
/// - `deleted`: 逻辑删除标志（true 查回收站，false 查正常附件）。
///
/// # 返回值
/// 返回附件值对象列表；若发生错误则返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_attachment_list(
    node_id: String,
    deleted: bool,
) -> Result<Vec<AttachmentVO>, ErrorCode> {
    preprocess(node_id, deleted)
}

/// `user_database_attachment_list` 的 preprocess 函数：校验 node_id 后接入 service 层的 list 函数。
pub fn preprocess(node_id: String, deleted: bool) -> Result<Vec<AttachmentVO>, ErrorCode> {
    let node_id = preprocess_util::preprocess_node_id(node_id)?;
    service::list(&node_id, deleted)
}
