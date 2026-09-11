//! 国际化语言：受支持语言的枚举、语言代码解析与操作系统语言探测。

/// 国际化支持的语言。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Locale {
    /// 中文。
    Chinese,
    /// 英文（系统语言无法识别时的兜底语言）。
    English,
}

impl Locale {
    /// 将语言代码解析为受支持的语言。
    ///
    /// 以 "zh" 开头（不区分大小写）视为中文，以 "en" 开头视为英文。
    /// 其余代码（含空字符串）返回 `None`，由调用方决定回退策略。
    ///
    /// # 参数
    /// - `code`：语言代码（如 "zh-CN"、"en-US"）。
    ///
    /// # 返回值
    /// 识别成功时返回对应的受支持语言，无法识别时返回 `None`。
    pub fn from_code(code: &str) -> Option<Locale> {
        let code = code.to_lowercase();
        if code.starts_with("zh") {
            Some(Locale::Chinese)
        } else if code.starts_with("en") {
            Some(Locale::English)
        } else {
            None
        }
    }

    /// 探测操作系统当前语言。
    ///
    /// # 参数
    /// 无。
    ///
    /// # 返回值
    /// 返回操作系统语言对应的受支持语言；系统不提供语言或该语言不受支持时返回英文。
    pub fn system() -> Locale {
        Locale::from_code(&sys_locale::get_locale().unwrap_or_default())
            .unwrap_or(Locale::English)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 覆盖语言代码解析的全部规则：中文各区域变体（含大小写混写）解析为中文，
    /// 英文各区域变体解析为英文，其余代码无法识别。
    #[test]
    fn test_from_code_all_rules() {
        // 中文：简体、繁体、无区域后缀与大小写混写都解析为中文。
        assert_eq!(Locale::from_code("zh-CN"), Some(Locale::Chinese));
        assert_eq!(Locale::from_code("zh-TW"), Some(Locale::Chinese));
        assert_eq!(Locale::from_code("zh"), Some(Locale::Chinese));
        assert_eq!(Locale::from_code("ZH-cn"), Some(Locale::Chinese));
        // 英文：区域变体与无区域后缀都解析为英文。
        assert_eq!(Locale::from_code("en-US"), Some(Locale::English));
        assert_eq!(Locale::from_code("en"), Some(Locale::English));
        // 其余语言与空字符串都无法识别。
        assert_eq!(Locale::from_code("fr-FR"), None);
        assert_eq!(Locale::from_code(""), None);
    }

    /// 系统语言探测必然返回受支持的语言（中英之一），不会因系统不提供语言或
    /// 语言不受支持而失败。
    #[test]
    fn test_system_returns_supported_locale() {
        let locale = Locale::system();
        assert!(matches!(locale, Locale::Chinese | Locale::English));
    }
}
