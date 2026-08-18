//! 备份打包的业务流程。
//!
//! 流程（[`pack`]）：
//! 1. 触发 `preference_save` 与 `metadata_save`。
//! 2. 递归遍历数据目录（跳过 `logs/`），按 tar 格式写入内存缓冲。
//! 3. 计算 SHA-256，按 [`compute_shard_params`] 决定 shard 参数。
//! 4. 调用 [`encode_shards`] 生成所有 shard。
//! 5. 按 `[Header | Shard 校验和表 | Shard 区]` 组装并写入目标文件。
//!
//! 各阶段边界通过调用方注入的进度回调上报 [`Phase`]，本模块不感知事件通道。

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
///
/// # 返回值
/// 成功时返回完整 tar 字节流。
pub(super) fn build_tar(data_directory: &Path) -> Result<Vec<u8>, ErrorCode> {
    let mut builder = Builder::new(Vec::new());
    append_dir_recursive(&mut builder, data_directory)?;
    Ok(builder.into_inner().map_err(|e| ErrorCode::FailToPackBackup {
        detail: format!("tar builder finish failed: {}", e),
    })?)
}

/// 递归把 `data_directory` 下的所有条目追加到 tar builder，跳过 `logs/` 子目录与符号链接。
///
/// # 参数
/// - `builder`：tar builder。
/// - `data_directory`：数据目录根，同时作为遍历起点与 tar 内相对路径基准。
fn append_dir_recursive<W: Write>(
    builder: &mut Builder<W>,
    data_directory: &Path,
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
            let mut file = File::open(path).map_err(|e| ErrorCode::FailToPackBackup {
                detail: format!("failed to open {}: {}", relative.display(), e),
            })?;
            builder
                .append_file(relative, &mut file)
                .map_err(|e| ErrorCode::FailToPackBackup {
                    detail: format!("tar append_file failed for {}: {}", relative.display(), e),
                })?;
        }
    }
    Ok(())
}

/// 计算 shard 校验和表：每块 SHA-256，返回 (N+M) × 32 字节。
pub(super) fn compute_shard_checksums(shards: &[Vec<u8>]) -> Vec<u8> {
    let mut out = Vec::with_capacity(shards.len() * 32);
    for shard in shards {
        let hash = Sha256::digest(shard);
        out.extend_from_slice(&hash);
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
///
/// # 返回值
/// 成功时返回 `Ok(())`；写入失败时返回对应的 `ErrorCode`。
pub(super) fn write_backup_file(
    target_path: &Path,
    header: &Header,
    shards: &[Vec<u8>],
    checksum_table: &[u8],
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
    for shard in shards {
        file.write_all(shard).map_err(|e| ErrorCode::FailToPackBackup {
            detail: format!("failed to write shard: {}", e),
        })?;
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
/// - `on_progress`：进度回调，流程推进到各阶段边界时以 `(Phase, 0.0/1.0)` 调用。
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
    let tar_bytes = build_tar(&data_directory)?;
    on_progress(Phase::BackupPack, 1.0);

    on_progress(Phase::BackupEncode, 0.0);
    let original_sha256 = Sha256::digest(&tar_bytes);
    let mut sha_arr = [0u8; 32];
    sha_arr.copy_from_slice(&original_sha256);
    let params = compute_shard_params(tar_bytes.len() as u64, redundancy_ratio)?;
    let shards = encode_shards(&tar_bytes, params)?;
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
    let checksum_table = compute_shard_checksums(&shards);
    write_backup_file(target_path, &header, &shards, &checksum_table)?;
    on_progress(Phase::BackupWrite, 1.0);
    Ok(())
}
