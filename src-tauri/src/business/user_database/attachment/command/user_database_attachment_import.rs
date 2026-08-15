use tauri_plugin_dialog::DialogExt;

use crate::business::user_database::attachment::service;
use crate::business::user_database::attachment::vo::AttachmentVO;
use crate::error_code::ErrorCode;
use crate::util::preprocess_util;

/// 导入附件：弹出系统文件选择对话框让用户选择源文件，加密后作为节点附件存储。
///
/// # 参数
/// - `app_handle`: Tauri 应用句柄（由 Tauri 自动注入），用于弹出系统对话框。
/// - `node_id`: 附件所属节点的 id。
///
/// # 返回值
/// 返回导入的附件值对象；用户取消系统对话框时返回 `Ok(None)`；
/// 发生错误时返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_attachment_import(
    app_handle: tauri::AppHandle,
    node_id: String,
) -> Result<Option<AttachmentVO>, ErrorCode> {
    let source_path = app_handle.dialog().file().blocking_pick_file();
    match source_path {
        Some(path) => preprocess(node_id, path.to_string()).map(Some),
        None => Ok(None),
    }
}

/// `user_database_attachment_import` 的 preprocess 函数：校验 node_id 与 source_path 后接入 service 层的 import 函数。
pub fn preprocess(node_id: String, source_path: String) -> Result<AttachmentVO, ErrorCode> {
    let node_id = preprocess_util::preprocess_node_id(node_id)?;
    let source_path = preprocess_util::preprocess_file_path(source_path)?;
    service::import(&node_id, &source_path)
}
