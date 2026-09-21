//! 调试自动化模块的窗口级截图支持。
//!
//! 截图始终将主应用程序窗口作为一个整体捕获，包括其标题栏与边框，
//! 因此其像素与 UI 树及输入端点使用同一窗口物理坐标系。
//! DWM 阴影位于截图所依据的窗口矩形之外，因此绝不会出现在截图中。

use super::model::{AutomationError, AutomationResult};
use super::platform;
use super::AppState;

/// 将图像编码为 PNG 字节。
///
/// # 参数
/// - `image`：要编码的图像。
///
/// # 返回值
/// PNG 编码后的字节，或编码失败时的英文内部错误。
fn encode_png(image: &image::RgbaImage) -> AutomationResult<Vec<u8>> {
    let mut bytes = Vec::new();
    image
        .write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Png,
        )
        .map_err(|error| {
            AutomationError::internal(format!("failed to encode screenshot as png: {error}"))
        })?;
    Ok(bytes)
}

/// 将主窗口作为一个整体捕获并编码为 PNG。
///
/// # 参数
/// - `state`：携带主窗口句柄的共享应用程序状态。
///
/// # 返回值
/// 物理像素的 PNG 编码窗口截图，或捕获、编码失败时的英文内部错误。
pub(crate) fn screenshot(state: &AppState) -> AutomationResult<Vec<u8>> {
    let image = platform::capture_window(state.main_hwnd)?;
    encode_png(&image)
}
