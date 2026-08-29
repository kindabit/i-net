//! 应用信息模块：向前端关于对话框提供编译期确定的应用元数据。

use serde::Serialize;

/// 关于对话框展示用的应用信息。
#[derive(Serialize)]
pub struct AppInfo {
    /// 应用版本号（取自 Cargo.toml package.version）
    pub app_version: String,
    /// 作者（取自 Cargo.toml package.authors）
    pub author: String,
    /// 源代码仓库地址（取自 Cargo.toml package.repository）
    pub repository: String,
    /// 编译所用的 Rust 编译器版本（由 build.rs 捕获注入）
    pub rust_version: String,
    /// Tauri 框架版本
    pub tauri_version: String,
}

/// 获取应用信息（应用版本、作者、仓库地址、Rust 版本、Tauri 版本）。
///
/// # 参数
/// 无。
///
/// # 返回值
/// 返回编译期确定的应用信息。
#[tauri::command]
pub fn app_info_get() -> AppInfo {
    AppInfo {
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        author: env!("CARGO_PKG_AUTHORS").to_string(),
        repository: env!("CARGO_PKG_REPOSITORY").to_string(),
        rust_version: env!("I_NET_RUSTC_VERSION").to_string(),
        tauri_version: tauri::VERSION.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 成功路径：各字段非空（同时验证 build.rs 的 I_NET_RUSTC_VERSION 注入生效）。
    /// 该 command 纯组装编译期常量，不存在失败路径。
    #[test]
    fn app_info_fields_are_not_empty() {
        let info = app_info_get();
        assert!(!info.app_version.is_empty());
        assert!(!info.author.is_empty());
        assert!(!info.repository.is_empty());
        assert!(!info.rust_version.is_empty());
        assert!(!info.tauri_version.is_empty());
    }
}