//! 后端国际化模块：统一定义后端全部面向用户的固定文案，并支持中文与英文。
//!
//! 取用方式统一为 [`text`]：传入目标语言（`None` 表示使用系统当前语言），
//! 取得该语言的文本集合，再按使用方分组读取属性——固定的单行文案是该分组结构体
//! 上的字段，需要拼装参数（如数据库名与版本号）的整段消息是该结构体上的方法。
//! 于是文案键始终是某个类型上可被编译器检查的属性，而非运行期查询的字符串键。
//!
//! 语言的来源分工：启动期（启动报错）前端尚未就绪，调用方传 `None` 取系统语言；
//! 运行期（导出）的语言由前端经 command 参数传入（前端充当语言 gate），
//! 传入的语言代码经 [`Locale::from_code`] 解析，无法识别的代码按未提供处理。
//!
//! 日志与 panic 文本一律使用英文（见 .agents 日志和panic文本要求），不纳入本模块。

mod export;
mod locale;
mod startup_error;

pub use export::ExportTexts;
pub use locale::Locale;
pub use startup_error::StartupErrorTexts;

/// 某一语言的完整文本集合，按使用方分组。
pub struct Texts {
    /// 启动期错误提示文案。
    pub startup_error: StartupErrorTexts,
    /// 导出 markdown 的固定文案。
    pub export: ExportTexts,
}

/// 取得指定语言的文本集合。
///
/// # 参数
/// - `locale`：目标语言；传 `None` 时使用系统当前语言。
///
/// # 返回值
/// 返回该语言文本集合的静态引用。
pub fn text(locale: Option<Locale>) -> &'static Texts {
    match locale.unwrap_or_else(Locale::system) {
        Locale::Chinese => &TEXTS_ZH_CN,
        Locale::English => &TEXTS_EN_US,
    }
}

/// 中文文本集合。
static TEXTS_ZH_CN: Texts = Texts {
    startup_error: startup_error::CHINESE,
    export: export::CHINESE,
};

/// 英文文本集合。
static TEXTS_EN_US: Texts = Texts {
    startup_error: startup_error::ENGLISH,
    export: export::ENGLISH,
};

#[cfg(test)]
mod tests {
    use super::*;

    /// 显式传入语言时取到该语言的文本集合，且同一语言始终取到同一份静态实例
    /// （避免每次调用构造新的文案集合）。
    #[test]
    fn test_text_by_explicit_locale() {
        assert_eq!(
            text(Some(Locale::Chinese)).startup_error.title_startup_failed,
            "启动失败"
        );
        assert_eq!(
            text(Some(Locale::English)).startup_error.title_startup_failed,
            "Startup Failed"
        );
        assert_eq!(text(Some(Locale::English)).export.canvas, "Canvas: ");
        // 同一语言始终取到同一份静态实例，避免每次调用构造新的文案集合。
        assert!(std::ptr::eq(
            text(Some(Locale::Chinese)),
            text(Some(Locale::Chinese))
        ));
    }

    /// 不传入语言时取到系统当前语言对应的文本集合：与显式传入系统语言的结果是
    /// 同一份静态实例。
    #[test]
    fn test_text_defaults_to_system_locale() {
        assert!(std::ptr::eq(text(None), text(Some(Locale::system()))));
    }
}
