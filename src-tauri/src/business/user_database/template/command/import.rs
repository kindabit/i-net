use crate::business::user_database::template::service;
use crate::error_code::ErrorCode;

/// 导入模板数据：读取前端文件选择器选定的源文件，
/// 将其中的 template、template_field、dictionary 数据替换当前数据库。
///
/// # 参数
/// - `source_path`：源文件路径，由前端文件选择器提供。
///
/// # 返回值
/// 导入完成时返回 `Ok(())`；发生错误时返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_template_import(source_path: String) -> Result<(), ErrorCode> {
    preprocess(source_path)?;
    Ok(())
}

/// `user_database_template_import` 的 preprocess 函数：校验 source_path 后接入 service 层的 import 函数。
pub fn preprocess(source_path: String) -> Result<(), ErrorCode> {
    let source_path = crate::util::preprocess_util::preprocess_file_path(source_path)?;
    service::import(&source_path)
}
