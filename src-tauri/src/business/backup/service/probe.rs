//! 备份探测的业务流程。
//!
//! 探测备份文件的有效性（不解压、不替换数据目录），仅返回校验结论。
//! 读取约定与还原链路一致：校验和表严格读取，shard 区容错读取（尾部截断按缺失 shard 处理）。

use std::fs::File;
use std::io::Read;
use std::path::Path;

use crate::business::backup::codec::ShardParams;
use crate::business::backup::format::{Header, HEADER_SIZE};
use crate::business::backup::service::unpack::{read_up_to, verify_shard_region};
use crate::error_code::ErrorCode;

/// 探测备份文件的有效性（不解压），仅返回校验结论。
///
/// # 返回值
/// 返回 `(recoverable, lost, recoverable_limit)`：
/// - `recoverable`：是否可还原（损坏 shard 数 ≤ parity）。
/// - `lost`：损坏 shard 数。
/// - `recoverable_limit`：可恢复的上限（parity_shards）。
pub fn probe(source_path: &Path) -> Result<(bool, usize, usize), ErrorCode> {
    let mut file = File::open(source_path).map_err(|e| ErrorCode::InvalidBackupFile {
        detail: format!("failed to open backup file: {}", e),
    })?;
    let mut header_bytes = [0u8; HEADER_SIZE];
    file.read_exact(&mut header_bytes)
        .map_err(|e| ErrorCode::InvalidBackupFile {
            detail: format!("failed to read header: {}", e),
        })?;
    let header = Header::from_bytes(&header_bytes)?;

    // 紧跟 Header 严格读取校验和表，再容错读取 shard 区（尾部截断按缺失 shard 处理）。
    let mut checksum_table = vec![0u8; header.shard_checksum_table_size()];
    file.read_exact(&mut checksum_table)
        .map_err(|e| ErrorCode::InvalidBackupFile {
            detail: format!("failed to read shard checksum table: {}", e),
        })?;
    let shard_bytes = read_up_to(&file, header.shard_region_size())?;

    let params = ShardParams {
        data_shards: header.data_shards,
        parity_shards: header.parity_shards,
        shard_size: header.shard_size,
    };
    let verified = verify_shard_region(&shard_bytes, params, &checksum_table)?;
    let lost = verified.iter().filter(|s| s.is_none()).count();
    let recoverable_limit = header.parity_shards as usize;
    let recoverable = lost <= recoverable_limit;
    Ok((recoverable, lost, recoverable_limit))
}
