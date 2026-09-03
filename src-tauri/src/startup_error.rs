//! 启动期错误处理模块：启动阶段（setup）数据库初始化失败时，按操作系统语言向终端用户
//! 展示友好的原生阻塞对话框提示，用户确认后以非零码退出进程。
//!
//! 启动阶段前端尚未就绪，错误无法经由 invoke 到达前端的受控崩溃通道
//! （use-fatal-error / FatalErrorDialog / fatal_exit），也无法从前端获取语言偏好，
//! 因此本模块在后端按操作系统语言选择中英双语提示文本，并直接使用 rfd 的阻塞式
//! 原生对话框（tauri-plugin-dialog 的对话框派发依赖事件循环，setup 阶段事件循环
//! 尚未启动，不可用）。

use crate::common::data_version::entity::DataVersion;
use crate::error_code::ErrorCode;

/// 启动期初始化失败的数据库。
pub enum Database {
    /// 偏好设置数据库。
    Preference,
    /// 元数据数据库。
    Metadata,
}

impl Database {
    /// 获取该数据库面向终端用户的本地化名称。
    ///
    /// # 参数
    /// - `locale`: 提示文本语言。
    ///
    /// # 返回值
    /// 返回该数据库在对应语言下的显示名称。
    fn display_name(&self, locale: Locale) -> &'static str {
        match (self, locale) {
            (Database::Preference, Locale::Chinese) => "偏好设置数据库",
            (Database::Preference, Locale::English) => "preference database",
            (Database::Metadata, Locale::Chinese) => "元数据数据库",
            (Database::Metadata, Locale::English) => "metadata database",
        }
    }
}

/// 提示文本的语言。启动阶段无法从前端获取语言偏好，按操作系统语言选择。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Locale {
    /// 中文。
    Chinese,
    /// 英文（中文之外的默认语言）。
    English,
}

/// 将系统 locale 字符串映射为提示语言。
///
/// # 参数
/// - `system_locale`: 操作系统 locale 字符串（如 "zh-CN"、"en-US"）。
///
/// # 返回值
/// 中文（含各区域变体，以 "zh" 开头）返回 `Locale::Chinese`，其余一律返回 `Locale::English`。
fn locale_from_system_locale(system_locale: &str) -> Locale {
    if system_locale.starts_with("zh") {
        Locale::Chinese
    } else {
        Locale::English
    }
}

/// 将数据版本格式化为 "major.minor.patch" 形式的字符串。
///
/// # 参数
/// - `version`: 待格式化的数据版本。
///
/// # 返回值
/// 返回格式化后的版本字符串。
fn format_version(version: &DataVersion) -> String {
    format!("{}.{}.{}", version.major, version.minor, version.patch)
}

/// 构造启动错误对话框的标题。
///
/// # 参数
/// - `locale`: 提示文本语言。
/// - `error`: 初始化失败返回的错误码。
///
/// # 返回值
/// 返回对应语言与错误类型的对话框标题。
fn title(locale: Locale, error: &ErrorCode) -> &'static str {
    match error {
        ErrorCode::DataVersionMismatch { .. } => match locale {
            Locale::Chinese => "数据版本不兼容",
            Locale::English => "Incompatible Data Version",
        },
        ErrorCode::NoDataVersion | ErrorCode::MultipleDataVersion => match locale {
            Locale::Chinese => "数据库文件无效",
            Locale::English => "Invalid Database File",
        },
        _ => match locale {
            Locale::Chinese => "启动失败",
            Locale::English => "Startup Failed",
        },
    }
}

/// 构造启动错误对话框的正文（面向终端用户的友好提示）。
///
/// 数据版本不匹配（用户实际会遇到的情况）给出完整的版本信息与应对建议：
/// 实际版本更高时建议升级应用程序，实际版本更低时说明数据库由旧版本应用创建。
/// 数据版本表行数异常提示数据库文件无效；其余理论上不应出现的错误附带
/// 调试详情以便用户反馈诊断。
///
/// # 参数
/// - `locale`: 提示文本语言。
/// - `database`: 初始化失败的数据库。
/// - `error`: 初始化失败返回的错误码。
///
/// # 返回值
/// 返回对应语言与错误类型的对话框正文。
fn message(locale: Locale, database: &Database, error: &ErrorCode) -> String {
    let database_name = database.display_name(locale);
    match error {
        ErrorCode::DataVersionMismatch { expected, actual } => {
            let expected_text = format_version(expected);
            let actual_text = format_version(actual);
            let actual_is_newer = (actual.major, actual.minor, actual.patch)
                > (expected.major, expected.minor, expected.patch);
            match (locale, actual_is_newer) {
                (Locale::Chinese, true) => format!(
                    "{database_name}的数据版本（{actual_text}）高于当前应用程序支持的版本（{expected_text}）。\n\n该数据库由更新版本的应用程序创建，请将应用程序升级到最新版本后重试。\n\n应用程序将退出。"
                ),
                (Locale::Chinese, false) => format!(
                    "{database_name}的数据版本（{actual_text}）低于当前应用程序要求的版本（{expected_text}）。\n\n该数据库由旧版本的应用程序创建，当前版本的应用程序无法打开。\n\n应用程序将退出。"
                ),
                (Locale::English, true) => format!(
                    "The data version of the {database_name} ({actual_text}) is newer than the version supported by this application ({expected_text}).\n\nThe database was created by a newer version of the application. Please update the application to the latest version and try again.\n\nThe application will now exit."
                ),
                (Locale::English, false) => format!(
                    "The data version of the {database_name} ({actual_text}) is older than the version required by this application ({expected_text}).\n\nThe database was created by an older version of the application and cannot be opened by this version.\n\nThe application will now exit."
                ),
            }
        }
        ErrorCode::NoDataVersion | ErrorCode::MultipleDataVersion => match locale {
            Locale::Chinese => {
                format!("{database_name}文件无效或已损坏，应用程序无法启动。\n\n应用程序将退出。")
            }
            Locale::English => format!(
                "The {database_name} file is invalid or corrupted, and the application cannot start.\n\nThe application will now exit."
            ),
        },
        _ => match locale {
            Locale::Chinese => {
                format!("{database_name}初始化失败：{error:?}\n\n应用程序将退出。")
            }
            Locale::English => format!(
                "Failed to initialize the {database_name}: {error:?}\n\nThe application will now exit."
            ),
        },
    }
}

/// 启动期数据库初始化失败的受控退出：记录英文错误日志，按操作系统语言弹出
/// 原生阻塞对话框告知用户，待用户确认后以非零码退出进程。
///
/// rfd 阻塞式对话框在主线程同步弹窗，期间 setup 不返回，避免应用在数据库连接
/// 状态未初始化的情况下继续运行而再次 panic。退出走 std::process::exit，
/// 与受控崩溃一致：不触发 RunEvent::Exit 的保存逻辑，避免脏数据写盘。
/// 该函数的对话框与退出路径按设计不可单元测试（弹原生对话框并终止进程），
/// 由代码审查保证；文本构造逻辑由模块内单元测试覆盖。
///
/// # 参数
/// - `database`: 初始化失败的数据库。
/// - `error`: 初始化失败返回的错误码。
///
/// # 返回值
/// 无（该函数不会返回）。
pub fn abort(database: Database, error: &ErrorCode) -> ! {
    tracing::error!("failed to initialize database during startup: {error:?}");
    let locale = locale_from_system_locale(&sys_locale::get_locale().unwrap_or_default());
    rfd::MessageDialog::new()
        .set_title(title(locale, error))
        .set_description(message(locale, &database, error))
        .set_level(rfd::MessageLevel::Error)
        .show();
    std::process::exit(1);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 覆盖 locale 映射的全部规则：中文各区域变体映射为中文，其余一律英文。
    #[test]
    fn test_locale_from_system_locale_all_rules() {
        // 中文简体、繁体与无区域后缀的形式都映射为中文。
        assert_eq!(locale_from_system_locale("zh-CN"), Locale::Chinese);
        assert_eq!(locale_from_system_locale("zh-TW"), Locale::Chinese);
        assert_eq!(locale_from_system_locale("zh"), Locale::Chinese);
        // 英文、其它语言与空字符串（系统 locale 获取失败的兜底）都映射为英文。
        assert_eq!(locale_from_system_locale("en-US"), Locale::English);
        assert_eq!(locale_from_system_locale("fr-FR"), Locale::English);
        assert_eq!(locale_from_system_locale(""), Locale::English);
    }

    /// 覆盖标题构造的全部错误类型与语言组合。
    #[test]
    fn test_title_all_kinds() {
        let mismatch = ErrorCode::DataVersionMismatch {
            expected: DataVersion { major: 3, minor: 0, patch: 0 },
            actual: DataVersion { major: 9, minor: 9, patch: 9 },
        };
        // 数据版本不匹配：中英标题。
        assert_eq!(title(Locale::Chinese, &mismatch), "数据版本不兼容");
        assert_eq!(title(Locale::English, &mismatch), "Incompatible Data Version");
        // 数据版本表行数异常：中英标题。
        assert_eq!(title(Locale::Chinese, &ErrorCode::NoDataVersion), "数据库文件无效");
        assert_eq!(title(Locale::English, &ErrorCode::NoDataVersion), "Invalid Database File");
        assert_eq!(title(Locale::Chinese, &ErrorCode::MultipleDataVersion), "数据库文件无效");
        // 其它错误（兜底）：中英标题。
        let other = ErrorCode::DatabaseError { detail: "boom".to_string() };
        assert_eq!(title(Locale::Chinese, &other), "启动失败");
        assert_eq!(title(Locale::English, &other), "Startup Failed");
    }

    /// 覆盖数据版本不匹配正文的两种版本大小关系：实际版本更高时建议升级应用程序，
    /// 实际版本更低时说明数据库由旧版本应用创建；正文包含数据库名与两个版本号。
    #[test]
    fn test_message_data_version_mismatch_both_directions() {
        let expected = DataVersion { major: 3, minor: 0, patch: 0 };

        // 实际版本更高（中文）：提示升级应用程序，含数据库名与两个版本号。
        let newer = ErrorCode::DataVersionMismatch {
            expected,
            actual: DataVersion { major: 9, minor: 9, patch: 9 },
        };
        let text = message(Locale::Chinese, &Database::Preference, &newer);
        assert!(text.contains("偏好设置数据库"));
        assert!(text.contains("9.9.9"));
        assert!(text.contains("3.0.0"));
        assert!(text.contains("升级"));

        // 实际版本更高（英文）：对称文案。
        let text = message(Locale::English, &Database::Metadata, &newer);
        assert!(text.contains("metadata database"));
        assert!(text.contains("newer version"));
        assert!(text.contains("update the application"));

        // 实际版本更低（中文）：提示旧版本应用创建，无法打开。
        let older = ErrorCode::DataVersionMismatch {
            expected,
            actual: DataVersion { major: 1, minor: 2, patch: 3 },
        };
        let text = message(Locale::Chinese, &Database::Metadata, &older);
        assert!(text.contains("元数据数据库"));
        assert!(text.contains("1.2.3"));
        assert!(text.contains("旧版本"));

        // 实际版本更低（英文）：对称文案。
        let text = message(Locale::English, &Database::Preference, &older);
        assert!(text.contains("preference database"));
        assert!(text.contains("older version"));
    }

    /// 覆盖数据版本表行数异常与其它错误（兜底）的正文构造。
    #[test]
    fn test_message_invalid_file_and_fallback() {
        // 行数异常（中英）：提示文件无效或损坏，含数据库名。
        let text = message(Locale::Chinese, &Database::Metadata, &ErrorCode::NoDataVersion);
        assert!(text.contains("元数据数据库"));
        assert!(text.contains("无效或已损坏"));
        let text = message(Locale::English, &Database::Preference, &ErrorCode::MultipleDataVersion);
        assert!(text.contains("preference database"));
        assert!(text.contains("invalid or corrupted"));

        // 兜底（中英）：附带错误的调试详情以便用户反馈诊断。
        let other = ErrorCode::DatabaseError { detail: "disk error".to_string() };
        let text = message(Locale::Chinese, &Database::Preference, &other);
        assert!(text.contains("disk error"));
        let text = message(Locale::English, &Database::Metadata, &other);
        assert!(text.contains("Failed to initialize"));
        assert!(text.contains("disk error"));
    }
}
