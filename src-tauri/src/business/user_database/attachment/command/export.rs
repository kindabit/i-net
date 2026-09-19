use crate::business::user_database::attachment::service;
use crate::error_code::ErrorCode;
use crate::util::preprocess_util;

/// 导出附件：将附件明文写入前端文件选择器指定的目标文件。
///
/// # 参数
/// - `id`: 附件 id。
/// - `target_path`: 目标文件路径（由前端文件选择器提供；用户取消由调用方处理）。
///
/// # 返回值
/// 导出完成时返回 `Ok(())`；附件不存在或发生其他错误时返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_attachment_export(
    id: String,
    target_path: String,
) -> Result<(), ErrorCode> {
    preprocess(id, target_path)
}

/// `user_database_attachment_export` 的 preprocess 函数：校验 id 与 target_path，
/// 并拒绝指向应用数据目录内的目标路径后，接入 service 层的 export 函数。
pub fn preprocess(id: String, target_path: String) -> Result<(), ErrorCode> {
    let id = preprocess_util::preprocess_attachment_id(id)?;
    let target_path = preprocess_util::preprocess_file_path(target_path)?;
    crate::state::path().ensure_outside_data_directory(&target_path)?;
    service::export(&id, &target_path)
}
