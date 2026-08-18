//! 备份还原的业务流程。
//!
//! 流程（[`unpack`]）：
//! 1. 读取 Header；非法时报 `InvalidBackupFile` / `UnsupportedBackupVersion`。
//! 2. 严格读取 shard 校验和表，容错读取 shard 区（尾部截断的 shard 按缺失处理），
//!    逐块 SHA-256 校验并标记坏块。
//! 3. 如有坏块，调用 [`reconstruct_shards`] 恢复；坏块超过 parity 报 `BackupTooManyShardsLost`。
//! 4. 拼接数据 shard → 原始字节流 → 解压到系统 temp 目录下的 `inet-restore-<pid>-<ts>/`。
//! 5. 清空数据目录（保留 `logs/`），把临时目录内容移动到数据目录。
//!
//! 临时目录**不在数据目录内**，避免被第 5 步误删；
//! 系统 temp 可能与数据目录在不同 mount point，因此移动时跨设备 fallback 到 copy+remove。
//! 临时目录由 RAII 守卫清理，任何错误路径（含解压失败、断言失败、panic）都不残留。
//!
//! 各阶段边界通过调用方注入的进度回调上报 [`Phase`]，本模块不感知事件通道。

use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use sha2::{Digest, Sha256};
use tar::Archive;
use walkdir::WalkDir;

use crate::business::backup::codec::{reconstruct_shards, ShardParams};
use crate::business::backup::format::{Header, HEADER_SIZE};
use crate::business::backup::progress::Phase;
use crate::error_code::ErrorCode;
use crate::state::path;
use crate::util::file_system_util;

/// 从 `file` 当前位置读取最多 `limit` 字节，容忍文件尾部截断（EOF 视为数据缺失而非错误）。
///
/// # 参数
/// - `file`：已打开的备份文件。
/// - `limit`：期望读取的最大字节数。
///
/// # 返回值
/// 成功时返回实际读取到的字节（长度 ≤ `limit`）；真实 IO 错误返回 `InvalidBackupFile`。
pub(super) fn read_up_to(file: &File, limit: usize) -> Result<Vec<u8>, ErrorCode> {
    let mut buf = Vec::with_capacity(limit);
    file.take(limit as u64)
        .read_to_end(&mut buf)
        .map_err(|e| ErrorCode::InvalidBackupFile {
            detail: format!("failed to read backup file: {}", e),
        })?;
    Ok(buf)
}

/// 校验 shard 区：把 `shard_bytes` 按 `shard_size` 切成至多 N+M 块并校验每块的 SHA-256。
///
/// `shard_bytes` 允许短于预期（备份文件尾部被截断的场景）：数据不完整的 shard 一律标记为缺失。
/// `checksum_table` 必须完整——校验和表紧跟 Header 存放，先于 shard 区被截断意味着
/// shard 数据已全部丢失，无恢复价值（由调用方严格读取保证）。
///
/// # 返回值
/// 返回长度为 N+M 的 `Vec<Option<Vec<u8>>>`：校验通过的为 `Some(data)`，损坏或缺失的为 `None`。
pub(super) fn verify_shard_region(
    shard_bytes: &[u8],
    params: ShardParams,
    checksum_table: &[u8],
) -> Result<Vec<Option<Vec<u8>>>, ErrorCode> {
    let total = params.data_shards as usize + params.parity_shards as usize;
    if checksum_table.len() != total * 32 {
        return Err(ErrorCode::InvalidBackupFile {
            detail: format!(
                "shard checksum table size mismatch: got {}, expected {}",
                checksum_table.len(),
                total * 32
            ),
        });
    }
    let shard_size = params.shard_size as usize;
    if shard_bytes.len() > total * shard_size {
        return Err(ErrorCode::InvalidBackupFile {
            detail: format!(
                "shard region size exceeds expectation: got {}, expected {}",
                shard_bytes.len(),
                total * shard_size
            ),
        });
    }

    let mut result = Vec::with_capacity(total);
    for i in 0..total {
        let start = i * shard_size;
        let end = start + shard_size;
        // 尾部截断导致数据不完整的 shard 直接标记为缺失，交给 Reed-Solomon 重建。
        if end > shard_bytes.len() {
            result.push(None);
            continue;
        }
        let shard = &shard_bytes[start..end];
        let expected = &checksum_table[i * 32..(i + 1) * 32];
        let actual = Sha256::digest(shard);
        if actual.as_slice() == expected {
            result.push(Some(shard.to_vec()));
        } else {
            result.push(None);
        }
    }
    Ok(result)
}

/// 生成临时目录名（用于还原时解压到系统 temp 目录）。
///
/// 包含进程 id 与毫秒时间戳，避免同一进程并发还原或短时间内多次还原冲突。
fn temp_directory_name() -> String {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let pid = std::process::id();
    format!("inet-restore-{}-{}", pid, ts)
}

/// 临时目录守卫：作用域结束（含错误路径）时尽力递归删除目录，避免还原失败残留。
struct TempDirGuard {
    path: PathBuf,
}

impl TempDirGuard {
    /// 创建守卫；不创建目录本身，目录由后续解压步骤创建。
    fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// 把 tar 流解压到指定目录。
fn extract_tar_to(tar_bytes: &[u8], target: &Path) -> Result<(), ErrorCode> {
    file_system_util::create_dir_all(target)?;
    let mut archive = Archive::new(tar_bytes);
    archive.unpack(target).map_err(|e| ErrorCode::FailToUnpackBackup {
        detail: format!("failed to unpack tar: {}", e),
    })?;
    Ok(())
}

/// 清空数据目录（保留 `logs/` 子目录）。
fn clear_data_directory_keeping_logs(data_directory: &Path) -> Result<(), ErrorCode> {
    let entries = file_system_util::read_dir(data_directory)?;
    for entry in entries {
        let entry = entry.map_err(|e| ErrorCode::FailToUnpackBackup {
            detail: format!("failed to iterate data directory: {}", e),
        })?;
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name == "logs" {
            continue;
        }
        let file_type = entry.file_type().map_err(|e| ErrorCode::FailToUnpackBackup {
            detail: format!("failed to read file_type: {}", e),
        })?;
        if file_type.is_dir() {
            file_system_util::remove_dir_all(&path)?;
        } else if file_type.is_file() {
            file_system_util::remove_file(&path)?;
        }
    }
    Ok(())
}

/// 把临时目录的内容移动到数据目录。
///
/// 临时目录位于系统 temp（可能与数据目录不在同一 mount point），
/// 因此递归遍历 `temp_dir`，对每个文件优先 `rename`（同设备 atomic 改名），
/// 失败时（典型场景：跨 mount point）fallback 到 copy + remove，以支持跨设备的场景；
/// 每个目录在数据目录对应位置 `create_dir_all`。
/// 临时目录本身的清理由调用方的 [`TempDirGuard`] 负责，不在此函数内删除。
fn move_temp_into_data_directory(temp_dir: &Path, data_directory: &Path) -> Result<(), ErrorCode> {
    let entries: Vec<_> = WalkDir::new(temp_dir).into_iter().collect();
    tracing::info!(
        "move_temp_into_data_directory: {} -> {} (entries={})",
        temp_dir.display(),
        data_directory.display(),
        entries.len()
    );
    for entry in entries {
        let entry = entry.map_err(|e| ErrorCode::FailToUnpackBackup {
            detail: format!("failed to iterate temp dir: {}", e),
        })?;
        // 跳过根目录本身，避免在 data 目录创建空目录条目。
        if entry.depth() == 0 {
            continue;
        }
        let src = entry.path();
        let relative = src.strip_prefix(temp_dir).map_err(|e| ErrorCode::FailToUnpackBackup {
            detail: format!("failed to strip prefix: {}", e),
        })?;
        let dst = data_directory.join(relative);
        let file_type = entry.file_type();
        // 跳过符号链接，按项目约定。
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            file_system_util::create_dir_all(&dst)?;
        } else if file_type.is_file() {
            // 优先 rename；跨设备时 fallback 到 copy + remove。
            if std::fs::rename(&src, &dst).is_err() {
                tracing::warn!(
                    "rename FAILED, falling back to copy+remove: {} -> {}",
                    src.display(),
                    dst.display()
                );
                std::fs::copy(&src, &dst).map_err(|e| ErrorCode::FailToUnpackBackup {
                    detail: format!(
                        "failed to copy {} -> {}: {}",
                        src.display(),
                        dst.display(),
                        e
                    ),
                })?;
                file_system_util::remove_file(&src)?;
            }
        }
    }
    Ok(())
}

/// 执行完整的还原流程：读取 → 校验 → 解码 → 解压 → 替换。
///
/// # 参数
/// - `source_path`：备份文件路径。
/// - `on_progress`：进度回调，流程推进到各阶段边界时以 `(Phase, 0.0/1.0)` 调用。
///
/// # 返回值
/// 成功时返回 `Ok(())`；校验失败、解码失败、解压失败、IO 失败时返回对应的 `ErrorCode`。
pub fn unpack(source_path: &Path, on_progress: &dyn Fn(Phase, f32)) -> Result<(), ErrorCode> {
    let data_directory = path().data_directory;

    // 1. 读取 Header。
    on_progress(Phase::RestoreReadHeader, 0.0);
    let mut file = File::open(source_path).map_err(|e| ErrorCode::InvalidBackupFile {
        detail: format!("failed to open backup file: {}", e),
    })?;
    let mut header_bytes = [0u8; HEADER_SIZE];
    file.read_exact(&mut header_bytes)
        .map_err(|e| ErrorCode::InvalidBackupFile {
            detail: format!("failed to read header: {}", e),
        })?;
    let header = Header::from_bytes(&header_bytes)?;
    on_progress(Phase::RestoreReadHeader, 1.0);

    // 2. 顺序读取校验和表与 shard 区（校验和表紧跟 Header，shard 区在其后直至文件尾）。
    //    校验和表严格读取：尾部截断若侵入校验和表，shard 数据必然已全部丢失，无恢复价值；
    //    shard 区容错读取：尾部截断按缺失 shard 处理，由 Reed-Solomon 冗余兜底。
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

    // 3. 逐块 SHA-256 校验。
    on_progress(Phase::RestoreVerify, 0.0);
    let verified = verify_shard_region(&shard_bytes, params, &checksum_table)?;
    on_progress(Phase::RestoreVerify, 1.0);

    // 4. 解码坏块。
    let mut missing = 0usize;
    for s in verified.iter() {
        if s.is_none() {
            missing += 1;
        }
    }
    let shards = if missing > 0 {
        on_progress(Phase::RestoreDecode, 0.0);
        let restored = reconstruct_shards(verified, params)?;
        on_progress(Phase::RestoreDecode, 1.0);
        restored
    } else {
        verified
            .into_iter()
            .map(|s| s.unwrap_or_default())
            .collect::<Vec<_>>()
    };

    // 5. 拼接数据 shard。
    let mut recovered_tar = Vec::with_capacity(header.original_size as usize);
    for chunk in shards.iter().take(header.data_shards as usize) {
        recovered_tar.extend_from_slice(chunk);
    }
    recovered_tar.truncate(header.original_size as usize);

    // 6. 校验整体 SHA-256。
    let actual_hash = Sha256::digest(&recovered_tar);
    if actual_hash.as_slice() != header.original_sha256 {
        return Err(ErrorCode::FailToUnpackBackup {
            detail: "recovered tar stream hash mismatch".to_string(),
        });
    }

    // 7. 解压到系统 temp 目录下的临时目录（不在 data 目录内，避免被 step 8 误删）；
    //    RAII 守卫负责清理临时目录，无论本函数早退与否都不残留。
    let temp_dir = std::env::temp_dir().join(temp_directory_name());
    let _temp_guard = TempDirGuard::new(temp_dir.clone());
    on_progress(Phase::RestoreUnpack, 0.0);
    extract_tar_to(&recovered_tar, &temp_dir)?;
    on_progress(Phase::RestoreUnpack, 1.0);

    // 8. 清空数据目录（保留 logs/）。
    on_progress(Phase::RestoreClear, 0.0);
    clear_data_directory_keeping_logs(&data_directory)?;
    on_progress(Phase::RestoreClear, 1.0);

    // 9. 移动临时目录内容到数据目录（跨设备场景会 fallback 到 copy+remove）。
    on_progress(Phase::RestoreMove, 0.0);
    move_temp_into_data_directory(&temp_dir, &data_directory)?;
    on_progress(Phase::RestoreMove, 1.0);

    Ok(())
}
