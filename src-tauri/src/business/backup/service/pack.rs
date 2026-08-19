//! 备份打包的业务流程。
//!
//! 流程（[`pack`]）：
//! 1. 触发 `preference_save` 与 `metadata_save`。
//! 2. 递归遍历数据目录（跳过 `logs/`），按 tar 格式写入内存缓冲。
//! 3. 计算 SHA-256，按 [`compute_shard_params`] 决定 shard 参数。
//! 4. 调用 [`encode_shards`] 生成所有 shard。
//! 5. 按 `[Header | Shard 校验和表 | Shard 区]` 组装并写入目标文件。
//!
//! 各阶段边界与阶段内粒度进度通过调用方注入的进度回调上报 [`Phase`]，本模块不感知事件通道。

use std::fs::File;
use std::io::Write;
use std::path::Path;

use sha2::{Digest, Sha256};
use tar::Builder;
use walkdir::WalkDir;

use crate::business::backup::codec::{compute_shard_params, encode_shards};
use crate::business::backup::format::Header;
use crate::business::backup::progress::Phase;
use crate::business::metadata::service as metadata_service;
use crate::business::preference::service as preference_service;
use crate::error_code::ErrorCode;
use crate::state::path;
use crate::util::file_system_util;

use super::data_directory_size::data_directory_size;

/// 在打包前持久化 preference 与 metadata，避免漏写最近修改。
fn persist_state() -> Result<(), ErrorCode> {
    preference_service::save().map_err(|e| ErrorCode::FailToPackBackup {
        detail: format!("failed to save preference: {:?}", e),
    })?;
    metadata_service::save().map_err(|e| ErrorCode::FailToPackBackup {
        detail: format!("failed to save metadata: {:?}", e),
    })?;
    Ok(())
}

/// 把数据目录内除 `logs/` 外的所有文件打包成 tar 字节流。
///
/// # 参数
/// - `data_directory`：应用数据根目录。
/// - `on_progress`：打包进度回调，按 已处理文件字节 / 预扫描总字节 上报，
///   值域 `(0.0, 1.0]`，逐文件单调递增。
///
/// # 返回值
/// 成功时返回完整 tar 字节流。
pub(super) fn build_tar(data_directory: &Path, on_progress: &dyn Fn(f32)) -> Result<Vec<u8>, ErrorCode> {
    // 预扫描总字节数作为进度分母（仅元数据遍历，开销远小于读取文件内容）；
    // 复用 data_directory_size，其过滤规则（跳过 logs/、不跟随符号链接）与打包遍历一致。
    let total_bytes = data_directory_size(data_directory)?;
    let mut builder = Builder::new(Vec::new());
    let mut processed_bytes = 0u64;
    append_dir_recursive(&mut builder, data_directory, total_bytes, &mut processed_bytes, on_progress)?;
    Ok(builder.into_inner().map_err(|e| ErrorCode::FailToPackBackup {
        detail: format!("tar builder finish failed: {}", e),
    })?)
}

/// 递归把 `data_directory` 下的所有条目追加到 tar builder，跳过 `logs/` 子目录与符号链接。
///
/// # 参数
/// - `builder`：tar builder。
/// - `data_directory`：数据目录根，同时作为遍历起点与 tar 内相对路径基准。
/// - `total_bytes`：预扫描得到的非 `logs/` 文件总字节数，作为进度分母。
/// - `processed_bytes`：累计已写入 tar 的文件字节数（可由外层持有，递归间共享）。
/// - `on_progress`：进度回调，每个文件处理完后以 `(processed/total_bytes)` 钳制到 1.0 上报。
fn append_dir_recursive<W: Write>(
    builder: &mut Builder<W>,
    data_directory: &Path,
    total_bytes: u64,
    processed_bytes: &mut u64,
    on_progress: &dyn Fn(f32),
) -> Result<(), ErrorCode> {
    let iter = WalkDir::new(data_directory).into_iter().filter_entry(|e| {
        // 根目录自身不参与过滤；其余条目按文件名剔除 logs 子目录。
        if e.depth() == 0 {
            return true;
        }
        e.file_name() != "logs"
    });
    for entry in iter {
        let entry = entry.map_err(|e| ErrorCode::FailToPackBackup {
            detail: format!("failed to iterate directory entry: {}", e),
        })?;
        // 跳过根目录本身，避免向 tar 写入空目录条目。
        if entry.depth() == 0 {
            continue;
        }
        let path = entry.path();
        let file_type = entry.file_type();
        // 跳过软链接，防止解压时跨越数据目录。
        if file_type.is_symlink() {
            continue;
        }

        let relative = path
            .strip_prefix(data_directory)
            .map_err(|e| ErrorCode::FailToPackBackup {
                detail: format!("failed to strip prefix: {}", e),
            })?;

        if file_type.is_dir() {
            builder
                .append_dir(relative, path)
                .map_err(|e| ErrorCode::FailToPackBackup {
                    detail: format!("tar append_dir failed for {}: {}", relative.display(), e),
                })?;
        } else if file_type.is_file() {
            let file_len = entry.metadata().map_err(|e| ErrorCode::FailToPackBackup {
                detail: format!("failed to read metadata for {}: {}", relative.display(), e),
            })?.len();
            let mut file = File::open(path).map_err(|e| ErrorCode::FailToPackBackup {
                detail: format!("failed to open {}: {}", relative.display(), e),
            })?;
            builder
                .append_file(relative, &mut file)
                .map_err(|e| ErrorCode::FailToPackBackup {
                    detail: format!("tar append_file failed for {}: {}", relative.display(), e),
                })?;
            *processed_bytes += file_len;
            // total_bytes 为 0（空目录）时跳过避免除零；文件在扫描后可能变大，故钳制到 1.0。
            if total_bytes > 0 {
                on_progress((*processed_bytes as f32 / total_bytes as f32).min(1.0));
            }
        }
    }
    Ok(())
}

/// 计算 shard 校验和表：每块 SHA-256，返回 (N+M) × 32 字节。
///
/// # 参数
/// - `shards`：完整 shard 列表。
/// - `on_progress`：进度回调，每块 SHA-256 完成后以 `(i+1)/shards.len()` 调用，
///   首值与末值均落在 `(0.0, 1.0]`。`shards.len() >= 3`（`data >= 1` + `parity >= 2`），无除零风险。
pub(super) fn compute_shard_checksums(shards: &[Vec<u8>], on_progress: &dyn Fn(f32)) -> Vec<u8> {
    let mut out = Vec::with_capacity(shards.len() * 32);
    for (i, shard) in shards.iter().enumerate() {
        let hash = Sha256::digest(shard);
        out.extend_from_slice(&hash);
        // shards.len() 恒 >= 3（data >= 1 + parity >= 2），无除零风险。
        on_progress((i + 1) as f32 / shards.len() as f32);
    }
    out
}

/// 把完整 shard 列表写成备份文件（顺序：Header → shard 校验和表 → 各 shard）。
///
/// 校验和表先于 shard 区写入，使备份文件尾部仅含 shard 数据：
/// 写入中断造成的尾部缺失只损失 shard，校验和表保持完整可读（见 `format.rs` 布局说明）。
///
/// # 参数
/// - `target_path`：备份文件目标路径。
/// - `header`：已填充的 [`Header`]。
/// - `shards`：长度为 `data_shards + parity_shards` 的 shard 列表。
/// - `checksum_table`：长度为 `(data_shards + parity_shards) * 32` 的校验和表。
/// - `on_progress`：进度回调，按 已写 shard 字节 / shard 区总字节 上报，
///   值域 `(0.0, 1.0]`，逐 shard 单调递增。Header 与校验和表体积小、不计入进度。
///
/// # 返回值
/// 成功时返回 `Ok(())`；写入失败时返回对应的 `ErrorCode`。
pub(super) fn write_backup_file(
    target_path: &Path,
    header: &Header,
    shards: &[Vec<u8>],
    checksum_table: &[u8],
    on_progress: &dyn Fn(f32),
) -> Result<(), ErrorCode> {
    if let Some(parent) = target_path.parent() {
        file_system_util::create_dir_all(parent)?;
    }
    let mut file = File::create(target_path).map_err(|e| ErrorCode::FailToPackBackup {
        detail: format!("failed to create backup file: {}", e),
    })?;
    file.write_all(&header.to_bytes())
        .map_err(|e| ErrorCode::FailToPackBackup {
            detail: format!("failed to write header: {}", e),
        })?;
    file.write_all(checksum_table)
        .map_err(|e| ErrorCode::FailToPackBackup {
            detail: format!("failed to write shard checksum table: {}", e),
        })?;
    let total_shard_bytes = shards.iter().map(|s| s.len() as u64).sum::<u64>();
    let mut written = 0u64;
    for shard in shards {
        file.write_all(shard).map_err(|e| ErrorCode::FailToPackBackup {
            detail: format!("failed to write shard: {}", e),
        })?;
        written += shard.len() as u64;
        if total_shard_bytes > 0 {
            on_progress((written as f32 / total_shard_bytes as f32).min(1.0));
        }
    }
    file.flush().map_err(|e| ErrorCode::FailToPackBackup {
        detail: format!("failed to flush backup file: {}", e),
    })?;
    Ok(())
}

/// 执行完整的备份流程：持久化 → 打包 → 编码 → 写入目标文件。
///
/// # 参数
/// - `target_path`：用户选定的备份文件路径。
/// - `redundancy_ratio`：冗余比例，范围 `0 < r < 1`。
/// - `on_progress`：进度回调。本函数在每个阶段边界以 `(Phase, 0.0)` 与 `(Phase, 1.0)` 调用，
///   阶段内按工作量单调上报中间值；中间进度语义详见 [`Phase`] 各阶段实现。
///
/// # 返回值
/// 成功时返回 `Ok(())`；任意阶段失败时返回对应的 `ErrorCode`。
pub fn pack(
    target_path: &Path,
    redundancy_ratio: f32,
    on_progress: &dyn Fn(Phase, f32),
) -> Result<(), ErrorCode> {
    persist_state()?;

    let data_directory = path().data_directory;
    on_progress(Phase::BackupPack, 0.0);
    let tar_bytes = build_tar(&data_directory, &|v| on_progress(Phase::BackupPack, v))?;
    on_progress(Phase::BackupPack, 1.0);

    on_progress(Phase::BackupEncode, 0.0);
    // 分块计算整体 SHA-256 并上报本阶段前 HASH_WEIGHT 比例
    //（经验权重：SHA-256 为内存速度，RS 编码更慢，故编码占大头）。
    const HASH_WEIGHT: f32 = 0.3;
    const HASH_CHUNK_SIZE: usize = 4 * 1024 * 1024;
    let mut hasher = Sha256::new();
    let total_len = tar_bytes.len();
    for (i, chunk) in tar_bytes.chunks(HASH_CHUNK_SIZE).enumerate() {
        hasher.update(chunk);
        let hashed = ((i + 1) * HASH_CHUNK_SIZE).min(total_len);
        on_progress(Phase::BackupEncode, (hashed as f32 / total_len as f32) * HASH_WEIGHT);
    }
    let original_sha256 = hasher.finalize();
    let mut sha_arr = [0u8; 32];
    sha_arr.copy_from_slice(&original_sha256);
    let params = compute_shard_params(tar_bytes.len() as u64, redundancy_ratio)?;
    let shards = encode_shards(&tar_bytes, params, &|v| {
        on_progress(Phase::BackupEncode, HASH_WEIGHT + v * (1.0 - HASH_WEIGHT));
    })?;
    on_progress(Phase::BackupEncode, 1.0);

    on_progress(Phase::BackupWrite, 0.0);
    let header = Header {
        original_size: tar_bytes.len() as u64,
        data_shards: params.data_shards,
        parity_shards: params.parity_shards,
        shard_size: params.shard_size,
        redundancy_ratio,
        original_sha256: sha_arr,
    };
    // 校验和表是对全部 shard 的第二轮 SHA-256，占本阶段前 CHECKSUM_WEIGHT；
    // 写盘占其余（经验权重）。
    const CHECKSUM_WEIGHT: f32 = 0.2;
    let checksum_table = compute_shard_checksums(&shards, &|v| {
        on_progress(Phase::BackupWrite, v * CHECKSUM_WEIGHT);
    });
    write_backup_file(target_path, &header, &shards, &checksum_table, &|v| {
        on_progress(Phase::BackupWrite, CHECKSUM_WEIGHT + v * (1.0 - CHECKSUM_WEIGHT));
    })?;
    on_progress(Phase::BackupWrite, 1.0);
    Ok(())
}
