//! Reed-Solomon 编解码与自适应分块参数计算。
//!
//! 使用 `reed_solomon_erasure` 库的 GF(2^8) 实现。
//! 该库遵循 RS 规范，要求 `data_shards + parity_shards <= 256`（`galois_8::Field::ORDER = 256`）。
//! 为安全起见，本模块保守地把总 shard 上限取作 255（`MAX_TOTAL_SHARDS`），
//! 并强制 `data_shards >= 1`、`parity_shards >= 1`。
//!
//! 备份文件 shard 区长度恒为 `(data_shards + parity_shards) * shard_size` 字节，
//! 因此任意大小（受 `shard_size` u32 上限约束）的数据目录都能被编码：
//! `shard_size = max(ceil(original_size / 253), MIN_SHARD_SIZE)` 保证
//! `data_shards <= MAX_TOTAL_SHARDS - MIN_PARITY_SHARDS = 253`。
//!
//! [`compute_shard_params`] 给定原始字节长度与冗余比例，返回 (N, M, S)：
//! - `N` 数据 shard 数，至少 1，上限 253。
//! - `M` 校验 shard 数，至少 2，且不超过 `MAX_TOTAL_SHARDS - N`。
//! - `S` 单个 shard 的字节长度（所有 shard 等长，最后一个数据 shard 末尾填充 0）。

use reed_solomon_erasure::{galois_8, ReedSolomon};

use crate::error_code::ErrorCode;

/// Reed-Solomon 编码后的 shard 参数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShardParams {
    /// 数据 shard 数。
    pub data_shards: u16,
    /// 校验 shard 数。
    pub parity_shards: u16,
    /// 单个 shard 的字节长度。
    pub shard_size: u32,
}

/// 单 shard 最小字节数（避免小文件下分块粒度过细导致编码开销占比过大）。
const MIN_SHARD_SIZE: u64 = 4 * 1024;

/// Reed-Solomon GF(2^8) 域保守上限（255）。`reed_solomon_erasure` 实际允许 256，
/// 这里取 255 以保留一个安全余量。
const MAX_TOTAL_SHARDS: u16 = 255;

/// 校验 shard 最小数量（保证即使小文件也能恢复至少 2 个损坏 shard）。
const MIN_PARITY_SHARDS: u16 = 2;

/// 给定原始字节长度与冗余比例，按自适应策略返回 shard 参数。
///
/// # 参数
/// - `original_size`：待编码的原始字节数。
/// - `redundancy_ratio`：冗余比例，`0 < r < 1`，如 0.05 表示增加 5% 体积。
///
/// # 返回值
/// 成功时返回 [`ShardParams`]；参数非法或超出物理容量上限时返回 `ErrorCode`。
pub fn compute_shard_params(
    original_size: u64,
    redundancy_ratio: f32,
) -> Result<ShardParams, ErrorCode> {
    if !(redundancy_ratio > 0.0 && redundancy_ratio < 1.0) {
        return Err(ErrorCode::InvalidBackupFile {
            detail: format!("redundancy_ratio out of range: {}", redundancy_ratio),
        });
    }

    // 0 字节特殊处理：返回最小的合规参数集（1 数据 shard + 2 校验 shard，shard 取最小值）。
    // 注意：shard_size 必须 ≥ 1，否则 Header::from_bytes 会拒绝（防止除零或无意义参数）。
    if original_size == 0 {
        return Ok(ShardParams {
            data_shards: 1,
            parity_shards: MIN_PARITY_SHARDS,
            shard_size: MIN_SHARD_SIZE as u32,
        });
    }

    // 正确性优先：shard_size 满足 ceil(original_size / (MAX_TOTAL_SHARDS - MIN_PARITY_SHARDS))
    // 时 data_shards 必然不超过 MAX_TOTAL_SHARDS - MIN_PARITY_SHARDS。
    // shard_size 仅有下界（≥ MIN_SHARD_SIZE），无显式上界；
    // 物理上限受 u32 限制——一旦 shard_size 超 u32::MAX，直接返回 FailToPackBackup。
    let max_data_for_total = (MAX_TOTAL_SHARDS - MIN_PARITY_SHARDS) as u64;
    let shard_size = original_size
        .div_ceil(max_data_for_total)
        .max(MIN_SHARD_SIZE);
    if shard_size > u32::MAX as u64 {
        return Err(ErrorCode::FailToPackBackup {
            detail: format!("data too large to backup: {} bytes", original_size),
        });
    }
    let data_shards = original_size.div_ceil(shard_size).max(1) as u16;

    // 计算校验 shard 数，至少 MIN_PARITY_SHARDS，且不超过 MAX_TOTAL_SHARDS - data_shards。
    let parity_estimate = ((data_shards as f32) * redundancy_ratio).ceil() as u16;
    let headroom = MAX_TOTAL_SHARDS.saturating_sub(data_shards);
    let parity_shards = parity_estimate
        .max(MIN_PARITY_SHARDS)
        .min(headroom.max(MIN_PARITY_SHARDS));

    Ok(ShardParams {
        data_shards,
        parity_shards,
        shard_size: shard_size as u32,
    })
}

/// 使用 Reed-Solomon GF(2^8) 将原始字节编码为 (data_shards + parity_shards) 个 shard。
///
/// 输入字节会按 `shard_size` 等长切分为 `data_shards` 块（最后一块填充 0），
/// 输出包含全部数据 shard 与校验 shard。
///
/// # 参数
/// - `data`：待编码的原始字节。
/// - `params`：shard 参数（来自 [`compute_shard_params`]）。
///
/// # 返回值
/// 成功时返回长度为 `data_shards + parity_shards` 的 shard 列表。
pub fn encode_shards(data: &[u8], params: ShardParams) -> Result<Vec<Vec<u8>>, ErrorCode> {
    let total = params.data_shards as usize + params.parity_shards as usize;
    if total > MAX_TOTAL_SHARDS as usize {
        return Err(ErrorCode::InvalidBackupFile {
            detail: format!("total shards {} exceeds GF(2^8) capacity", total),
        });
    }

    // 构造数据 shard：按 shard_size 等长切分，最后一块填充 0。
    let mut shards: Vec<Vec<u8>> = Vec::with_capacity(total);
    let shard_size = params.shard_size as usize;
    for i in 0..params.data_shards as usize {
        let start = i * shard_size;
        if start >= data.len() {
            // 末尾空 shard（理论上 data_shards 已经向上取整，仅保险处理）。
            shards.push(vec![0u8; shard_size]);
        } else {
            let end = (start + shard_size).min(data.len());
            let mut shard = Vec::with_capacity(shard_size);
            shard.extend_from_slice(&data[start..end]);
            shard.resize(shard_size, 0);
            shards.push(shard);
        }
    }
    // 预分配校验 shard 占位。
    for _ in 0..params.parity_shards as usize {
        shards.push(vec![0u8; shard_size]);
    }

    let rs = ReedSolomon::<galois_8::Field>::new(
        params.data_shards as usize,
        params.parity_shards as usize,
    )
    .map_err(|e| ErrorCode::FailToPackBackup {
        detail: format!("failed to create ReedSolomon encoder: {:?}", e),
    })?;

    // 使用 SEP 变体：数据 shard 只读、校验 shard 可写，便于流式或并行场景。
    let (data_ref, parity_ref) = shards.split_at_mut(params.data_shards as usize);
    rs.encode_sep(&data_ref.iter().collect::<Vec<_>>(), parity_ref)
        .map_err(|e| ErrorCode::FailToPackBackup {
            detail: format!("Reed-Solomon encode failed: {:?}", e),
        })?;

    Ok(shards)
}

/// 接收任意完整度（可能有部分 shard 损坏/缺失）的 shard 列表，
/// 用 Reed-Solomon 解码恢复缺失的 shard；保留未损坏的 shard 原样。
///
/// # 参数
/// - `shards`：长度为 `data_shards + parity_shards` 的 shard 数组，
///   `Some(shard)` 表示该位置内容可信，`None` 表示缺失或损坏。
/// - `params`：shard 参数。
///
/// # 返回值
/// 成功时返回完整 shard 列表；缺失 shard 数超过 `parity_shards` 时返回 `BackupTooManyShardsLost`。
pub fn reconstruct_shards(
    shards: Vec<Option<Vec<u8>>>,
    params: ShardParams,
) -> Result<Vec<Vec<u8>>, ErrorCode> {
    let total = params.data_shards as usize + params.parity_shards as usize;
    if shards.len() != total {
        return Err(ErrorCode::InvalidBackupFile {
            detail: format!(
                "shards count {} does not match expected {}",
                shards.len(),
                total
            ),
        });
    }

    // 统计缺失 shard 数。
    let missing_count = shards.iter().filter(|s| s.is_none()).count();
    if missing_count == 0 {
        return Ok(shards.into_iter().map(|s| s.unwrap()).collect());
    }
    if missing_count > params.parity_shards as usize {
        return Err(ErrorCode::BackupTooManyShardsLost {
            lost: missing_count,
            recoverable: params.parity_shards as usize,
        });
    }

    let rs = ReedSolomon::<galois_8::Field>::new(
        params.data_shards as usize,
        params.parity_shards as usize,
    )
    .map_err(|e| ErrorCode::FailToUnpackBackup {
        detail: format!("failed to create ReedSolomon decoder: {:?}", e),
    })?;

    // `ReconstructShard<Field>` 在 reed-solomon-erasure 6.0 中为 `Option<T>` 实现。
    // 缺失 shard 用 `None` 表示，已知 shard 用 `Some(Vec<u8>)` 表示。
    let mut rshards: Vec<Option<Vec<u8>>> = shards;

    rs.reconstruct(&mut rshards)
        .map_err(|e| ErrorCode::FailToUnpackBackup {
            detail: format!("Reed-Solomon reconstruct failed: {:?}", e),
        })?;

    // reconstruct 成功后所有 shard 都已恢复为 Some，unwrap 取出。
    let restored: Vec<Vec<u8>> = rshards
        .into_iter()
        .map(|opt| opt.expect("reconstruct must populate all shards on success"))
        .collect();

    Ok(restored)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 覆盖 compute_shard_params 的典型输入：不同数据大小都应落在合法范围。
    #[test]
    fn test_compute_shard_params_typical_inputs() {
        for size in [
            100u64,
            1024,
            10_240,
            102_400,
            1_048_576,
            104_857_600,
            4_294_967_296,
        ] {
            let params = compute_shard_params(size, 0.05).unwrap();
            assert!(
                params.data_shards >= 1
                    && params.data_shards <= MAX_TOTAL_SHARDS - MIN_PARITY_SHARDS
            );
            assert!(params.parity_shards >= MIN_PARITY_SHARDS);
            assert!(params.shard_size as u64 >= MIN_SHARD_SIZE);
            let total = params.data_shards as u64 + params.parity_shards as u64;
            assert!(total <= MAX_TOTAL_SHARDS as u64);
            let covered = params.data_shards as u64 * params.shard_size as u64;
            assert!(covered >= size, "size={size} covered={covered}");
        }
    }

    /// 覆盖 compute_shard_params 拒绝非法的冗余比例。
    #[test]
    fn test_compute_shard_params_rejects_invalid_ratio() {
        assert!(compute_shard_params(1024, 0.0).is_err());
        assert!(compute_shard_params(1024, -0.1).is_err());
        assert!(compute_shard_params(1024, 1.0).is_err());
        assert!(compute_shard_params(1024, 1.5).is_err());
    }

    /// 覆盖 compute_shard_params 对 0 字节输入的特殊处理：
    /// 必须返回正 shard_size（≥ MIN_SHARD_SIZE），否则 Header::from_bytes 会拒绝、
    /// 整个备份文件结构不自洽。
    #[test]
    fn test_compute_shard_params_empty_input() {
        let params = compute_shard_params(0, 0.05).unwrap();
        assert_eq!(params.data_shards, 1);
        assert_eq!(params.parity_shards, MIN_PARITY_SHARDS);
        assert_eq!(params.shard_size, MIN_SHARD_SIZE as u32);
    }

    /// 覆盖 compute_shard_params 在超过物理上限的输入上报错。
    /// 阈值推导：shard_size = ceil(size / 253) > u32::MAX 当且仅当
    /// size > 253 × 4294967295 = 1_086_626_725_635（精确值）。
    #[test]
    fn test_compute_shard_params_rejects_oversized() {
        assert!(matches!(
            compute_shard_params(1_086_626_725_636u64, 0.05),
            Err(ErrorCode::FailToPackBackup { .. })
        ));
        assert!(compute_shard_params(1_086_626_725_635u64, 0.05).is_ok());
    }

    /// 覆盖 encode → reconstruct 往返：随机丢一半 shard 仍能恢复出全部原始字节。
    #[test]
    fn test_encode_reconstruct_with_random_losses() {
        let original: Vec<u8> = (0u32..4096).map(|i| (i % 251) as u8).collect();
        let params = compute_shard_params(original.len() as u64, 0.20).unwrap();
        let total = params.data_shards as usize + params.parity_shards as usize;

        let shards = encode_shards(&original, params).unwrap();
        assert_eq!(shards.len(), total);

        // 构造 partial：随机把部分 shard 标记为 None（模拟丢失或损坏）。
        let mut partial: Vec<Option<Vec<u8>>> = shards.into_iter().map(Some).collect();
        // 模拟丢失 data_shards 个中的若干 + parity 中的若干，总数不超过 parity_shards。
        let to_drop = params.parity_shards as usize;
        for s in partial.iter_mut().take(to_drop) {
            *s = None;
        }

        let restored = reconstruct_shards(partial, params).unwrap();
        assert_eq!(restored.len(), total);

        // 拼接所有数据 shard（按 shard_size 截断到 original.len()），验证字节一致。
        let mut recovered = Vec::with_capacity(original.len());
        for chunk in restored.iter().take(params.data_shards as usize) {
            recovered.extend_from_slice(chunk);
        }
        recovered.truncate(original.len());
        assert_eq!(recovered, original);
    }

    /// 覆盖 reconstruct 拒绝丢失 shard 数超过 parity 的输入。
    #[test]
    fn test_reconstruct_rejects_too_many_losses() {
        let original = vec![1u8, 2, 3, 4, 5, 6, 7, 8];
        let params = compute_shard_params(original.len() as u64, 0.5).unwrap();
        let shards = encode_shards(&original, params).unwrap();
        let mut partial: Vec<Option<Vec<u8>>> = shards.into_iter().map(Some).collect();
        // 故意多丢一个 shard（> parity_shards）。
        for s in partial.iter_mut().take(params.parity_shards as usize + 1) {
            *s = None;
        }
        assert!(matches!(
            reconstruct_shards(partial, params),
            Err(ErrorCode::BackupTooManyShardsLost { .. })
        ));
    }

    /// 覆盖 reconstruct 对全完整输入的快速返回路径。
    #[test]
    fn test_reconstruct_passes_through_when_complete() {
        let original = vec![9u8; 128];
        let params = compute_shard_params(original.len() as u64, 0.1).unwrap();
        let shards = encode_shards(&original, params).unwrap();
        let partial: Vec<Option<Vec<u8>>> = shards.iter().cloned().map(Some).collect();
        let restored = reconstruct_shards(partial, params).unwrap();
        let mut recovered = Vec::new();
        for chunk in restored.iter().take(params.data_shards as usize) {
            recovered.extend_from_slice(chunk);
        }
        recovered.truncate(original.len());
        assert_eq!(recovered, original);
    }
}
