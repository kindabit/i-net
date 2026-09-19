use crate::business::user_database::template::service;
use crate::error_code::ErrorCode;

/// 导出模板数据：将 template、template_field、dictionary 数据导出到前端文件选择器选定的目标文件。
///
/// # 参数
/// - `target_path`：目标文件路径，由前端文件选择器提供。
///
/// # 返回值
/// 导出完成时返回 `Ok(())`；发生错误时返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_template_export(target_path: String) -> Result<(), ErrorCode> {
    preprocess(target_path)?;
    Ok(())
}

/// `user_database_template_export` 的 preprocess 函数：校验 target_path 后接入 service 层的 export 函数。
pub fn preprocess(target_path: String) -> Result<(), ErrorCode> {
    let target_path = crate::util::preprocess_util::preprocess_file_path(target_path)?;
    service::export(&target_path)
}
