//! 备份文件格式定义。
//!
//! 备份文件采用自定义二进制格式，前 64 字节是固定大小的 Header，
//! 之后是 (N+M) 条 32 字节 SHA-256 校验和，最后是 N 个数据 shard 与 M 个校验 shard
//! （顺序排列，每个 shard 填充到 `shard_size` 字节）。
//! `original_size` 仍表示原始字节流的总长度，用于还原时按需截断与整体 SHA-256 终验。
//!
//! 校验和表前置的设计意图：备份文件写入中断（如磁盘写满）造成的尾部缺失只影响 shard 区，
//! 校验和表始终完整可读；还原端对 shard 区做容错读取，把数据不完整的 shard 按缺失处理，
//! 缺失数不超过 `parity_shards` 时经 Reed-Solomon 恢复（即可容忍的尾部缺失阈值为
//! `parity_shards * shard_size` 字节）。
//!
//! 文件结构：
//! ```text
//! +--------------+------------------------+------------------+
//! | Header (64B) | Shard 校验和表（变长） | Shard 区（变长） |
//! +--------------+------------------------+------------------+
//! ```
//!
//! 文件头部写入固定 8 字节的 magic `"IBACKUP\0"`，防止普通 tar 工具误识别，
//! 前端文件选择对话框建议使用 `.ibackup` 扩展名以增强可见性。

use crate::error_code::ErrorCode;

/// 备份文件 magic 字符串，长度 8 字节（含末尾 `\0`）。
pub const MAGIC: &[u8; 8] = b"IBACKUP\0";

/// 当前备份文件格式版本号。
pub const VERSION: u16 = 1;

/// Header 固定大小。
pub const HEADER_SIZE: usize = 64;

/// Header 二进制布局，按 little-endian 编码：
///
/// | 偏移 | 长度 | 字段                |
/// |------|------|---------------------|
/// | 0    | 8    | magic               |
/// | 8    | 2    | version             |
/// | 10   | 8    | original_size       |
/// | 18   | 2    | data_shards         |
/// | 20   | 2    | parity_shards       |
/// | 22   | 4    | shard_size          |
/// | 26   | 4    | redundancy_ratio    |
/// | 30   | 32   | original_sha256     |
/// | 62   | 2    | 保留（填 0）        |
#[derive(Debug, Clone, PartialEq)]
pub struct Header {
    /// 原始字节流的总字节数。
    pub original_size: u64,
    /// 数据 shard 数。
    pub data_shards: u16,
    /// 校验 shard 数。
    pub parity_shards: u16,
    /// 每个 shard 的字节长度（最后一个 shard 也按此长度填充）。
    pub shard_size: u32,
    /// 备份时使用的冗余比例（f32 LE）。
    pub redundancy_ratio: f32,
    /// 原始字节流的 SHA-256，用于还原后做最终完整性校验。
    pub original_sha256: [u8; 32],
}

impl Header {
    /// 将 Header 序列化为固定 64 字节。
    pub fn to_bytes(&self) -> [u8; HEADER_SIZE] {
        let mut buf = [0u8; HEADER_SIZE];
        buf[0..8].copy_from_slice(MAGIC);
        buf[8..10].copy_from_slice(&VERSION.to_le_bytes());
        buf[10..18].copy_from_slice(&self.original_size.to_le_bytes());
        buf[18..20].copy_from_slice(&self.data_shards.to_le_bytes());
        buf[20..22].copy_from_slice(&self.parity_shards.to_le_bytes());
        buf[22..26].copy_from_slice(&self.shard_size.to_le_bytes());
        buf[26..30].copy_from_slice(&self.redundancy_ratio.to_le_bytes());
        buf[30..62].copy_from_slice(&self.original_sha256);
        // buf[62..64] 保留为 0
        buf
    }

    /// 从字节切片解析 Header；要求切片至少 64 字节。
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ErrorCode> {
        if bytes.len() < HEADER_SIZE {
            return Err(ErrorCode::InvalidBackupFile {
                detail: format!("header too short: {} bytes", bytes.len()),
            });
        }
        if &bytes[0..8] != MAGIC {
            return Err(ErrorCode::InvalidBackupFile {
                detail: "magic bytes mismatch".to_string(),
            });
        }
        let version = u16::from_le_bytes(bytes[8..10].try_into().unwrap());
        if version != VERSION {
            return Err(ErrorCode::UnsupportedBackupVersion { version });
        }
        let original_size = u64::from_le_bytes(bytes[10..18].try_into().unwrap());
        let data_shards = u16::from_le_bytes(bytes[18..20].try_into().unwrap());
        let parity_shards = u16::from_le_bytes(bytes[20..22].try_into().unwrap());
        let shard_size = u32::from_le_bytes(bytes[22..26].try_into().unwrap());
        let redundancy_ratio = f32::from_le_bytes(bytes[26..30].try_into().unwrap());
        let mut original_sha256 = [0u8; 32];
        original_sha256.copy_from_slice(&bytes[30..62]);
        if shard_size == 0 || data_shards == 0 || parity_shards == 0 {
            return Err(ErrorCode::InvalidBackupFile {
                detail: "header fields must be positive".to_string(),
            });
        }
        Ok(Self {
            original_size,
            data_shards,
            parity_shards,
            shard_size,
            redundancy_ratio,
            original_sha256,
        })
    }

    /// Shard 区在文件中占用的总字节数（不含 shard 校验和表）。
    pub fn shard_region_size(&self) -> usize {
        (self.data_shards as usize + self.parity_shards as usize) * self.shard_size as usize
    }

    /// Shard 校验和表在文件中占用的总字节数。
    pub fn shard_checksum_table_size(&self) -> usize {
        (self.data_shards as usize + self.parity_shards as usize) * 32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    /// 覆盖 Header 序列化的往返正确性：to_bytes → from_bytes 应得到完全相同的结构。
    #[test]
    fn test_header_round_trip() {
        let mut sha = [0u8; 32];
        let hash = Sha256::digest(b"hello");
        sha.copy_from_slice(&hash);
        let header = Header {
            original_size: 12345,
            data_shards: 100,
            parity_shards: 5,
            shard_size: 4096,
            redundancy_ratio: 0.05,
            original_sha256: sha,
        };
        let bytes = header.to_bytes();
        assert_eq!(bytes.len(), HEADER_SIZE);
        let restored = Header::from_bytes(&bytes).unwrap();
        assert_eq!(restored, header);
        assert_eq!(&bytes[0..8], MAGIC);
    }

    /// 覆盖 from_bytes 拒绝 magic 不匹配的输入。
    #[test]
    fn test_header_rejects_bad_magic() {
        let mut bytes = [0u8; HEADER_SIZE];
        bytes[0..7].copy_from_slice(b"BADMAG!");
        assert!(matches!(
            Header::from_bytes(&bytes),
            Err(ErrorCode::InvalidBackupFile { .. })
        ));
    }

    /// 覆盖 from_bytes 拒绝长度不足的输入。
    #[test]
    fn test_header_rejects_short_input() {
        let bytes = [0u8; 32];
        assert!(matches!(
            Header::from_bytes(&bytes),
            Err(ErrorCode::InvalidBackupFile { .. })
        ));
    }

    /// 覆盖 from_bytes 拒绝不兼容的版本号。
    #[test]
    fn test_header_rejects_unsupported_version() {
        let mut bytes = [0u8; HEADER_SIZE];
        bytes[0..8].copy_from_slice(MAGIC);
        bytes[8..10].copy_from_slice(&999u16.to_le_bytes());
        assert!(matches!(
            Header::from_bytes(&bytes),
            Err(ErrorCode::UnsupportedBackupVersion { .. })
        ));
    }

    /// 覆盖 from_bytes 拒绝字段为零的输入（防止除零或无意义参数）。
    #[test]
    fn test_header_rejects_zero_fields() {
        let mut bytes = [0u8; HEADER_SIZE];
        bytes[0..8].copy_from_slice(MAGIC);
        bytes[8..10].copy_from_slice(&VERSION.to_le_bytes());
        bytes[10..18].copy_from_slice(&100u64.to_le_bytes());
        bytes[18..20].copy_from_slice(&10u16.to_le_bytes()); // data_shards
        bytes[20..22].copy_from_slice(&2u16.to_le_bytes()); // parity_shards
        bytes[22..26].copy_from_slice(&0u32.to_le_bytes()); // shard_size = 0
        assert!(matches!(
            Header::from_bytes(&bytes),
            Err(ErrorCode::InvalidBackupFile { .. })
        ));
    }
}
