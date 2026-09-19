use crate::business::user_database::attachment::service;
use crate::business::user_database::attachment::vo::AttachmentVO;
use crate::error_code::ErrorCode;
use crate::util::preprocess_util;

/// 导入附件：把前端文件选择器提供的源文件加密后作为节点附件存储。
///
/// # 参数
/// - `node_id`: 附件所属节点的 id。
/// - `source_path`: 源文件路径（由前端文件选择器提供；用户取消由调用方处理）。
///
/// # 返回值
/// 返回导入的附件值对象；发生错误时返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_attachment_import(
    node_id: String,
    source_path: String,
) -> Result<AttachmentVO, ErrorCode> {
    preprocess(node_id, source_path)
}

/// `user_database_attachment_import` 的 preprocess 函数：校验 node_id 与 source_path 后接入 service 层的 import 函数。
pub fn preprocess(node_id: String, source_path: String) -> Result<AttachmentVO, ErrorCode> {
    let node_id = preprocess_util::preprocess_node_id(node_id)?;
    let source_path = preprocess_util::preprocess_file_path(source_path)?;
    service::import(&node_id, &source_path)
}
