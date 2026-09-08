use tauri_plugin_dialog::DialogExt;

use crate::error_code::ErrorCode;

/// 弹出系统文件选择对话框，让用户选择要导入的 KeePass2 数据库文件（.kdbx）。
///
/// # 参数
/// - `app_handle`: Tauri 应用句柄（由 Tauri 自动注入），用于弹出系统对话框。
///
/// # 返回值
/// 返回用户选择的文件路径；用户取消系统对话框时返回 `Ok(None)`；
/// 发生错误时返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_migration_pick_file(
    app_handle: tauri::AppHandle,
) -> Result<Option<String>, ErrorCode> {
    let source_path = app_handle
        .dialog()
        .file()
        .add_filter("KeePass 2.0 Database", &["kdbx"])
        .blocking_pick_file();
    Ok(source_path.map(|path| path.to_string()))
}
