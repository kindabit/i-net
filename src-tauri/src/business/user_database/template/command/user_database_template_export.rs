use tauri_plugin_dialog::DialogExt;

use crate::business::user_database::template::service;
use crate::error_code::ErrorCode;

/// 导出模板数据：弹出系统保存对话框让用户选择目标文件，然后将 template、template_field、dictionary 数据导出。
///
/// # 参数
/// - `app_handle`: Tauri 应用句柄（由 Tauri 自动注入），用于弹出系统对话框。
///
/// # 返回值
/// 导出完成时返回 `Ok(true)`；用户取消系统对话框时返回 `Ok(false)`；
/// 发生错误时返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_template_export(app_handle: tauri::AppHandle) -> Result<bool, ErrorCode> {
    let target_path = app_handle
        .dialog()
        .file()
        .set_file_name("templates.sqlite")
        .blocking_save_file();
    match target_path {
        Some(path) => {
            preprocess(path.to_string())?;
            Ok(true)
        }
        None => Ok(false),
    }
}

/// `user_database_template_export` 的 preprocess 函数：校验 target_path 后接入 service 层的 export 函数。
pub fn preprocess(target_path: String) -> Result<(), ErrorCode> {
    let target_path = crate::util::preprocess_util::preprocess_file_path(target_path)?;
    service::export(&target_path)
}
