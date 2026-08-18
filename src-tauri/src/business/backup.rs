//! 全量备份与还原用户数据目录的业务模块。
//!
//! 数据目录除 `logs/` 外的所有文件被递归打包为 tar 流，按自适应块大小切成
//! N 个等长的数据 shard，再由 Reed-Solomon 编码产生 M 个校验 shard，
//! 一同写入备份文件。还原时按 shard 校验和识别坏块，必要时用 RS 解码恢复，
//! 然后解压到临时目录并替换原数据目录（保留 logs/），临时目录由守卫保证清理。
//!
//! 模块划分：
//! - [`codec`]：备份文件 Header 的 Reed-Solomon 编解码与自适应分块参数计算。
//! - [`format`]：备份文件 Header 的序列化与反序列化。
//! - [`progress`]：备份/还原进度上报的 Tauri Event 封装。
//! - [`service`]：打包与解包的业务流程（pack / unpack / probe / data_directory_size）。
//! - [`command`]：面向前端的 `#[tauri::command]` 入口与 preprocess 校验。

pub mod codec;
pub mod command;
pub mod format;
pub mod progress;
pub mod service;
