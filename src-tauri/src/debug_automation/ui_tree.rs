//! 调试自动化模块的 UI 树收集。
//!
//! HTTP 层从不亲自遍历 DOM。取而代之的是，[`collect`] 在主 webview 中对单个
//! 脚本求值：该脚本安装来自 `ui_tree_bridge.js` 的桥接收集器并立即遍历 DOM，
//! 将结果包装进 `{ok, tree | error}` 信封，使 JavaScript 异常永远不会逃逸出
//! 被求值的表达式。求值结果被序列化为 JSON 字符串，并通过每请求的回调通道
//! 投递，超时时间为十秒。

use std::time::Duration;

use serde_json::Value;
use tauri::{AppHandle, Manager};

use super::model::{AutomationError, AutomationResult};
use super::platform::RawWindow;

/// 注入主 webview 的自包含收集器的源码。
const BRIDGE_SCRIPT: &str = include_str!("ui_tree_bridge.js");

/// 单次收集尝试的时间预算。
const COLLECT_TIMEOUT: Duration = Duration::from_secs(10);

/// 构建用于安装桥接并收集树的 JavaScript 表达式。
///
/// 桥接脚本与收集器调用作为单个表达式求值，且该调用被包裹在 try/catch 中，
/// 因此求值结果始终是收集的 `{ok, tree | error}` 信封。
/// 收集器始终遍历完整文档；它不接受任何选项。
///
/// # 返回值
/// 在页面中求值的表达式。
fn build_collect_script() -> String {
    format!(
        "{BRIDGE_SCRIPT}\n;(function () {{ try {{ return {{ ok: true, tree: window.__iNetDebugAutomation.collect() }}; }} \
         catch (error) {{ return {{ ok: false, error: String(error && error.message ? error.message : error) }}; }} }})()"
    )
}

/// 通过求值注入的桥接脚本来收集当前 UI 树。
///
/// 单次求值即安装桥接并运行收集器；JSON 编码的求值结果通过每请求的回调通道
/// 到达。整个请求有十秒预算。
///
/// # 参数
/// - `app_handle`：正在运行的 Tauri 应用程序的句柄，用于访问主 webview
///   窗口。
///
/// # 返回值
/// 注入式收集器投递的树 JSON。当主窗口缺失、脚本派发失败、收集器报告
/// JavaScript 错误、回调始终未到达或其载荷无法解析时，返回英文内部错误。
pub(crate) async fn collect(app_handle: &AppHandle) -> AutomationResult<Value> {
    let window = app_handle
        .get_webview_window("main")
        .ok_or_else(|| AutomationError::internal("main webview window not found"))?;

    let (sender, mut receiver) = tokio::sync::mpsc::channel::<String>(1);
    window
        .eval_with_callback(build_collect_script(), move |result| {
            // 通道关闭只会在请求超时后发生；迟到的结果会被有意丢弃。
            let _ = sender.try_send(result);
        })
        .map_err(|error| {
            AutomationError::internal(format!("failed to evaluate ui tree collector: {error}"))
        })?;

    let result = tokio::time::timeout(COLLECT_TIMEOUT, receiver.recv())
        .await
        .map_err(|_| AutomationError::internal("ui tree collection timed out"))?
        .ok_or_else(|| {
            AutomationError::internal("ui tree collector callback channel closed before responding")
        })?;

    parse_collect_result(&result)
}

/// 解析注入式收集器返回的 `{ok, tree | error}` 信封。
///
/// # 参数
/// - `result`：webview 投递的原始求值结果。抛出的 JavaScript 异常在 Windows
///   上表现为字符串 `null`，在 WebKit 上表现为空字符串，二者在此都会被拒绝。
///
/// # 返回值
/// 成功路径上返回树 JSON；否则返回描述结果缺失、收集器失败或信封格式错误的
/// 英文内部错误。
fn parse_collect_result(result: &str) -> AutomationResult<Value> {
    if result.is_empty() || result == "null" {
        return Err(AutomationError::internal(
            "ui tree collector returned no result",
        ));
    }

    let envelope: Value = serde_json::from_str(result).map_err(|error| {
        AutomationError::internal(format!("failed to parse ui tree collector result: {error}"))
    })?;
    let object = envelope.as_object().ok_or_else(|| {
        AutomationError::internal("ui tree collector returned a malformed result")
    })?;

    match object.get("ok").and_then(Value::as_bool) {
        Some(true) => object.get("tree").cloned().ok_or_else(|| {
            AutomationError::internal("ui tree collector reported success without a tree")
        }),
        Some(false) => {
            let message = object
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("unknown error");
            Err(AutomationError::internal(format!(
                "ui tree collector failed: {message}"
            )))
        }
        None => Err(AutomationError::internal(
            "ui tree collector returned a malformed result",
        )),
    }
}

/// 将每个条目的 bounds 从 webview CSS 像素改写为主窗口物理像素。
///
/// 注入式收集器报告的视口相对 CSS 坐标以客户区左上角为原点。截图与输入
/// 端点以整个窗口的左上角为原点，因此在应用窗口缩放系数之前，先加上窗口内
/// 客户区的偏移量。每个值都四舍五入为整数物理像素。该转换是 Rust 侧的
/// 内部事务：HTTP API 的调用方始终看到窗口物理像素。
///
/// # 参数
/// - `entries`：注入式收集器生成的简化树数组，就地转换。
/// - `raw`：主窗口几何信息，提供窗口内客户区偏移量与 DPI 缩放系数。
pub(crate) fn to_window_coordinates(entries: &mut Value, raw: &RawWindow) {
    let offset_x = f64::from(raw.client_x - raw.window_x);
    let offset_y = f64::from(raw.client_y - raw.window_y);
    if let Some(array) = entries.as_array_mut() {
        for entry in array {
            convert_entry(entry, offset_x, offset_y, raw.scale_factor);
        }
    }
}

/// 就地转换单个条目及其所有后代的 bounds。
///
/// # 参数
/// - `entry`：要转换的条目 JSON；非对象值保持原样。
/// - `offset_x`：窗口内客户区的水平偏移量，单位为物理像素。
/// - `offset_y`：窗口内客户区的垂直偏移量，单位为物理像素。
/// - `scale_factor`：应用于 CSS 值的窗口 DPI 缩放系数。
fn convert_entry(entry: &mut Value, offset_x: f64, offset_y: f64, scale_factor: f64) {
    let Some(object) = entry.as_object_mut() else {
        return;
    };

    if let Some(bounds) = object.get_mut("bounds").and_then(Value::as_object_mut) {
        for (key, offset) in [
            ("left", offset_x),
            ("top", offset_y),
            ("width", 0.0),
            ("height", 0.0),
        ] {
            if let Some(value) = bounds.get_mut(key) {
                if let Some(number) = value.as_f64() {
                    *value = Value::from((number * scale_factor + offset).round() as i64);
                }
            }
        }
    }

    if let Some(children) = object.get_mut("children").and_then(Value::as_array_mut) {
        for child in children {
            convert_entry(child, offset_x, offset_y, scale_factor);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 构建一个原始窗口，其窗口/客户区原点对与 DPI 缩放可独立选择。
    ///
    /// # 参数
    /// - `window`：物理屏幕坐标中的完整窗口原点。
    /// - `client`：物理屏幕坐标中的客户区原点。
    /// - `scale_factor`：窗口的 DPI 缩放。
    ///
    /// # 返回值
    /// 适用于坐标转换测试的 [`RawWindow`]。
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

    #[test]
    fn entry_bounds_become_window_physical_pixels() {
        // 转换路径：CSS bounds 先加上窗口内客户区偏移量，再应用 DPI 缩放，
        // 且总是四舍五入为整数物理像素；宽度和高度只缩放、不加偏移。
        // #text 条目与元素条目的转换方式完全相同。
        let raw = raw_window((100, 50), (108, 82), 2.0);
        let mut entries = serde_json::json!([
            {
                "tag": "DIV",
                "role": "dialog",
                "bounds": {"left": 0.0, "top": 0.0, "width": 100.5, "height": 50.5},
                "children": [
                    {
                        "tag": "#text",
                        "bounds": {"left": 10.25, "top": 20.5, "width": 30.0, "height": 40.0},
                        "text": "hello"
                    }
                ]
            }
        ]);

        to_window_coordinates(&mut entries, &raw);

        assert_eq!(entries[0]["bounds"]["left"], 8);
        assert_eq!(entries[0]["bounds"]["top"], 32);
        assert_eq!(entries[0]["bounds"]["width"], 201);
        assert_eq!(entries[0]["bounds"]["height"], 101);
        let text = &entries[0]["children"][0];
        assert_eq!(text["bounds"]["left"], 29);
        assert_eq!(text["bounds"]["top"], 73);
        assert_eq!(text["bounds"]["width"], 60);
        assert_eq!(text["bounds"]["height"], 80);
    }

    #[test]
    fn empty_and_non_array_payloads_are_left_untouched() {
        // 防御路径：空树与畸形载荷都能在不 panic 的情况下处理。
        let raw = raw_window((0, 0), (8, 32), 1.0);
        let mut empty = serde_json::json!([]);
        to_window_coordinates(&mut empty, &raw);
        assert!(empty.as_array().unwrap().is_empty());

        let mut malformed = serde_json::json!({"root": null});
        to_window_coordinates(&mut malformed, &raw);
        assert!(malformed["root"].is_null());
    }

    #[test]
    fn collect_script_embeds_the_bridge_without_options() {
        // 成功路径：表达式以桥接脚本开头，并在不传任何选项的情况下调用收集器。
        let script = build_collect_script();
        assert!(script.starts_with(BRIDGE_SCRIPT));
        assert!(script.contains("window.__iNetDebugAutomation.collect()"));
        // 收集器调用被包裹起来，使抛出的异常成为信封的错误分支，
        // 而不是逃逸出表达式。
        assert!(script.contains("try { return { ok: true, tree: "));
        assert!(script.contains(
            "catch (error) { return { ok: false, error: String(error && error.message ? error.message : error) }; }"
        ));
    }

    #[test]
    fn collect_result_returns_tree_from_success_envelope() {
        // 成功路径：带树的 ok 信封恰好产出该树。
        let tree = parse_collect_result(r#"{"ok":true,"tree":{"format":"inet-ui-tree/v1"}}"#)
            .expect("success envelope must parse");
        assert_eq!(tree["format"], "inet-ui-tree/v1");
    }

    #[test]
    fn collect_result_maps_success_without_tree_to_internal_error() {
        // 失败路径：不带 tree 字段的成功信封会被拒绝。
        let error = parse_collect_result(r#"{"ok":true}"#).unwrap_err();
        assert!(error.to_string().starts_with("InternalError"));
    }

    #[test]
    fn collect_result_maps_collector_error_to_internal_error() {
        // 失败路径：失败的信封连同其消息一起被包装。
        let error = parse_collect_result(r#"{"ok":false,"error":"boom"}"#).unwrap_err();
        let message = error.to_string();
        assert!(message.starts_with("InternalError"));
        assert!(message.contains("boom"));
    }

    #[test]
    fn collect_result_rejects_missing_empty_and_malformed_payloads() {
        // 失败路径：空结果、null、非对象 JSON、无效 JSON 以及缺少 ok 字段的
        // 信封都会映射为 InternalError。
        for payload in ["", "null", "\"tree\"", "not json", r#"{"tree":{}}"#] {
            let error = parse_collect_result(payload).unwrap_err();
            assert!(
                error.to_string().starts_with("InternalError"),
                "payload: {payload}"
            );
        }
    }
}
