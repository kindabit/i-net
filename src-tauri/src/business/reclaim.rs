//! 全量还原后的 reclaim 业务模块。
//!
//! 当 restore 把磁盘文件替换后，持有内存 connection 的业务模块
//! （preference / metadata / user_database）的 in-memory state
//! 仍然指向被替换前的文件。如果不刷新内存就触发 save（如 exit-save），
//! 磁盘上的还原结果会被陈旧内存覆盖。
//!
//! 这里提供三个 reclaim 入口。前端在 restore 完成后依次调用，
//! 刷新内存 connection，使三个模块重新持有磁盘文件的所有权。
//!
//! 接口设计原则：每个接口只做一件事，命名与对应业务模块同名。
//!
//! 模块划分：
//! - [`command`]：面向前端的 `#[tauri::command]` 入口与 preprocess 校验。

pub mod command;