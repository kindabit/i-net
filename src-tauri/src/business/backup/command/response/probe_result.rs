use serde::Serialize;

/// `restore_probe` 命令的响应：探测备份文件的有效性。
///
/// 序列化时字段名采用 snake_case 以与 Rust 命名习惯一致。
#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ProbeResult {
    /// 是否可以还原（损坏 shard 数 ≤ parity）。
    pub recoverable: bool,
    /// 损坏 shard 数。
    pub lost: usize,
    /// 可恢复上限（parity shard 数）。
    pub limit: usize,
    /// 探测通过的文件路径，前端可继续用于 [`restore`](crate::business::backup::command::restore::restore) 命令。
    pub source_path: String,
}