//! 调试自动化各子模块共享的数据模型。
//!
//! 此处定义的传输格式是 HTTP 层、输入/截图后端与注入式 UI 树收集器之间的契约。
//! JSON 字段名使用 camelCase，以匹配前端约定。

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};

/// 以主窗口物理像素表示的位置：原点是整个应用程序窗口的左上角，
/// 它同时也是截图像素与 UI 树 bounds 的原点。
#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

/// 自动化 API 允许合成的鼠标按键。
#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MouseButton {
    Left,
    Right,
}

impl Default for MouseButton {
    fn default() -> Self {
        Self::Left
    }
}

/// 鼠标滚轮方向。
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ScrollDirection {
    Up,
    Down,
    Left,
    Right,
}

/// `wait` 命令的时间单位。
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TimeUnit {
    Ms,
    S,
}

impl Default for TimeUnit {
    fn default() -> Self {
        Self::Ms
    }
}

/// `POST /input` 的请求体。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InputRequest {
    /// 按数组顺序串行执行的命令列表。
    pub commands: Vec<InputCommand>,
}

/// 队列输入接口接受的单条命令。
///
/// 判别字段 `kind` 的取值使用 snake_case（如 `mouse_click`），
/// 各命令的参数使用 camelCase，与模块内其他 JSON 字段一致。
#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum InputCommand {
    /// 把光标以受配置控制的直线平滑移动到目标位置。
    MouseMove(MouseMoveCommand),
    /// 在当前光标位置按下鼠标按键。
    MousePress(MousePressCommand),
    /// 在当前光标位置松开鼠标按键。
    MouseRelease(MouseReleaseCommand),
    /// 在当前光标位置快速按下并松开鼠标按键若干次。
    MouseClick(MouseClickCommand),
    /// 在当前光标位置滚动鼠标滚轮。
    MouseScroll(MouseScrollCommand),
    /// 从起点按下鼠标按键并沿直线拖动到终点后松开。
    MouseDrag(MouseDragCommand),
    /// 按数组顺序依次按下键盘按键。
    KeyPress(KeyPressCommand),
    /// 按数组顺序依次松开键盘按键。
    KeyRelease(KeyReleaseCommand),
    /// 把整个按键序列快速点击若干轮。
    KeyClick(KeyClickCommand),
    /// 输入一段 Unicode 文本。
    Type(TypeCommand),
    /// 等待一段时长。
    Wait(WaitCommand),
    /// 临时修改本次请求中后续命令的执行配置。
    Config(ConfigCommand),
}

impl InputCommand {
    /// 返回命令的稳定名称，用于错误消息中定位失败的命令。
    ///
    /// # 返回值
    /// 与 `kind` 判别字段取值一致的命令名。
    pub fn kind(&self) -> &'static str {
        match self {
            Self::MouseMove(_) => "mouse_move",
            Self::MousePress(_) => "mouse_press",
            Self::MouseRelease(_) => "mouse_release",
            Self::MouseClick(_) => "mouse_click",
            Self::MouseScroll(_) => "mouse_scroll",
            Self::MouseDrag(_) => "mouse_drag",
            Self::KeyPress(_) => "key_press",
            Self::KeyRelease(_) => "key_release",
            Self::KeyClick(_) => "key_click",
            Self::Type(_) => "type",
            Self::Wait(_) => "wait",
            Self::Config(_) => "config",
        }
    }
}

/// `mouse_move` 命令：把光标平滑移动到目标位置。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MouseMoveCommand {
    pub x: f64,
    pub y: f64,
}

/// `mouse_press` 命令：在当前光标位置按下鼠标按键。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MousePressCommand {
    #[serde(default)]
    pub button: MouseButton,
}

/// `mouse_release` 命令：在当前光标位置松开鼠标按键。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MouseReleaseCommand {
    #[serde(default)]
    pub button: MouseButton,
}

/// `mouse_click` 命令：在当前光标位置快速按下并松开鼠标按键。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MouseClickCommand {
    #[serde(default)]
    pub button: MouseButton,
    /// 点击次数，取值范围 1..=10。
    #[serde(default = "default_click_count")]
    pub count: u32,
}

/// `mouse_scroll` 命令：在当前光标位置滚动鼠标滚轮。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MouseScrollCommand {
    pub direction: ScrollDirection,
    /// 滚轮刻度数，取值范围 1..=1000。
    pub amount: u32,
}

/// `mouse_drag` 命令：从起点按下鼠标按键并沿直线拖动到终点后松开。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MouseDragCommand {
    pub from: Point,
    pub to: Point,
    #[serde(default)]
    pub button: MouseButton,
}

/// `key_press` 命令：按数组顺序依次按下键盘按键。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyPressCommand {
    /// 按键名数组，顺序即按下顺序，长度范围 1..=8。
    pub keys: Vec<String>,
}

/// `key_release` 命令：按数组顺序依次松开键盘按键。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyReleaseCommand {
    /// 按键名数组，顺序即松开顺序，长度范围 1..=8。
    pub keys: Vec<String>,
}

/// `key_click` 命令：把整个按键序列快速点击若干轮。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyClickCommand {
    /// 按键名数组，顺序即按下顺序与 FIFO 松开顺序，长度范围 1..=8。
    pub keys: Vec<String>,
    /// 整个按键序列的重复轮数，取值范围 1..=10。
    #[serde(default = "default_click_count")]
    pub count: u32,
}

/// `type` 命令：输入一段 Unicode 文本。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TypeCommand {
    pub text: String,
}

/// `wait` 命令：等待一段时长。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WaitCommand {
    /// 时长的数值，配合 `unit` 解释。
    pub duration: u64,
    /// 时长的单位，缺省为毫秒。
    #[serde(default)]
    pub unit: TimeUnit,
}

/// `config` 命令：临时修改本次请求中后续命令的执行配置。
///
/// 所有字段均为可选，未提供的字段保持当前值；修改仅对本次请求内
/// 其后的命令生效，请求结束后恢复默认值。
#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ConfigCommand {
    /// `mouse_click` 按下与松开之间的时长（毫秒），范围 0..=5000。
    pub mouse_click_span: Option<u64>,
    /// `mouse_move` 与 `mouse_drag` 的直线移动速度（物理像素/秒），范围 50..=20000。
    pub mouse_move_speed: Option<f64>,
    /// `key_click` 一轮序列内相邻按键按下或松开之间的间隔（毫秒），范围 0..=1000。
    pub key_click_in_turn_interval: Option<u64>,
    /// `key_click` 相邻两轮之间的间隔（毫秒），范围 0..=5000。
    pub key_click_cross_turn_interval: Option<u64>,
    /// `key_click` 一轮中全部按键按下后保持的时长（毫秒），范围 0..=5000。
    pub key_click_span: Option<u64>,
    /// 相邻两条命令之间的额外间隔（毫秒），范围 0..=5000。
    pub command_interval: Option<u64>,
}

/// 返回点击次数字段的默认值。
///
/// # 返回值
/// `mouse_click` 与 `key_click` 在未提供 `count` 时使用的点击次数。
fn default_click_count() -> u32 {
    1
}

/// `GET /health` 的响应体。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthResponse {
    pub version: &'static str,
    /// 整个主窗口的宽度，单位为物理像素。
    pub width: u32,
    /// 整个主窗口的高度，单位为物理像素。
    pub height: u32,
}

/// 自动化 API 返回的机器可读错误码。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    BadRequest,
    Forbidden,
    #[allow(dead_code)]
    NotFound,
    InternalError,
}

impl ErrorCode {
    /// 返回错误码的稳定传输名称。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::BadRequest => "BadRequest",
            Self::Forbidden => "Forbidden",
            Self::NotFound => "NotFound",
            Self::InternalError => "InternalError",
        }
    }

    /// 返回与错误码关联的 HTTP 状态码。
    pub fn status(self) -> StatusCode {
        match self {
            Self::BadRequest => StatusCode::BAD_REQUEST,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::InternalError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

/// 携带 HTTP 状态码、机器可读错误码和消息的错误类型。
#[derive(Debug)]
pub struct AutomationError {
    code: ErrorCode,
    message: String,
}

impl AutomationError {
    /// 根据错误码与人类可读消息创建自动化错误。
    ///
    /// # 参数
    /// - `code`：机器可读的错误码。
    /// - `message`：英文的、可安全记录日志的错误描述。
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    /// 创建一个 `BadRequest` 错误。
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::BadRequest, message)
    }

    /// 创建一个 `InternalError` 错误。
    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::InternalError, message)
    }

    /// 返回错误的机器可读错误码。
    pub fn code(&self) -> ErrorCode {
        self.code
    }

    /// 返回错误的人类可读消息。
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl std::fmt::Display for AutomationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code.as_str(), self.message)
    }
}

impl std::error::Error for AutomationError {}

impl IntoResponse for AutomationError {
    fn into_response(self) -> Response {
        let body = Json(serde_json::json!({
            "error": {
                "code": self.code.as_str(),
                "message": self.message,
            }
        }));
        (self.code.status(), body).into_response()
    }
}

/// 供自动化处理器与后端使用的便捷结果别名。
pub type AutomationResult<T> = Result<T, AutomationError>;
