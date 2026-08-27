use tauri_plugin_dialog::DialogExt;

use crate::business::user_database::export::service;
use crate::business::user_database::state;
use crate::error_code::ErrorCode;
use crate::util::preprocess_util;

/// 导出用户数据库为 markdown 文件：弹出系统保存对话框让用户选择目标文件，
/// 然后将画布、节点、字段、边导出为单个 markdown 文件。
///
/// # 参数
/// - `app_handle`: Tauri 应用句柄（由 Tauri 自动注入），用于弹出系统对话框。
/// - `mode`: 导出模式字符串，"exclude-fields" / "mask-values" / "include-values"。
/// - `locale`: 导出语言代码（由前端传入当前 i18n 语言），决定导出文件的固定文案语言。
///
/// # 返回值
/// 导出完成时返回 `Ok(true)`；用户取消系统对话框时返回 `Ok(false)`；
/// 发生错误时返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_export_export(
    app_handle: tauri::AppHandle,
    mode: String,
    locale: String,
) -> Result<bool, ErrorCode> {
    let mode = service::parse_mode(&mode)?;
    let file_name = format!("{}.md", state::metadata().name);
    let target_path = app_handle
        .dialog()
        .file()
        .set_file_name(&file_name)
        .blocking_save_file();
    match target_path {
        Some(path) => {
            preprocess(mode, locale, path.to_string())?;
            Ok(true)
        }
        None => Ok(false),
    }
}

/// `user_database_export_export` 的 preprocess 函数：校验 mode 与 target_path，
/// 并拒绝指向应用数据目录内的目标路径后，接入 service 层的 export 函数。
///
/// # 参数
/// - `mode`: 导出模式。
/// - `locale`: 导出语言代码。
/// - `target_path`: 导出目标文件路径。
///
/// # 返回值
/// 成功时返回 `Ok(())`；参数非法时返回对应的 `ErrorCode`。
pub fn preprocess(
    mode: service::ExportMode,
    locale: String,
    target_path: String,
) -> Result<(), ErrorCode> {
    let target_path = preprocess_util::preprocess_file_path(target_path)?;
    crate::state::path().ensure_outside_data_directory(&target_path)?;
    service::export(mode, &locale, &target_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 覆盖 preprocess 函数的失败路径：非法 mode 字符串通过 parse_mode 返回错误。
    #[test]
    fn test_export_preprocess_invalid_mode() {
        // parse_mode 失败路径：非法模式字符串返回 InvalidExportMode。
        assert!(matches!(
            service::parse_mode("no-such-mode"),
            Err(ErrorCode::InvalidExportMode { .. })
        ));
        // parse_mode 成功路径：三种合法模式字符串解析正确。
        assert_eq!(service::parse_mode("exclude-fields").unwrap(), service::ExportMode::ExcludeFields);
        assert_eq!(service::parse_mode("mask-values").unwrap(), service::ExportMode::MaskValues);
        assert_eq!(service::parse_mode("include-values").unwrap(), service::ExportMode::IncludeValues);
    }
}
