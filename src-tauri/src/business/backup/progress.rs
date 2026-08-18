//! 备份/还原的进度阶段定义。
//!
//! 业务层（service）在流程推进到各阶段边界时，通过调用方注入的进度回调上报 [`Phase`]；
//! 业务层不感知事件通道。事件名与 payload 结构属于前端 IPC 契约，
//! 由 command 层负责（见 [`crate::business::backup::command::progress`]）。

use serde::Serialize;

/// 备份/还原的进度阶段。前端按 `phase` 决定提示文案。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    /// 备份：打包数据目录到 tar 流。
    BackupPack,
    /// 备份：Reed-Solomon 编码。
    BackupEncode,
    /// 备份：写入备份文件。
    BackupWrite,
    /// 还原：读取并校验 Header。
    RestoreReadHeader,
    /// 还原：校验 shard SHA-256。
    RestoreVerify,
    /// 还原：Reed-Solomon 解码（如需）。
    RestoreDecode,
    /// 还原：解压到临时目录。
    RestoreUnpack,
    /// 还原：清空当前数据目录。
    RestoreClear,
    /// 还原：将临时目录移动到数据目录。
    RestoreMove,
}
