// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use clap::Parser;
use i_net_lib::argv::ArgV;

/// 应用程序入口函数，解析命令行参数并启动 Tauri 应用。
///
/// # 返回值
/// 无。
fn main() {
    let argv = ArgV::parse();
    i_net_lib::run(argv)
}
