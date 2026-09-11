//! 启动期错误提示文案：程序启动阶段数据库初始化失败时弹给终端用户的固定文案。
//!
//! 启动阶段前端尚未就绪，语言只能取系统语言，因此本组文案的调用方一律以
//! `None` 取得文本集合，不向本组文案传入语言。
//!
//! 带参数的整段消息（如数据版本不匹配的正文）是本组结构体上的方法：
//! 参数拼装留在方法内、模板字面量直接写在方法体里，既让调用方只提供数据，
//! 也让占位符与参数在编译期对齐。

use super::Locale;

/// 启动期错误提示文案集合。
pub struct StartupErrorTexts {
    /// 本组文案的语言，供带参数的整段消息选择模板。
    locale: Locale,
    /// 数据版本不匹配的对话框标题。
    pub title_data_version_mismatch: &'static str,
    /// 数据库文件无效（数据版本表行数异常）的对话框标题。
    pub title_invalid_database_file: &'static str,
    /// 其余启动失败的对话框标题。
    pub title_startup_failed: &'static str,
    /// 偏好设置数据库面向用户的显示名。
    pub database_preference: &'static str,
    /// 元数据数据库面向用户的显示名。
    pub database_metadata: &'static str,
}

impl StartupErrorTexts {
    /// 构造数据版本不匹配的对话框正文。
    ///
    /// # 参数
    /// - `database`：初始化失败的数据库的显示名。
    /// - `actual`：数据库实际的数据版本。
    /// - `expected`：当前应用程序要求的数据版本。
    /// - `actual_is_newer`：数据库实际版本是否高于应用程序要求的版本。
    ///
    /// # 返回值
    /// 返回对应语言与版本大小关系的完整正文。
    pub fn message_version_mismatch(
        &self,
        database: &str,
        actual: &str,
        expected: &str,
        actual_is_newer: bool,
    ) -> String {
        match (self.locale, actual_is_newer) {
            (Locale::Chinese, true) => format!(
                "{database}的数据版本（{actual}）高于当前应用程序支持的版本（{expected}）。\n\n该数据库由更新版本的应用程序创建，请将应用程序升级到最新版本后重试。\n\n应用程序将退出。"
            ),
            (Locale::Chinese, false) => format!(
                "{database}的数据版本（{actual}）低于当前应用程序要求的版本（{expected}）。\n\n该数据库由旧版本的应用程序创建，当前版本的应用程序无法打开。\n\n应用程序将退出。"
            ),
            (Locale::English, true) => format!(
                "The data version of the {database} ({actual}) is newer than the version supported by this application ({expected}).\n\nThe database was created by a newer version of the application. Please update the application to the latest version and try again.\n\nThe application will now exit."
            ),
            (Locale::English, false) => format!(
                "The data version of the {database} ({actual}) is older than the version required by this application ({expected}).\n\nThe database was created by an older version of the application and cannot be opened by this version.\n\nThe application will now exit."
            ),
        }
    }

    /// 构造数据库文件无效的对话框正文。
    ///
    /// # 参数
    /// - `database`：初始化失败的数据库的显示名。
    ///
    /// # 返回值
    /// 返回对应语言的完整正文。
    pub fn message_invalid_file(&self, database: &str) -> String {
        match self.locale {
            Locale::Chinese => {
                format!("{database}文件无效或已损坏，应用程序无法启动。\n\n应用程序将退出。")
            }
            Locale::English => format!(
                "The {database} file is invalid or corrupted, and the application cannot start.\n\nThe application will now exit."
            ),
        }
    }

    /// 构造初始化失败的兜底正文（理论上不应出现的错误，附带调试详情供用户反馈诊断）。
    ///
    /// # 参数
    /// - `database`：初始化失败的数据库的显示名。
    /// - `error`：初始化失败返回的错误码的调试详情。
    ///
    /// # 返回值
    /// 返回对应语言的完整正文。
    pub fn message_initialize_failed(&self, database: &str, error: &str) -> String {
        match self.locale {
            Locale::Chinese => {
                format!("{database}初始化失败：{error}\n\n应用程序将退出。")
            }
            Locale::English => format!(
                "Failed to initialize the {database}: {error}\n\nThe application will now exit."
            ),
        }
    }
}

/// 中文启动期错误提示文案。
pub(crate) const CHINESE: StartupErrorTexts = StartupErrorTexts {
    locale: Locale::Chinese,
    title_data_version_mismatch: "数据版本不兼容",
    title_invalid_database_file: "数据库文件无效",
    title_startup_failed: "启动失败",
    database_preference: "偏好设置数据库",
    database_metadata: "元数据数据库",
};

/// 英文启动期错误提示文案。
pub(crate) const ENGLISH: StartupErrorTexts = StartupErrorTexts {
    locale: Locale::English,
    title_data_version_mismatch: "Incompatible Data Version",
    title_invalid_database_file: "Invalid Database File",
    title_startup_failed: "Startup Failed",
    database_preference: "preference database",
    database_metadata: "metadata database",
};

#[cfg(test)]
mod tests {
    use super::*;

    /// 覆盖中文与英文两种语言下带参数的整段消息的拼装，确认两种语言的模板都被
    /// 真实文案覆盖、且参数被正确填入正文。
    #[test]
    fn test_message_composition_both_locales() {
        // 中文：数据版本不匹配的两个方向。
        let text = CHINESE.message_version_mismatch("偏好设置数据库", "9.9.9", "3.0.0", true);
        assert!(text.contains("偏好设置数据库"));
        assert!(text.contains("9.9.9"));
        assert!(text.contains("3.0.0"));
        assert!(text.contains("升级"));
        let text = CHINESE.message_version_mismatch("元数据数据库", "1.2.3", "3.0.0", false);
        assert!(text.contains("元数据数据库"));
        assert!(text.contains("旧版本"));
        // 中文：文件无效与初始化失败兜底。
        assert!(CHINESE.message_invalid_file("元数据数据库").contains("无效或已损坏"));
        assert!(CHINESE
            .message_initialize_failed("偏好设置数据库", "disk error")
            .contains("disk error"));

        // 英文：数据版本不匹配的两个方向。
        let text = ENGLISH.message_version_mismatch("metadata database", "9.9.9", "3.0.0", true);
        assert!(text.contains("metadata database"));
        assert!(text.contains("newer version"));
        assert!(text.contains("update the application"));
        let text = ENGLISH.message_version_mismatch("preference database", "1.2.3", "3.0.0", false);
        assert!(text.contains("preference database"));
        assert!(text.contains("older version"));
        // 英文：文件无效与初始化失败兜底。
        assert!(ENGLISH
            .message_invalid_file("preference database")
            .contains("invalid or corrupted"));
        let text = ENGLISH.message_initialize_failed("metadata database", "disk error");
        assert!(text.contains("Failed to initialize"));
        assert!(text.contains("disk error"));
    }
}
