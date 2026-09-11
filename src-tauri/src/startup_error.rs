//! 启动期错误处理模块：启动阶段（setup）数据库初始化失败时，按系统语言向终端用户
//! 展示友好的原生阻塞对话框提示，用户确认后以非零码退出进程。
//!
//! 启动阶段前端尚未就绪，错误无法经由 invoke 到达前端的受控崩溃通道
//! （use-fatal-error / FatalErrorDialog / fatal_exit），也无法从前端获取语言偏好，
//! 因此本模块向 i18n 模块传入 `None` 以取系统语言的提示文本，并直接使用 rfd 的
//! 阻塞式原生对话框（tauri-plugin-dialog 的对话框派发依赖事件循环，setup 阶段
//! 事件循环尚未启动，不可用）。

use crate::common::data_version::entity::DataVersion;
use crate::error_code::ErrorCode;
use crate::i18n::{self, Texts};

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
    /// - `text`：目标语言的文本集合。
    ///
    /// # 返回值
    /// 返回该数据库在对应语言下的显示名称。
    fn display_name(&self, text: &Texts) -> &'static str {
        match self {
            Database::Preference => text.startup_error.database_preference,
            Database::Metadata => text.startup_error.database_metadata,
        }
    }
}

/// 将数据版本格式化为 "major.minor.patch" 形式的字符串。
///
/// # 参数
/// - `version`：待格式化的数据版本。
///
/// # 返回值
/// 返回格式化后的版本字符串。
fn format_version(version: &DataVersion) -> String {
    format!("{}.{}.{}", version.major, version.minor, version.patch)
}

/// 构造启动错误对话框的标题。
///
/// # 参数
/// - `text`：目标语言的文本集合。
/// - `error`：初始化失败返回的错误码。
///
/// # 返回值
/// 返回对应语言与错误类型的对话框标题。
fn title(text: &Texts, error: &ErrorCode) -> &'static str {
    match error {
        ErrorCode::DataVersionMismatch { .. } => text.startup_error.title_data_version_mismatch,
        ErrorCode::NoDataVersion | ErrorCode::MultipleDataVersion => {
            text.startup_error.title_invalid_database_file
        }
        _ => text.startup_error.title_startup_failed,
    }
}

/// 构造启动错误对话框的正文（面向终端用户的友好提示）。
///
/// 数据版本不匹配（用户实际会遇到的情况）给出完整的版本信息与应对建议：
/// 实际版本更高时建议升级应用程序，实际版本更低时说明数据库由旧版本应用创建。
/// 数据版本表行数异常提示数据库文件无效；其余理论上不应出现的错误附带
/// 调试详情以便用户反馈诊断。文案本身由 i18n 模块提供，本函数只负责
/// 判断错误类型与版本大小关系。
///
/// # 参数
/// - `text`：目标语言的文本集合。
/// - `database`：初始化失败的数据库。
/// - `error`：初始化失败返回的错误码。
///
/// # 返回值
/// 返回对应语言与错误类型的对话框正文。
fn message(text: &Texts, database: &Database, error: &ErrorCode) -> String {
    let database_name = database.display_name(text);
    let startup_error = &text.startup_error;
    match error {
        ErrorCode::DataVersionMismatch { expected, actual } => {
            let actual_text = format_version(actual);
            let expected_text = format_version(expected);
            let actual_is_newer = (actual.major, actual.minor, actual.patch)
                > (expected.major, expected.minor, expected.patch);
            startup_error.message_version_mismatch(
                database_name,
                &actual_text,
                &expected_text,
                actual_is_newer,
            )
        }
        ErrorCode::NoDataVersion | ErrorCode::MultipleDataVersion => {
            startup_error.message_invalid_file(database_name)
        }
        _ => startup_error.message_initialize_failed(database_name, &format!("{error:?}")),
    }
}

/// 启动期数据库初始化失败的受控退出：记录英文错误日志，按系统语言弹出
/// 原生阻塞对话框告知用户，待用户确认后以非零码退出进程。
///
/// rfd 阻塞式对话框在主线程同步弹窗，期间 setup 不返回，避免应用在数据库连接
/// 状态未初始化的情况下继续运行而再次 panic。退出走 std::process::exit，
/// 与受控崩溃一致：不触发 RunEvent::Exit 的保存逻辑，避免脏数据写盘。
/// 该函数的对话框与退出路径按设计不可单元测试（弹原生对话框并终止进程），
/// 由代码审查保证；文本构造逻辑由模块内单元测试覆盖。
///
/// # 参数
/// - `database`：初始化失败的数据库。
/// - `error`：初始化失败返回的错误码。
///
/// # 返回值
/// 无（该函数不会返回）。
pub fn abort(database: Database, error: &ErrorCode) -> ! {
    tracing::error!("failed to initialize database during startup: {error:?}");
    let text = i18n::text(None);
    rfd::MessageDialog::new()
        .set_title(title(text, error))
        .set_description(message(text, &database, error))
        .set_level(rfd::MessageLevel::Error)
        .show();
    std::process::exit(1);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::Locale;

    /// 覆盖标题构造的全部错误类型与语言组合。
    #[test]
    fn test_title_all_kinds() {
        let mismatch = ErrorCode::DataVersionMismatch {
            expected: DataVersion { major: 3, minor: 0, patch: 0 },
            actual: DataVersion { major: 9, minor: 9, patch: 9 },
        };
        let zh = i18n::text(Some(Locale::Chinese));
        let en = i18n::text(Some(Locale::English));
        // 数据版本不匹配：中英标题。
        assert_eq!(title(zh, &mismatch), "数据版本不兼容");
        assert_eq!(title(en, &mismatch), "Incompatible Data Version");
        // 数据版本表行数异常：中英标题。
        assert_eq!(title(zh, &ErrorCode::NoDataVersion), "数据库文件无效");
        assert_eq!(title(en, &ErrorCode::NoDataVersion), "Invalid Database File");
        assert_eq!(title(zh, &ErrorCode::MultipleDataVersion), "数据库文件无效");
        // 其它错误（兜底）：中英标题。
        let other = ErrorCode::DatabaseError { detail: "boom".to_string() };
        assert_eq!(title(zh, &other), "启动失败");
        assert_eq!(title(en, &other), "Startup Failed");
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
        let text = message(
            i18n::text(Some(Locale::Chinese)),
            &Database::Preference,
            &newer,
        );
        assert!(text.contains("偏好设置数据库"));
        assert!(text.contains("9.9.9"));
        assert!(text.contains("3.0.0"));
        assert!(text.contains("升级"));

        // 实际版本更高（英文）：对称文案。
        let text = message(
            i18n::text(Some(Locale::English)),
            &Database::Metadata,
            &newer,
        );
        assert!(text.contains("metadata database"));
        assert!(text.contains("newer version"));
        assert!(text.contains("update the application"));

        // 实际版本更低（中文）：提示旧版本应用创建，无法打开。
        let older = ErrorCode::DataVersionMismatch {
            expected,
            actual: DataVersion { major: 1, minor: 2, patch: 3 },
        };
        let text = message(
            i18n::text(Some(Locale::Chinese)),
            &Database::Metadata,
            &older,
        );
        assert!(text.contains("元数据数据库"));
        assert!(text.contains("1.2.3"));
        assert!(text.contains("旧版本"));

        // 实际版本更低（英文）：对称文案。
        let text = message(
            i18n::text(Some(Locale::English)),
            &Database::Preference,
            &older,
        );
        assert!(text.contains("preference database"));
        assert!(text.contains("older version"));
    }

    /// 覆盖数据版本表行数异常与其它错误（兜底）的正文构造。
    #[test]
    fn test_message_invalid_file_and_fallback() {
        // 行数异常（中英）：提示文件无效或损坏，含数据库名。
        let text = message(
            i18n::text(Some(Locale::Chinese)),
            &Database::Metadata,
            &ErrorCode::NoDataVersion,
        );
        assert!(text.contains("元数据数据库"));
        assert!(text.contains("无效或已损坏"));
        let text = message(
            i18n::text(Some(Locale::English)),
            &Database::Preference,
            &ErrorCode::MultipleDataVersion,
        );
        assert!(text.contains("preference database"));
        assert!(text.contains("invalid or corrupted"));

        // 兜底（中英）：附带错误的调试详情以便用户反馈诊断。
        let other = ErrorCode::DatabaseError { detail: "disk error".to_string() };
        let text = message(
            i18n::text(Some(Locale::Chinese)),
            &Database::Preference,
            &other,
        );
        assert!(text.contains("disk error"));
        let text = message(
            i18n::text(Some(Locale::English)),
            &Database::Metadata,
            &other,
        );
        assert!(text.contains("Failed to initialize"));
        assert!(text.contains("disk error"));
    }
}
