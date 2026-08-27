use tauri_plugin_dialog::DialogExt;

use crate::business::user_database::template::service;
use crate::error_code::ErrorCode;

/// 导入模板数据：弹出系统文件选择对话框让用户选择源文件，
/// 读取其中的 template、template_field、dictionary 数据并替换当前数据库。
///
/// # 参数
/// - `app_handle`: Tauri 应用句柄（由 Tauri 自动注入），用于弹出系统对话框。
///
/// # 返回值
/// 导入完成时返回 `Ok(true)`；用户取消系统对话框时返回 `Ok(false)`；
/// 发生错误时返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_template_import(app_handle: tauri::AppHandle) -> Result<bool, ErrorCode> {
    let source_path = app_handle.dialog().file().blocking_pick_file();
    match source_path {
        Some(path) => {
            preprocess(path.to_string())?;
            Ok(true)
        }
        None => Ok(false),
    }
}

/// `user_database_template_import` 的 preprocess 函数：校验 source_path 后接入 service 层的 import 函数。
pub fn preprocess(source_path: String) -> Result<(), ErrorCode> {
    let source_path = crate::util::preprocess_util::preprocess_file_path(source_path)?;
    service::import(&source_path)
}
