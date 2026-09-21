//! 调试自动化模块的原生输入合成。
//!
//! 请求携带一串按数组顺序串行执行的命令，输入在操作系统层级注入
//! （Windows 上通过 `enigo` 调用 `SendInput`），使 webview 接收到真实硬件消息
//! 并启动原生 HTML5 拖拽会话。坐标以主窗口物理像素为单位传入，与截图像素和
//! UI 树 bounds 一致，并通过加上窗口原点换算为物理屏幕坐标。
//! 注入型命令都先确保主窗口处于前台，使注入的输入无法泄漏到无关窗口。

mod config;
mod key_click;
mod key_press;
mod key_release;
mod mouse_click;
mod mouse_drag;
mod mouse_move;
mod mouse_press;
mod mouse_release;
mod mouse_scroll;
mod type_text;
mod wait;

use std::time::Duration;

use enigo::{Button, Direction, Enigo, Key, Keyboard, Mouse, Settings};

use super::model::{
    AutomationError, AutomationResult, ConfigCommand, InputCommand, InputRequest, KeyClickCommand,
    KeyPressCommand, KeyReleaseCommand, MouseButton, MouseClickCommand, MouseDragCommand,
    MouseMoveCommand, MousePressCommand, MouseReleaseCommand, MouseScrollCommand, ScrollDirection,
    TimeUnit, TypeCommand, WaitCommand,
};
use super::platform::RawWindow;
use super::{platform, AppState};

/// `mouse_move` 与 `mouse_drag` 在配置未指定速度时使用的默认移动速度（物理像素/秒）。
const DEFAULT_MOVE_SPEED: f64 = 1500.0;

/// `mouseMoveSpeed` 配置允许的最小移动速度（物理像素/秒）。
const MIN_MOVE_SPEED: f64 = 50.0;

/// `mouseMoveSpeed` 配置允许的最大移动速度（物理像素/秒）。
const MAX_MOVE_SPEED: f64 = 20000.0;

/// 单次平滑移动的时长上限。
const MAX_MOVE_DURATION: Duration = Duration::from_secs(60);

/// 平滑移动的单个插值步所覆盖的最大物理距离。
const MOVE_MAX_STEP_PIXELS: f64 = 15.0;

/// 平滑移动的最少插值步数，使短距离移动仍以原生方式呈现。
const MOVE_MIN_STEPS: u32 = 8;

/// 平滑移动的最大插值步数。
const MOVE_MAX_STEPS: u32 = 1000;

/// `mouse_click` 相邻两次点击之间的固定间隔。
const MOUSE_CLICK_GAP: Duration = Duration::from_millis(50);

/// `mouse_click` 与 `key_click` 的 `count` 上限（下限为 1）。
const MAX_CLICK_COUNT: u32 = 10;

/// `key_press`、`key_release` 与 `key_click` 的按键数组长度上限（下限为 1）。
const MAX_KEY_ARRAY: usize = 8;

/// 单次请求的命令队列长度上限（下限为 1）。
const MAX_COMMANDS: usize = 256;

/// `mouse_scroll` 的 `amount` 上限（下限为 1）。
const MAX_SCROLL_AMOUNT: u32 = 1000;

/// `wait` 命令允许等待的最长时长。
const MAX_WAIT: Duration = Duration::from_secs(60);

/// `mouseClickSpan` 配置允许的最大毫秒值。
const MAX_MOUSE_CLICK_SPAN_MS: u64 = 5000;

/// `keyClickInTurnInterval` 配置允许的最大毫秒值。
const MAX_KEY_CLICK_IN_TURN_MS: u64 = 1000;

/// `keyClickCrossTurnInterval` 配置允许的最大毫秒值。
const MAX_KEY_CLICK_CROSS_TURN_MS: u64 = 5000;

/// `keyClickSpan` 配置允许的最大毫秒值。
const MAX_KEY_CLICK_SPAN_MS: u64 = 5000;

/// `commandInterval` 配置允许的最大毫秒值。
const MAX_COMMAND_INTERVAL_MS: u64 = 5000;

/// 拖拽按下按键之前抖动的距离。
const DRAG_JITTER_PIXELS: i32 = 2;

/// 光标到达拖拽目标与释放按键之间的延迟。
const DRAG_SETTLE_DELAY: Duration = Duration::from_millis(80);

/// 滚动手势两个独立滚轮刻度之间的延迟。
const SCROLL_NOTCH_DELAY: Duration = Duration::from_millis(10);

/// 前台窗口变化后给予系统的延迟。
const FOREGROUND_SETTLE_DELAY: Duration = Duration::from_millis(50);

/// 为一次队列执行创建一个 enigo 实例。
///
/// # 返回值
/// 返回可用的原生输入句柄；当无法建立平台输入连接时，返回英文内部错误。
fn new_enigo() -> AutomationResult<Enigo> {
    Enigo::new(&Settings::default()).map_err(|error| {
        AutomationError::internal(format!("failed to initialize native input injection: {error}"))
    })
}

/// 将有限浮点值舍入为最接近的 `i32`。
///
/// 标准库未提供 `TryFrom<f64> for i32`，因此此处显式进行范围检查；
/// 超出 `i32` 范围的值会被拒绝，而不是依赖饱和转换。
///
/// # 参数
/// - `value`：要舍入的值。
///
/// # 返回值
/// 当舍入后的值能容纳于 `i32` 时返回 `Some(rounded)`，否则返回 `None`。
fn round_to_i32(value: f64) -> Option<i32> {
    let rounded = value.round();
    if (f64::from(i32::MIN)..=f64::from(i32::MAX)).contains(&rounded) {
        Some(rounded as i32)
    } else {
        None
    }
}

/// 将以主窗口物理像素表示的点换算为物理屏幕坐标。
///
/// API 坐标系的原点位于整个窗口的左上角，这也是截图像素和 UI 树 bounds 的原点，
/// 因此该换算只是按窗口原点进行的纯平移，不涉及 DPI 缩放。
/// 该值会舍入到最接近的物理像素。
///
/// # 参数
/// - `main`：主应用程序窗口的几何信息。
/// - `x`：以主窗口物理像素表示的水平坐标。
/// - `y`：以主窗口物理像素表示的垂直坐标。
///
/// # 返回值
/// 返回物理屏幕坐标对；当坐标不是有限值或无法表示时，返回英文 bad-request 错误。
fn resolve_screen_point(main: &RawWindow, x: f64, y: f64) -> AutomationResult<(i32, i32)> {
    if !x.is_finite() || !y.is_finite() {
        return Err(AutomationError::bad_request(
            "coordinates must be finite numbers",
        ));
    }

    let offset_x = round_to_i32(x)
        .ok_or_else(|| AutomationError::bad_request("x coordinate is out of range"))?;
    let offset_y = round_to_i32(y)
        .ok_or_else(|| AutomationError::bad_request("y coordinate is out of range"))?;
    let screen_x = main
        .window_x
        .checked_add(offset_x)
        .ok_or_else(|| AutomationError::bad_request("x coordinate is out of range"))?;
    let screen_y = main
        .window_y
        .checked_add(offset_y)
        .ok_or_else(|| AutomationError::bad_request("y coordinate is out of range"))?;

    Ok((screen_x, screen_y))
}

/// 将 enigo 输入失败包装为英文内部错误。
///
/// # 参数
/// - `context`：失败注入步骤的英文描述。
/// - `error`：底层的 enigo 错误。
///
/// # 返回值
/// 返回携带上下文和平台错误文本的自动化错误。
fn input_error(context: &str, error: enigo::InputError) -> AutomationError {
    AutomationError::internal(format!("{context}: {error}"))
}

/// 将本进程的窗口置为前台窗口。
///
/// 当前台窗口已属于本进程时不执行任何操作。否则激活主窗口：
/// 还原最小化的窗口，临时附加调用线程与当前前台窗口的输入队列，
/// 并注入一次 ALT 键敲击，使系统将本进程视为已接收到用户输入，
/// 这正是当另一个应用程序当前占有前台时解锁 `SetForegroundWindow` 所需的条件。
///
/// # 参数
/// - `state`：携带主窗口句柄的共享应用程序状态。
///
/// # 返回值
/// 当本进程处于前台时返回 `Ok(())`；
/// 当系统拒绝激活时，返回英文内部错误。
#[cfg(windows)]
fn ensure_foreground(state: &AppState) -> AutomationResult<()> {
    use windows::Win32::{
        Foundation::HWND,
        System::Threading::{AttachThreadInput, GetCurrentThreadId},
        UI::{
            Input::KeyboardAndMouse::{
                SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS,
                KEYEVENTF_KEYUP, VK_MENU,
            },
            WindowsAndMessaging::{
                GetForegroundWindow, GetWindowThreadProcessId, IsIconic, SetForegroundWindow,
                ShowWindow, SW_RESTORE,
            },
        },
    };

    unsafe {
        let foreground = GetForegroundWindow();
        let mut foreground_pid = 0u32;
        GetWindowThreadProcessId(foreground, Some(&mut foreground_pid));
        if foreground_pid == std::process::id() {
            return Ok(());
        }

        let main = HWND(state.main_hwnd as *mut std::ffi::c_void);
        if IsIconic(main).as_bool() {
            let _ = ShowWindow(main, SW_RESTORE);
        }

        let current_thread = GetCurrentThreadId();
        let foreground_thread = GetWindowThreadProcessId(foreground, None);
        let attached = foreground_thread != 0
            && foreground_thread != current_thread
            && AttachThreadInput(current_thread, foreground_thread, true).as_bool();

        let mut activated =
            SetForegroundWindow(main).as_bool() && GetForegroundWindow().0 as isize == state.main_hwnd;
        if !activated {
            // 单独的一次 ALT 敲击会将本进程标记为已接收到用户输入；
            // 随后 Windows 便允许它取得前台。
            let alt_down = INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VK_MENU,
                        wScan: 0,
                        dwFlags: KEYBD_EVENT_FLAGS(0),
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            };
            let alt_up = INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VK_MENU,
                        wScan: 0,
                        dwFlags: KEYEVENTF_KEYUP,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            };
            if SendInput(&[alt_down, alt_up], std::mem::size_of::<INPUT>() as i32) == 2 {
                activated = SetForegroundWindow(main).as_bool()
                    && GetForegroundWindow().0 as isize == state.main_hwnd;
            }
        }

        if attached {
            let _ = AttachThreadInput(current_thread, foreground_thread, false);
        }

        if activated {
            std::thread::sleep(FOREGROUND_SETTLE_DELAY);
            Ok(())
        } else {
            Err(AutomationError::internal(
                "failed to bring the application window to the foreground",
            ))
        }
    }
}

/// 将本进程的窗口置为前台窗口。
///
/// 非 Windows 平台依赖窗口管理器以及这样一个事实：
/// 自动化 API 的使用时机是应用程序已占有前台；此平台不会尝试激活窗口。
///
/// # 参数
/// - `state`：携带主窗口句柄的共享应用程序状态。
///
/// # 返回值
/// 在非 Windows 平台上始终返回 `Ok(())`。
#[cfg(not(windows))]
fn ensure_foreground(_state: &AppState) -> AutomationResult<()> {
    Ok(())
}

/// 将鼠标光标移动到物理屏幕坐标。
///
/// Windows 使用带 `MOUSEEVENTF_VIRTUALDESK` 的 `SendInput`，
/// 因此整个虚拟桌面都可寻址，包括原点为负的副显示器；
/// enigo 自身的绝对映射只覆盖主显示器。
/// 其余平台使用 enigo 的绝对移动。
///
/// # 参数
/// - `enigo`：整个队列共用的原生输入句柄。
/// - `point`：目标物理屏幕坐标。
///
/// # 返回值
/// 注入移动后返回 `Ok(())`；当平台拒绝时返回英文内部错误。
#[cfg(windows)]
fn move_pointer(enigo: &mut Enigo, point: (i32, i32)) -> AutomationResult<()> {
    use windows::Win32::UI::{
        Input::KeyboardAndMouse::{
            SendInput, INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_MOVE,
            MOUSEEVENTF_VIRTUALDESK, MOUSEINPUT,
        },
        WindowsAndMessaging::{
            GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN,
            SM_YVIRTUALSCREEN,
        },
    };

    unsafe {
        let virtual_x = GetSystemMetrics(SM_XVIRTUALSCREEN);
        let virtual_y = GetSystemMetrics(SM_YVIRTUALSCREEN);
        let virtual_width = GetSystemMetrics(SM_CXVIRTUALSCREEN);
        let virtual_height = GetSystemMetrics(SM_CYVIRTUALSCREEN);
        if virtual_width <= 1 || virtual_height <= 1 {
            return Err(AutomationError::internal(
                "failed to query the virtual desktop geometry",
            ));
        }

        // 绝对鼠标坐标空间在虚拟桌面上为 0..65535；
        // 加半个除数用于舍入，同时不会越过边缘。
        let width = i64::from(virtual_width - 1);
        let height = i64::from(virtual_height - 1);
        let normalized_x = (i64::from(point.0 - virtual_x) * 65535 + width / 2) / width;
        let normalized_y = (i64::from(point.1 - virtual_y) * 65535 + height / 2) / height;
        if !(0..=65535).contains(&normalized_x) || !(0..=65535).contains(&normalized_y) {
            return Err(AutomationError::bad_request(
                "point is outside of the virtual desktop",
            ));
        }

        let input = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: normalized_x as i32,
                    dy: normalized_y as i32,
                    mouseData: 0,
                    dwFlags: MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK,
                    time: 0,
                    dwExtraInfo: enigo::EVENT_MARKER as usize,
                },
            },
        };
        if SendInput(&[input], std::mem::size_of::<INPUT>() as i32) != 1 {
            let error = windows::Win32::Foundation::GetLastError();
            return Err(AutomationError::internal(format!(
                "failed to inject mouse move: {error:?}"
            )));
        }
    }

    let _ = enigo;
    Ok(())
}

/// 将鼠标光标移动到物理屏幕坐标。
///
/// # 参数
/// - `enigo`：整个队列共用的原生输入句柄。
/// - `point`：目标物理屏幕坐标。
///
/// # 返回值
/// 注入移动后返回 `Ok(())`；当平台拒绝时返回英文内部错误。
#[cfg(not(windows))]
fn move_pointer(enigo: &mut Enigo, point: (i32, i32)) -> AutomationResult<()> {
    enigo
        .move_mouse(point.0, point.1, enigo::Coordinate::Abs)
        .map_err(|error| input_error("failed to inject mouse move", error))
}

/// 将 API 鼠标按键映射为 enigo 按键。
///
/// # 参数
/// - `button`：API 请求的按键。
///
/// # 返回值
/// 返回匹配的 enigo 按键。
fn map_button(button: MouseButton) -> Button {
    match button {
        MouseButton::Left => Button::Left,
        MouseButton::Right => Button::Right,
    }
}

/// 将 API 按键名映射为 enigo 按键。
///
/// 单个字符通过 `Key::Unicode` 映射，因此它们遵循当前键盘布局；
/// 具名按键以不区分大小写的方式匹配，并支持自动化 API 的常见别名。
///
/// # 参数
/// - `name`：请求中的按键名，例如 `Control`、`a` 或 `PageUp`。
///
/// # 返回值
/// 返回匹配的 enigo 按键；当该名称在此平台上不受支持时，
/// 返回英文 bad-request 错误。
fn key_from_name(name: &str) -> AutomationResult<Key> {
    let trimmed = name.trim();
    let mut characters = trimmed.chars();
    if let (Some(character), None) = (characters.next(), characters.next()) {
        return Ok(Key::Unicode(character));
    }

    let key = match trimmed.to_ascii_lowercase().as_str() {
        "alt" | "option" => Key::Alt,
        "backspace" | "back" => Key::Backspace,
        "capslock" | "caps" => Key::CapsLock,
        "control" | "ctrl" | "controlleft" | "lcontrol" | "lctrl" => Key::LControl,
        "controlright" | "rcontrol" | "rctrl" => Key::RControl,
        "delete" | "del" => Key::Delete,
        "down" | "downarrow" | "arrowdown" => Key::DownArrow,
        "end" => Key::End,
        "enter" | "return" => Key::Return,
        "escape" | "esc" => Key::Escape,
        "f1" => Key::F1,
        "f2" => Key::F2,
        "f3" => Key::F3,
        "f4" => Key::F4,
        "f5" => Key::F5,
        "f6" => Key::F6,
        "f7" => Key::F7,
        "f8" => Key::F8,
        "f9" => Key::F9,
        "f10" => Key::F10,
        "f11" => Key::F11,
        "f12" => Key::F12,
        "f13" => Key::F13,
        "f14" => Key::F14,
        "f15" => Key::F15,
        "f16" => Key::F16,
        "f17" => Key::F17,
        "f18" => Key::F18,
        "f19" => Key::F19,
        "f20" => Key::F20,
        "help" => Key::Help,
        "home" => Key::Home,
        "insert" | "ins" => Key::Insert,
        "left" | "leftarrow" | "arrowleft" => Key::LeftArrow,
        "meta" | "super" | "win" | "windows" | "cmd" | "command" => Key::Meta,
        "pagedown" | "pgdn" => Key::PageDown,
        "pageup" | "pgup" => Key::PageUp,
        "right" | "rightarrow" | "arrowright" => Key::RightArrow,
        "shift" | "shiftleft" | "lshift" => Key::LShift,
        "shiftright" | "rshift" => Key::RShift,
        "space" | "spacebar" => Key::Space,
        "tab" => Key::Tab,
        "up" | "uparrow" | "arrowup" => Key::UpArrow,
        "add" | "plus" => Key::Add,
        "subtract" | "minus" => Key::Subtract,
        "multiply" => Key::Multiply,
        "divide" => Key::Divide,
        "decimal" => Key::Decimal,
        "medianexttrack" | "nexttrack" => Key::MediaNextTrack,
        "mediaprevtrack" | "prevtrack" => Key::MediaPrevTrack,
        "mediaplaypause" | "playpause" => Key::MediaPlayPause,
        "volumedown" => Key::VolumeDown,
        "volumeup" => Key::VolumeUp,
        "volumemute" | "mute" => Key::VolumeMute,
        "numpad0" => Key::Numpad0,
        "numpad1" => Key::Numpad1,
        "numpad2" => Key::Numpad2,
        "numpad3" => Key::Numpad3,
        "numpad4" => Key::Numpad4,
        "numpad5" => Key::Numpad5,
        "numpad6" => Key::Numpad6,
        "numpad7" => Key::Numpad7,
        "numpad8" => Key::Numpad8,
        "numpad9" => Key::Numpad9,
        _ => {
            return Err(AutomationError::bad_request(format!(
                "unsupported key name: {name}"
            )))
        }
    };
    Ok(key)
}

/// 队列执行期间的运行期配置，由 `config` 命令按字段覆盖。
#[derive(Debug, Clone, Copy)]
struct CommandConfig {
    /// `mouse_click` 按下与松开之间的时长。
    mouse_click_span: Duration,
    /// `mouse_move` 与 `mouse_drag` 的直线移动速度（物理像素/秒）。
    mouse_move_speed: f64,
    /// `key_click` 一轮序列内相邻按键按下或松开之间的间隔。
    key_click_in_turn_interval: Duration,
    /// `key_click` 相邻两轮之间的间隔。
    key_click_cross_turn_interval: Duration,
    /// `key_click` 一轮中全部按键按下后保持的时长。
    key_click_span: Duration,
    /// 相邻两条命令之间的额外间隔。
    command_interval: Duration,
}

impl Default for CommandConfig {
    fn default() -> Self {
        Self {
            mouse_click_span: Duration::from_millis(50),
            mouse_move_speed: DEFAULT_MOVE_SPEED,
            key_click_in_turn_interval: Duration::from_millis(10),
            key_click_cross_turn_interval: Duration::from_millis(50),
            key_click_span: Duration::from_millis(50),
            command_interval: Duration::ZERO,
        }
    }
}

/// 校验毫秒值不超过给定上限并换算为 [`Duration`]。
///
/// # 参数
/// - `value`：请求中的毫秒值。
/// - `max`：该字段允许的最大毫秒值。
/// - `field`：用于错误消息的、与查询无关的字段名。
///
/// # 返回值
/// 校验通过时返回换算后的时长；越界时返回英文 bad-request 错误。
fn validate_millis(value: u64, max: u64, field: &str) -> AutomationResult<Duration> {
    if value > max {
        return Err(AutomationError::bad_request(format!(
            "{field} must not exceed {max} milliseconds"
        )));
    }
    Ok(Duration::from_millis(value))
}

/// 本次请求中已按下且尚未释放的鼠标按键与键盘按键。
#[derive(Default)]
struct PressedState {
    /// 左键是否处于按下状态。
    left_pressed: bool,
    /// 右键是否处于按下状态。
    right_pressed: bool,
    /// 已按下且尚未释放的键盘按键，按按下先后顺序排列。
    keys: Vec<Key>,
}

impl PressedState {
    /// 记录一次鼠标按下；`Button::Left` 与 `Button::Right` 之外的按键被忽略。
    ///
    /// # 参数
    /// - `button`：被按下的 enigo 鼠标按键。
    fn press_mouse(&mut self, button: Button) {
        match button {
            Button::Left => self.left_pressed = true,
            Button::Right => self.right_pressed = true,
            _ => {}
        }
    }

    /// 记录一次鼠标释放；`Button::Left` 与 `Button::Right` 之外的按键被忽略。
    ///
    /// # 参数
    /// - `button`：被释放的 enigo 鼠标按键。
    fn release_mouse(&mut self, button: Button) {
        match button {
            Button::Left => self.left_pressed = false,
            Button::Right => self.right_pressed = false,
            _ => {}
        }
    }

    /// 记录一次键盘按下。
    ///
    /// # 参数
    /// - `key`：被按下的 enigo 按键。
    fn press_key(&mut self, key: Key) {
        self.keys.push(key);
    }

    /// 记录一次键盘释放，移除最后一个匹配的已按下按键。
    ///
    /// # 参数
    /// - `key`：被释放的 enigo 按键。
    fn release_key(&mut self, key: Key) {
        if let Some(index) = self.keys.iter().rposition(|pressed| *pressed == key) {
            self.keys.remove(index);
        }
    }

    /// 尽力释放所有仍处于按下状态的按键。
    ///
    /// 记录为按下的鼠标按键直接发送释放，键盘按键按后进先出逐个发送释放，
    /// 所有注入错误都被忽略；状态在发送后清空。
    ///
    /// # 参数
    /// - `enigo`：整个队列共用的原生输入句柄。
    fn release_all(&mut self, enigo: &mut Enigo) {
        if self.left_pressed {
            let _ = enigo.button(Button::Left, Direction::Release);
            self.left_pressed = false;
        }
        if self.right_pressed {
            let _ = enigo.button(Button::Right, Direction::Release);
            self.right_pressed = false;
        }
        while let Some(key) = self.keys.pop() {
            let _ = enigo.key(key, Direction::Release);
        }
    }
}

/// 校验命令队列的长度。
///
/// # 参数
/// - `request`：待执行的输入请求。
///
/// # 返回值
/// 队列长度在 1..=256 之内时返回 `Ok(())`，否则返回英文 bad-request 错误。
fn validate_queue(request: &InputRequest) -> AutomationResult<()> {
    if request.commands.is_empty() {
        return Err(AutomationError::bad_request("commands must not be empty"));
    }
    if request.commands.len() > MAX_COMMANDS {
        return Err(AutomationError::bad_request(format!(
            "commands must not contain more than {MAX_COMMANDS} commands"
        )));
    }
    Ok(())
}

/// 串行执行请求中的输入命令队列。
///
/// 整个队列共用一个原生输入实例。相邻两条命令之间按配置插入额外间隔，
/// 首条命令之前不插入。任一命令失败时，先尽力释放仍处于按下状态的
/// 鼠标按键与键盘按键，再把失败命令的 0 基索引与命令名包装进错误消息后返回。
///
/// # 参数
/// - `state`：共享应用程序状态。
/// - `request`：按数组顺序串行执行的命令列表。
///
/// # 返回值
/// 队列全部执行成功后返回 `Ok(())`；队列为空、超长或任一命令失败时，
/// 返回英文错误。
pub(crate) fn execute(state: &AppState, request: &InputRequest) -> AutomationResult<()> {
    validate_queue(request)?;

    let mut enigo = new_enigo()?;
    let mut command_config = CommandConfig::default();
    let mut pressed = PressedState::default();

    for (index, command) in request.commands.iter().enumerate() {
        if index > 0 {
            std::thread::sleep(command_config.command_interval);
        }
        if let Err(error) = execute_command(state, &mut enigo, &mut command_config, &mut pressed, command) {
            // 释放失败不覆盖原始错误，原始错误才是队列停止的原因。
            pressed.release_all(&mut enigo);
            return Err(AutomationError::new(
                error.code(),
                format!(
                    "commands[{index}] ({}): {}",
                    command.kind(),
                    error.message()
                ),
            ));
        }
    }
    Ok(())
}

/// 执行队列中的单条命令。
///
/// 除 `wait` 与 `config` 外的注入型命令都会在执行注入前确保主窗口处于前台。
///
/// # 参数
/// - `state`：共享应用程序状态。
/// - `enigo`：整个队列共用的原生输入句柄。
/// - `command_config`：本次请求的运行期配置，`config` 命令会就地修改它。
/// - `pressed`：本次请求的按下状态追踪。
/// - `command`：要执行的命令。
///
/// # 返回值
/// 命令执行成功后返回 `Ok(())`；参数非法或原生注入失败时返回英文错误。
fn execute_command(
    state: &AppState,
    enigo: &mut Enigo,
    command_config: &mut CommandConfig,
    pressed: &mut PressedState,
    command: &InputCommand,
) -> AutomationResult<()> {
    match command {
        InputCommand::MouseMove(command) => {
            mouse_move::execute(state, enigo, command, command_config.mouse_move_speed)
        }
        InputCommand::MousePress(command) => mouse_press::execute(state, enigo, pressed, command),
        InputCommand::MouseRelease(command) => {
            mouse_release::execute(state, enigo, pressed, command)
        }
        InputCommand::MouseClick(command) => {
            mouse_click::execute(state, enigo, pressed, command, command_config.mouse_click_span)
        }
        InputCommand::MouseScroll(command) => mouse_scroll::execute(state, enigo, command),
        InputCommand::MouseDrag(command) => {
            mouse_drag::execute(state, enigo, pressed, command, command_config.mouse_move_speed)
        }
        InputCommand::KeyPress(command) => key_press::execute(state, enigo, pressed, command),
        InputCommand::KeyRelease(command) => key_release::execute(state, enigo, pressed, command),
        InputCommand::KeyClick(command) => {
            key_click::execute(state, enigo, pressed, command, command_config)
        }
        InputCommand::Type(command) => type_text::execute(state, enigo, command),
        InputCommand::Wait(command) => wait::execute(command),
        InputCommand::Config(command) => config::execute(command_config, command),
    }
}

/// 把 `wait` 命令的数值与单位换算为等待时长并校验上限。
///
/// # 参数
/// - `command`：时长的数值与单位。
///
/// # 返回值
/// 返回换算后的时长；超过 60 秒上限时返回英文 bad-request 错误。
fn wait_duration(command: &WaitCommand) -> AutomationResult<Duration> {
    let duration = match command.unit {
        TimeUnit::Ms => Duration::from_millis(command.duration),
        TimeUnit::S => Duration::from_secs(command.duration),
    };
    if duration > MAX_WAIT {
        return Err(AutomationError::bad_request(
            "wait must not exceed 60 seconds",
        ));
    }
    Ok(duration)
}

/// 按数组顺序解析按键名数组，并校验其长度。
///
/// # 参数
/// - `names`：请求中的按键名数组。
///
/// # 返回值
/// 返回按数组顺序排列的 enigo 按键；数组为空、超长或含未知按键名时
/// 返回英文 bad-request 错误。
fn parse_key_names(names: &[String]) -> AutomationResult<Vec<Key>> {
    if names.is_empty() {
        return Err(AutomationError::bad_request("keys must not be empty"));
    }
    if names.len() > MAX_KEY_ARRAY {
        return Err(AutomationError::bad_request(format!(
            "keys must not contain more than {MAX_KEY_ARRAY} keys"
        )));
    }
    names.iter().map(|name| key_from_name(name)).collect()
}

/// 校验按键数组不包含重复按键。
///
/// # 参数
/// - `keys`：已解析的 enigo 按键数组。
///
/// # 返回值
/// 所有按键互不相同时返回 `Ok(())`，否则返回英文 bad-request 错误。
fn ensure_unique_keys(keys: &[Key]) -> AutomationResult<()> {
    for (index, key) in keys.iter().enumerate() {
        if keys[..index].contains(key) {
            return Err(AutomationError::bad_request(
                "keys must not contain duplicates",
            ));
        }
    }
    Ok(())
}

/// 校验点击次数在允许范围内。
///
/// # 参数
/// - `count`：请求中的点击次数。
///
/// # 返回值
/// 次数在 1..=10 之内时返回 `Ok(())`，否则返回英文 bad-request 错误。
fn validate_click_count(count: u32) -> AutomationResult<()> {
    if !(1..=MAX_CLICK_COUNT).contains(&count) {
        return Err(AutomationError::bad_request(format!(
            "count must be within 1..={MAX_CLICK_COUNT}"
        )));
    }
    Ok(())
}

/// 校验滚轮刻度数在允许范围内。
///
/// # 参数
/// - `amount`：请求中的滚轮刻度数。
///
/// # 返回值
/// 刻度数在 1..=1000 之内时返回 `Ok(())`，否则返回英文 bad-request 错误。
fn validate_scroll_amount(amount: u32) -> AutomationResult<()> {
    if !(1..=MAX_SCROLL_AMOUNT).contains(&amount) {
        return Err(AutomationError::bad_request(format!(
            "amount must be within 1..={MAX_SCROLL_AMOUNT}"
        )));
    }
    Ok(())
}

/// 以给定速度把光标沿直线从 from 平滑移动到 to（物理屏幕坐标）。
///
/// # 参数
/// - `enigo`：整个队列共用的原生输入句柄。
/// - `from`：以物理屏幕坐标表示的起点。
/// - `to`：以物理屏幕坐标表示的终点。
/// - `speed`：物理像素每秒的移动速度。
///
/// # 返回值
/// 移动完成后返回 `Ok(())`；速度非法或预计时长超过上限时返回英文
/// bad-request 错误，原生注入失败时返回英文内部错误。
fn move_smooth(enigo: &mut Enigo, from: (i32, i32), to: (i32, i32), speed: f64) -> AutomationResult<()> {
    let delta_x = f64::from(to.0) - f64::from(from.0);
    let delta_y = f64::from(to.1) - f64::from(from.1);
    let distance = delta_x.hypot(delta_y);
    if distance == 0.0 {
        return Ok(());
    }

    // 配置校验已经拦截非法速度，这里保留防御性检查。
    if !speed.is_finite() || speed <= 0.0 {
        return Err(AutomationError::bad_request(
            "movement speed must be a positive finite number",
        ));
    }
    if distance / speed > MAX_MOVE_DURATION.as_secs_f64() {
        return Err(AutomationError::bad_request(
            "movement would take longer than 60 seconds at the configured speed",
        ));
    }

    // 单步距离上限决定步数下限；步数上下限保证插值粒度与注入次数都受控。
    let steps = ((distance / MOVE_MAX_STEP_PIXELS).ceil() as u32)
        .clamp(MOVE_MIN_STEPS, MOVE_MAX_STEPS);
    let step_delay = Duration::from_secs_f64(MOVE_MAX_STEP_PIXELS / speed);

    for step in 1..=steps {
        let progress = f64::from(step) / f64::from(steps);
        let x = from
            .0
            .saturating_add((delta_x * progress).round() as i32);
        let y = from
            .1
            .saturating_add((delta_y * progress).round() as i32);
        move_pointer(enigo, (x, y))?;
        if step < steps {
            std::thread::sleep(step_delay);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 构建具有固定尺寸和给定原点提示信息的 raw window。
    ///
    /// # 参数
    /// - `window`：以物理屏幕坐标表示的完整窗口原点。
    /// - `client`：以物理屏幕坐标表示的客户区原点。
    /// - `scale_factor`：窗口的 DPI 缩放比例。
    ///
    /// # 返回值
    /// 返回适用于坐标换算测试的 [`RawWindow`]。
    fn raw_window(window: (i32, i32), client: (i32, i32), scale_factor: f64) -> RawWindow {
        RawWindow {
            window_x: window.0,
            window_y: window.1,
            window_width: 800,
            window_height: 600,
            client_x: client.0,
            client_y: client.1,
            scale_factor,
        }
    }

    /// 构建恰好装满指定条数 `wait` 命令的请求。
    ///
    /// # 参数
    /// - `count`：要生成的命令条数。
    ///
    /// # 返回值
    /// 由 `count` 条零时长 `wait` 命令组成的输入请求。
    fn wait_request(count: usize) -> InputRequest {
        InputRequest {
            commands: (0..count)
                .map(|_| {
                    InputCommand::Wait(WaitCommand {
                        duration: 0,
                        unit: TimeUnit::Ms,
                    })
                })
                .collect(),
        }
    }

    #[test]
    fn window_coordinates_are_translated_to_screen_coordinates() {
        let main = raw_window((100, 50), (108, 82), 2.0);
        let point = resolve_screen_point(&main, 10.25, 20.5);
        assert_eq!(point.unwrap(), (110, 71));
    }

    #[test]
    fn non_finite_coordinates_are_rejected() {
        let main = raw_window((0, 0), (8, 32), 1.0);
        for (x, y) in [(f64::NAN, 0.0), (0.0, f64::INFINITY)] {
            let error = resolve_screen_point(&main, x, y).unwrap_err();
            assert!(error.to_string().starts_with("BadRequest"));
        }
    }

    #[test]
    fn astronomically_large_coordinates_are_rejected() {
        let main = raw_window((0, 0), (8, 32), 1.0);
        let error = resolve_screen_point(&main, 1.0e30, 0.0).unwrap_err();
        assert!(error.to_string().starts_with("BadRequest"));
    }

    #[test]
    fn key_names_map_to_enigo_keys() {
        assert_eq!(key_from_name("Control").unwrap(), Key::LControl);
        assert_eq!(key_from_name("ctrl").unwrap(), Key::LControl);
        assert_eq!(key_from_name("a").unwrap(), Key::Unicode('a'));
        assert_eq!(key_from_name("A").unwrap(), Key::Unicode('A'));
        assert_eq!(key_from_name("1").unwrap(), Key::Unicode('1'));
        assert_eq!(key_from_name("Enter").unwrap(), Key::Return);
        assert_eq!(key_from_name("Escape").unwrap(), Key::Escape);
        assert_eq!(key_from_name("PageUp").unwrap(), Key::PageUp);
        assert_eq!(key_from_name("F12").unwrap(), Key::F12);
        assert_eq!(key_from_name("Meta").unwrap(), Key::Meta);
        assert_eq!(key_from_name("Numpad5").unwrap(), Key::Numpad5);
        assert_eq!(key_from_name("ArrowDown").unwrap(), Key::DownArrow);
        assert_eq!(key_from_name("ArrowUp").unwrap(), Key::UpArrow);
        assert_eq!(key_from_name("ArrowLeft").unwrap(), Key::LeftArrow);
        assert_eq!(key_from_name("ArrowRight").unwrap(), Key::RightArrow);
        assert_eq!(key_from_name("Insert").unwrap(), Key::Insert);
    }

    #[test]
    fn whitespace_around_a_single_character_is_ignored() {
        assert_eq!(key_from_name(" x ").unwrap(), Key::Unicode('x'));
    }

    #[test]
    fn unsupported_key_names_are_rejected() {
        for name in ["nonsense", "", "F24", "MouseLeft"] {
            let error = key_from_name(name).unwrap_err();
            assert!(error.to_string().starts_with("BadRequest"));
        }
    }

    #[test]
    fn empty_or_oversized_command_queues_are_rejected() {
        // 空队列被拒绝。
        let error = validate_queue(&wait_request(0)).unwrap_err();
        assert_eq!(error.message(), "commands must not be empty");

        // 超过 256 条的队列被拒绝。
        let error = validate_queue(&wait_request(MAX_COMMANDS + 1)).unwrap_err();
        assert_eq!(
            error.message(),
            "commands must not contain more than 256 commands"
        );

        // 恰好 256 条的队列必须被接受。
        assert!(validate_queue(&wait_request(MAX_COMMANDS)).is_ok());
    }

    #[test]
    fn key_arrays_are_parsed_and_validated() {
        // 合法数组必须按数组顺序映射为 enigo 按键。
        let keys = parse_key_names(&["Control".to_string(), "a".to_string()]).unwrap();
        assert_eq!(keys, vec![Key::LControl, Key::Unicode('a')]);

        // 空数组被拒绝。
        let error = parse_key_names(&[]).unwrap_err();
        assert_eq!(error.message(), "keys must not be empty");

        // 超过 8 个按键的数组被拒绝。
        let nine: Vec<String> = (1..=9).map(|index| index.to_string()).collect();
        let error = parse_key_names(&nine).unwrap_err();
        assert_eq!(error.message(), "keys must not contain more than 8 keys");

        // 未知按键名被拒绝。
        let error = parse_key_names(&["nonsense".to_string()]).unwrap_err();
        assert!(error.to_string().starts_with("BadRequest"));
    }

    #[test]
    fn duplicate_keys_in_key_click_are_rejected() {
        // 互不相同的按键数组被接受。
        assert!(ensure_unique_keys(&[Key::LControl, Key::Unicode('a')]).is_ok());

        // 包含重复按键的数组被拒绝。
        let error = ensure_unique_keys(&[Key::Unicode('a'), Key::Unicode('a')]).unwrap_err();
        assert!(error.to_string().starts_with("BadRequest"));
        assert_eq!(error.message(), "keys must not contain duplicates");

        // 大小写不同的单字符在 enigo 中是不同按键，因此不算重复。
        assert!(ensure_unique_keys(&[Key::Unicode('a'), Key::Unicode('A')]).is_ok());
    }

    #[test]
    fn click_counts_and_scroll_amounts_are_validated() {
        // 点击次数 1 与 10 合法，0 与 11 非法。
        for count in [1, MAX_CLICK_COUNT] {
            assert!(validate_click_count(count).is_ok());
        }
        for count in [0, MAX_CLICK_COUNT + 1] {
            let error = validate_click_count(count).unwrap_err();
            assert_eq!(error.message(), "count must be within 1..=10");
        }

        // 滚轮刻度数 1 与 1000 合法，0 与 1001 非法。
        for amount in [1, MAX_SCROLL_AMOUNT] {
            assert!(validate_scroll_amount(amount).is_ok());
        }
        for amount in [0, MAX_SCROLL_AMOUNT + 1] {
            let error = validate_scroll_amount(amount).unwrap_err();
            assert_eq!(error.message(), "amount must be within 1..=1000");
        }
    }
}
