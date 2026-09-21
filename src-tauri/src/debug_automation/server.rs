//! 调试自动化模块的回环 HTTP 服务器。
//!
//! 服务器拥有一个专用线程及其自有的 Tokio 运行时，因此绝不会与 Tauri 的
//! 内部运行时耦合。它仅绑定 `127.0.0.1`，并针对回环名称校验 `Host` 头
//! （DNS 重绑定防护）。失败时响应体遵循共享的 [`AutomationError`] 传输格式，
//! 成功时遵循各端点的 JSON（或 PNG）格式。

use std::{net::SocketAddr, sync::Arc};

use axum::{
    extract::{rejection::JsonRejection, Request, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};

use super::{
    capture, input,
    model::{AutomationError, AutomationResult, ErrorCode, HealthResponse, InputRequest},
    platform, ui_tree, AppState,
};

/// 构建自动化 API 的 axum 路由器。
///
/// # 参数
/// - `state`：传递给每个处理器与授权中间件的共享应用程序状态。
///
/// # 返回值
/// 包含全部 v1 端点与组合 Host 防护层的完整配置路由器。
fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/screenshot", get(screenshot))
        .route("/ui-tree", get(ui_tree_endpoint))
        .route("/input", post(input_endpoint))
        .route("/shutdown", post(shutdown))
        .layer(middleware::from_fn(authorize))
        .with_state(state)
}

/// 在专用线程与运行时上启动自动化 HTTP 服务器。
///
/// 该函数会阻塞，直到回环监听器完成绑定，因此端口冲突会在应用程序继续启动
/// 之前被报告。
///
/// # 参数
/// - `state`：传递给 HTTP 层的共享应用程序状态。
/// - `port`：要绑定的固定 TCP 端口。
///
/// # 返回值
/// 服务器开始监听后返回 `Ok(())`；运行时线程无法启动或端口无法绑定时，
/// 返回英文错误消息。
pub(crate) fn start(state: Arc<AppState>, port: u16) -> Result<(), String> {
    let (result_sender, result_receiver) = std::sync::mpsc::channel::<Result<(), String>>();

    std::thread::Builder::new()
        .name("debug-automation-http".to_string())
        .spawn(move || {
            let runtime = match tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
            {
                Ok(runtime) => runtime,
                Err(error) => {
                    let _ = result_sender.send(Err(format!(
                        "failed to build the automation runtime: {error}"
                    )));
                    return;
                }
            };

            runtime.block_on(async move {
                let requested = SocketAddr::from((std::net::Ipv4Addr::LOCALHOST, port));
                let listener = match tokio::net::TcpListener::bind(requested).await {
                    Ok(listener) => listener,
                    Err(error) => {
                        let _ =
                            result_sender.send(Err(format!("failed to bind {requested}: {error}")));
                        return;
                    }
                };

                let app = router(state);
                if result_sender.send(Ok(())).is_err() {
                    return;
                }

                if let Err(error) = axum::serve(listener, app).await {
                    tracing::error!("debug automation http server stopped: {error}");
                }
            });
        })
        .map_err(|error| format!("failed to spawn the automation http thread: {error}"))?;

    result_receiver
        .recv()
        .map_err(|_| "the automation http thread exited before reporting its status".to_string())?
}

/// 授权中间件：校验 `Host` 头。
///
/// # 参数
/// - `request`：待校验请求头的传入请求。
/// - `next`：流水线中的下一个中间件或处理器。
///
/// # 返回值
/// 成功时返回下游响应，否则在不触及处理器的情况下返回 forbidden
/// [`AutomationError`]。
async fn authorize(request: Request, next: Next) -> Result<Response, AutomationError> {
    verify_host(request.headers())?;
    Ok(next.run(request).await)
}

/// 校验 `Host` 头指定的是回环主机。
///
/// 仅接受 `127.0.0.1` 与 `localhost`（可带或不带端口，大小写不敏感），
/// 从而阻止针对回环服务器的 DNS 重绑定攻击。
///
/// # 参数
/// - `headers`：要检查的请求头。
///
/// # 返回值
/// 主机被允许时返回 `Ok(())`；否则返回英文 forbidden 错误。
fn verify_host(headers: &HeaderMap) -> AutomationResult<()> {
    let raw = headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| {
            AutomationError::new(ErrorCode::Forbidden, "missing or malformed Host header")
        })?;
    let host = raw.rsplit_once(':').map_or(raw, |(host, _)| host);

    if host.eq_ignore_ascii_case("localhost") || host == "127.0.0.1" {
        Ok(())
    } else {
        Err(AutomationError::new(
            ErrorCode::Forbidden,
            format!("Host header is not allowed: {raw}"),
        ))
    }
}

/// 将阻塞任务 join 失败映射为英文内部错误。
///
/// # 参数
/// - `error`：Tokio 阻塞池抛出的 join 错误。
///
/// # 返回值
/// 描述失败阻塞任务的自动化错误。
fn join_error(error: tokio::task::JoinError) -> AutomationError {
    AutomationError::internal(format!("blocking task failed: {error}"))
}

/// 提取 JSON 请求体，同时保持统一的错误格式。
///
/// # 参数
/// - `payload`：axum 生成的 JSON 提取器结果。
///
/// # 返回值
/// 反序列化后的请求体，或携带 axum 拒绝文本的英文 bad-request 错误。
fn json_body<T>(payload: Result<Json<T>, JsonRejection>) -> AutomationResult<T> {
    match payload {
        Ok(Json(value)) => Ok(value),
        Err(rejection) => Err(AutomationError::bad_request(format!(
            "invalid request body: {}",
            rejection.body_text()
        ))),
    }
}

/// 返回存活状态与整个主窗口的尺寸。
///
/// # 参数
/// - `state`：共享应用程序状态。
///
/// # 返回值
/// 携带物理像素窗口尺寸的健康检查载荷，或几何查询失败时的英文错误。
async fn health(State(state): State<Arc<AppState>>) -> AutomationResult<Json<HealthResponse>> {
    let geometry_state = state.clone();
    let main = tokio::task::spawn_blocking(move || platform::main_raw_window(&geometry_state))
        .await
        .map_err(join_error)??;

    Ok(Json(HealthResponse {
        version: env!("CARGO_PKG_VERSION"),
        width: main.window_width,
        height: main.window_height,
    }))
}

/// 将整个主窗口捕获为 PNG。
///
/// 图像覆盖完整窗口，包含标题栏与边框，其像素为主窗口物理像素，
/// 与 UI 树 bounds 及输入端点使用同一坐标系。
///
/// # 参数
/// - `state`：共享应用程序状态。
///
/// # 返回值
/// PNG 响应，或捕获失败时的英文错误。
async fn screenshot(State(state): State<Arc<AppState>>) -> AutomationResult<Response> {
    let capture_state = state.clone();
    let png = tokio::task::spawn_blocking(move || capture::screenshot(&capture_state))
        .await
        .map_err(join_error)??;

    let mut response = png.into_response();
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static("image/png"));
    Ok(response)
}

/// 收集主 webview 的简化 UI 树。
///
/// 树以 CSS 像素从 webview 中收集，并在返回前转换为主窗口物理像素，
/// 因此其 bounds 与截图像素及输入端点共享同一坐标系。仅输出容器、
/// 交互元素与文本节点；载荷经过美化打印，使其嵌套结构保持可读。
///
/// # 参数
/// - `state`：共享应用程序状态。
///
/// # 返回值
/// 作为 `application/json` 响应的 UI 树 JSON，或收集、序列化失败时的英文错误。
async fn ui_tree_endpoint(State(state): State<Arc<AppState>>) -> AutomationResult<Response> {
    let geometry_state = state.clone();
    let main = tokio::task::spawn_blocking(move || platform::main_raw_window(&geometry_state))
        .await
        .map_err(join_error)??;

    let mut entries = ui_tree::collect(&state.app_handle).await?;
    ui_tree::to_window_coordinates(&mut entries, &main);

    let body = serde_json::to_string_pretty(&entries).map_err(|error| {
        AutomationError::internal(format!("failed to serialize the ui tree: {error}"))
    })?;
    let mut response = body.into_response();
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static("application/json"));
    Ok(response)
}

/// 执行一批按顺序排列的输入命令。
///
/// # 参数
/// - `state`：共享应用程序状态。
/// - `payload`：请求的可选 JSON 请求体。
///
/// # 返回值
/// 队列全部执行成功后返回 `204 No Content`；任一命令失败时返回
/// 携带失败命令索引的英文自动化错误。
async fn input_endpoint(
    State(state): State<Arc<AppState>>,
    payload: Result<Json<InputRequest>, JsonRejection>,
) -> AutomationResult<StatusCode> {
    let request = json_body(payload)?;
    let input_state = state.clone();
    tokio::task::spawn_blocking(move || input::execute(&input_state, &request))
        .await
        .map_err(join_error)??;
    Ok(StatusCode::NO_CONTENT)
}

/// 请求关闭应用程序。
///
/// 先返回响应，再在独立线程中短暂延迟后触发 Tauri 的正常退出流程，
/// 使调用方先收到确认、再等待进程结束；退出流程与用户手动关闭窗口一致，
/// 会保存偏好与元数据。
///
/// # 参数
/// - `state`：共享应用程序状态。
///
/// # 返回值
/// 确认响应 `204 No Content`；应用随后自行退出。
async fn shutdown(State(state): State<Arc<AppState>>) -> StatusCode {
    let app_handle = state.app_handle.clone();
    std::thread::spawn(move || {
        // 延迟到响应送达调用方之后再退出。
        std::thread::sleep(std::time::Duration::from_millis(250));
        app_handle.exit(0);
    });
    StatusCode::NO_CONTENT
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 构建仅携带 `Host` 头的请求头映射。
    ///
    /// # 参数
    /// - `host`：作为 `Host` 头插入的原始值。
    ///
    /// # 返回值
    /// 恰好携带该 `Host` 头的请求头映射。
    fn headers_with_host(host: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(header::HOST, HeaderValue::from_str(host).unwrap());
        headers
    }

    #[test]
    fn host_header_accepts_loopback_names_with_optional_port() {
        for host in ["127.0.0.1", "127.0.0.1:43210", "localhost", "LocalHost:8080"] {
            assert!(verify_host(&headers_with_host(host)).is_ok(), "host: {host}");
        }
    }

    #[test]
    fn host_header_rejects_foreign_names_and_missing_values() {
        for host in ["evil.example", "evil.example:80", "192.168.1.10", "[::1]:80"] {
            let error = verify_host(&headers_with_host(host)).unwrap_err();
            assert!(error.to_string().starts_with("Forbidden"), "host: {host}");
        }
        let error = verify_host(&HeaderMap::new()).unwrap_err();
        assert!(error.to_string().starts_with("Forbidden"));
    }
}
