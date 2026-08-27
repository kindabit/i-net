use tauri_plugin_dialog::DialogExt;

use crate::business::user_database::attachment::service;
use crate::error_code::ErrorCode;
use crate::util::preprocess_util;

/// 导出附件：弹出系统保存对话框让用户选择目标文件，然后将附件明文写入目标文件。
///
/// # 参数
/// - `app_handle`: Tauri 应用句柄（由 Tauri 自动注入），用于弹出系统对话框。
/// - `id`: 附件 id。
///
/// # 返回值
/// 导出完成时返回 `Ok(true)`；用户取消系统对话框时返回 `Ok(false)`；
/// 附件不存在或发生其他错误时返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_attachment_export(
    app_handle: tauri::AppHandle,
    id: String,
) -> Result<bool, ErrorCode> {
    let file_name = service::get(&id)?.file_name;
    let target_path = app_handle
        .dialog()
        .file()
        .set_file_name(&file_name)
        .blocking_save_file();
    match target_path {
        Some(path) => {
            preprocess(id, path.to_string())?;
            Ok(true)
        }
        None => Ok(false),
    }
}

/// `user_database_attachment_export` 的 preprocess 函数：校验 id 与 target_path，
/// 并拒绝指向应用数据目录内的目标路径后，接入 service 层的 export 函数。
pub fn preprocess(id: String, target_path: String) -> Result<(), ErrorCode> {
    let id = preprocess_util::preprocess_attachment_id(id)?;
    let target_path = preprocess_util::preprocess_file_path(target_path)?;
    crate::state::path().ensure_outside_data_directory(&target_path)?;
    service::export(&id, &target_path)
}
