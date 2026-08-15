use std::path::PathBuf;

use clap::Parser;

/// 应用程序命令行参数。
///
/// 通过 `clap` 解析，支持以下运行参数：
/// - `--data-dir <path>`：指定应用程序数据目录。
#[derive(Parser, Debug)]
#[command(name = "i-net")]
pub struct ArgV {
    /// 指定应用程序数据目录。
    ///
    /// 如果未提供，则使用系统默认的应用数据目录。
    #[arg(long = "data-dir")]
    pub data_directory: Option<PathBuf>,
}
