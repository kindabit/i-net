use crate::business::user_database::attachment::service;
use crate::error_code::ErrorCode;
use crate::util::preprocess_util;

/// 加载附件明文内容（二进制响应，前端用于预览）。
///
/// # 参数
/// - `id`: 附件 id。
///
/// # 返回值
/// 返回附件明文内容的二进制响应；若发生错误则返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_attachment_load(id: String) -> Result<tauri::ipc::Response, ErrorCode> {
    Ok(tauri::ipc::Response::new(preprocess(id)?))
}

/// `user_database_attachment_load` 的 preprocess 函数：校验 id 后接入 service 层的 load 函数，返回明文内容。
pub fn preprocess(id: String) -> Result<Vec<u8>, ErrorCode> {
    let id = preprocess_util::preprocess_attachment_id(id)?;
    service::load(&id)
}
