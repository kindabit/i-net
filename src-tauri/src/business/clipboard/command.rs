use crate::error_code::ErrorCode;
use arboard::Clipboard;

/// 清空系统剪贴板内容。
///
/// # 返回值
/// 成功返回Ok(()); 失败返回对应的ErrorCode。
#[tauri::command]
pub fn clipboard_clear() -> Result<(), ErrorCode> {
    let mut clipboard = Clipboard::new().map_err(|e| {
        tracing::error!("failed to create clipboard instance: {:?}", e);
        ErrorCode::ClipboardError { detail: e.to_string() }
    })?;
    clipboard.clear().map_err(|e| {
        tracing::error!("failed to clear clipboard: {:?}", e);
        ErrorCode::ClipboardError { detail: e.to_string() }
    })?;
    Ok(())
}
