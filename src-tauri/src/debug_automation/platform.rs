//! 调试自动化模块的窗口几何原语。
//!
//! 平台垫片将自动化模块的其余部分与用于查询窗口矩形、客户区原点及每窗口
//! DPI 的具体 Win32 调用隔离开来。在 Windows 上，所有查询都通过 `windows`
//! crate 进行；在其它平台上，主窗口几何信息取自 Tauri。

use super::model::{AutomationError, AutomationResult};
use super::AppState;

/// 为一个顶层窗口查询到的几何事实。
pub(crate) struct RawWindow {
    pub window_x: i32,
    pub window_y: i32,
    pub window_width: u32,
    pub window_height: u32,
    pub client_x: i32,
    pub client_y: i32,
    pub scale_factor: f64,
}

#[cfg(windows)]
use windows::Win32::{
    Foundation::{HWND, POINT, RECT},
    Graphics::Gdi::ClientToScreen,
    UI::{HiDpi::GetDpiForWindow, WindowsAndMessaging::GetWindowRect},
};

/// 查询某个原生句柄所对应窗口的几何事实。
///
/// # 参数
/// - `hwnd`：要检查的顶层窗口的原生句柄。
///
/// # 返回值
/// 成功时返回 [`RawWindow`]，否则返回描述哪次 Win32 查询失败的英文内部错误。
#[cfg(windows)]
fn raw_window_from_hwnd(hwnd: HWND) -> AutomationResult<RawWindow> {
    unsafe {
        let mut rect = RECT::default();
        GetWindowRect(hwnd, &mut rect).map_err(|error| {
            AutomationError::internal(format!("failed to get window rectangle: {error}"))
        })?;

        // 映射原点可得到客户区左上角的屏幕坐标。
        let mut point = POINT::default();
        ClientToScreen(hwnd, &mut point)
            .ok()
            .map_err(|error| {
                AutomationError::internal(format!(
                    "failed to convert client area origin to screen coordinates: {error}"
                ))
            })?;

        let dpi = GetDpiForWindow(hwnd);
        let scale_factor = dpi as f64 / 96.0;

        Ok(RawWindow {
            window_x: rect.left,
            window_y: rect.top,
            window_width: (rect.right - rect.left) as u32,
            window_height: (rect.bottom - rect.top) as u32,
            client_x: point.x,
            client_y: point.y,
            scale_factor,
        })
    }
}

/// 查询主应用程序窗口的几何信息。
///
/// # 参数
/// - `state`：携带主窗口句柄的共享应用程序状态。
///
/// # 返回值
/// 描述主窗口的 [`RawWindow`]，或几何查询失败时的英文内部错误。
#[cfg(windows)]
pub(crate) fn main_raw_window(state: &AppState) -> AutomationResult<RawWindow> {
    let hwnd = HWND(state.main_hwnd as *mut std::ffi::c_void);
    raw_window_from_hwnd(hwnd)
}

/// 将整个顶层窗口捕获为 RGBA 图像。
///
/// 截图通过 GDI `PrintWindow` 配合 `PW_RENDERFULLCONTENT` 完成，该调用向 DWM
/// 请求合成后的窗口表面：画面包含标题栏、边框与 WebView2 内容，
/// 而不会像普通 `WM_PRINT` 那样留下黑色矩形。位图尺寸按窗口矩形设置，
/// 因此位于该矩形之外的 DWM 阴影绝不会出现在截图中。
///
/// # 参数
/// - `hwnd`：要捕获窗口的原始原生窗口句柄。
///
/// # 返回值
/// 物理像素的窗口图像，或几何查询、`PrintWindow` 调用及像素传输失败时的英文内部错误。
#[cfg(windows)]
pub(crate) fn capture_window(hwnd: isize) -> AutomationResult<image::RgbaImage> {
    use windows::Win32::{
        Foundation::HWND,
        Graphics::Gdi::{
            BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleBitmap, CreateCompatibleDC,
            DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC, GetDIBits, ReleaseDC, SelectObject,
        },
        Storage::Xps::{PRINT_WINDOW_FLAGS, PrintWindow},
        UI::WindowsAndMessaging::{GetWindowRect, PW_RENDERFULLCONTENT},
    };

    let hwnd = HWND(hwnd as *mut std::ffi::c_void);

    unsafe {
        let mut rect = RECT::default();
        GetWindowRect(hwnd, &mut rect).map_err(|error| {
            AutomationError::internal(format!("failed to get window rectangle: {error}"))
        })?;
        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;
        if width <= 0 || height <= 0 {
            return Err(AutomationError::internal(
                "window has an empty capture rectangle",
            ));
        }

        let screen_dc = GetDC(None);
        if screen_dc.0.is_null() {
            return Err(AutomationError::internal(
                "failed to acquire the screen device context",
            ));
        }
        let memory_dc = CreateCompatibleDC(Some(screen_dc));
        if memory_dc.0.is_null() {
            ReleaseDC(None, screen_dc);
            return Err(AutomationError::internal(
                "failed to create the capture device context",
            ));
        }
        let bitmap = CreateCompatibleBitmap(screen_dc, width, height);
        if bitmap.0.is_null() {
            let _ = DeleteDC(memory_dc);
            ReleaseDC(None, screen_dc);
            return Err(AutomationError::internal(
                "failed to create the capture bitmap",
            ));
        }
        let previous = SelectObject(memory_dc, bitmap.into());

        let printed = PrintWindow(hwnd, memory_dc, PRINT_WINDOW_FLAGS(PW_RENDERFULLCONTENT));

        // 负高度请求自顶向下的 DIB，其行顺序与图像缓冲区直接一致。
        let mut info = BITMAPINFO::default();
        info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        info.bmiHeader.biWidth = width;
        info.bmiHeader.biHeight = -height;
        info.bmiHeader.biPlanes = 1;
        info.bmiHeader.biBitCount = 32;
        info.bmiHeader.biCompression = BI_RGB.0;

        let mut buffer = vec![0u8; width as usize * height as usize * 4];
        let copied_lines = if printed.as_bool() {
            GetDIBits(
                memory_dc,
                bitmap,
                0,
                height as u32,
                Some(buffer.as_mut_ptr().cast()),
                &mut info,
                DIB_RGB_COLORS,
            )
        } else {
            0
        };

        SelectObject(memory_dc, previous);
        let _ = DeleteObject(bitmap.into());
        let _ = DeleteDC(memory_dc);
        ReleaseDC(None, screen_dc);

        if !printed.as_bool() {
            return Err(AutomationError::internal(
                "PrintWindow failed to capture the window",
            ));
        }
        if copied_lines != height {
            return Err(AutomationError::internal(format!(
                "GetDIBits copied {copied_lines} of {height} window lines"
            )));
        }

        // GDI 交出的是带未使用 alpha 通道的 BGRA 像素。
        for pixel in buffer.chunks_exact_mut(4) {
            pixel.swap(0, 2);
            pixel[3] = 255;
        }

        image::RgbaImage::from_raw(width as u32, height as u32, buffer).ok_or_else(|| {
            AutomationError::internal("failed to assemble the captured window image")
        })
    }
}

/// 查询主应用程序窗口的几何信息。
///
/// # 参数
/// - `state`：携带 Tauri 应用程序句柄的共享应用程序状态。
///
/// # 返回值
/// 由 Tauri 窗口几何信息组装而成的 [`RawWindow`]，或主窗口缺失、几何查询失败时的英文内部错误。
#[cfg(not(windows))]
pub(crate) fn main_raw_window(state: &AppState) -> AutomationResult<RawWindow> {
    use tauri::Manager;

    let window = state
        .app_handle
        .get_webview_window("main")
        .ok_or_else(|| AutomationError::internal("main webview window not found"))?;

    let outer_position = window
        .outer_position()
        .map_err(|error| AutomationError::internal(format!("failed to get window position: {error}")))?;
    let client_position = window
        .inner_position()
        .map_err(|error| AutomationError::internal(format!("failed to get client position: {error}")))?;
    let outer_size = window
        .outer_size()
        .map_err(|error| AutomationError::internal(format!("failed to get window size: {error}")))?;
    let scale_factor = window
        .scale_factor()
        .map_err(|error| AutomationError::internal(format!("failed to get scale factor: {error}")))?;

    Ok(RawWindow {
        window_x: outer_position.x,
        window_y: outer_position.y,
        window_width: outer_size.width,
        window_height: outer_size.height,
        client_x: client_position.x,
        client_y: client_position.y,
        scale_factor,
    })
}

/// 将整个顶层窗口捕获为 RGBA 图像。
///
/// 非 Windows 平台仅保证可编译与服务器可启动；其截图后端尚未实现，
/// 会报告明确的错误。
///
/// # 参数
/// - `hwnd`：要捕获窗口的原始原生窗口句柄。
///
/// # 返回值
/// 始终返回说明非 Windows 平台不支持窗口捕获的英文内部错误。
#[cfg(not(windows))]
pub(crate) fn capture_window(_hwnd: isize) -> AutomationResult<image::RgbaImage> {
    Err(AutomationError::internal(
        "window capture is not supported on this platform",
    ))
}
