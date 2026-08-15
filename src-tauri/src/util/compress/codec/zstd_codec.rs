use crate::error_code::ErrorCode;

/// 使用 zstd 压缩数据。
///
/// # 参数
///
/// * `data` - 需要压缩的明文数据。
/// * `level` - 压缩等级，数值越高压缩率越好但越慢。
///
/// # 返回值
///
/// 返回压缩后的字节数组；若失败则返回 ErrorCode::FailToCompress。
pub fn compress(data: &[u8], level: i32) -> Result<Vec<u8>, ErrorCode> {
    zstd::encode_all(data, level)
        .map_err(|e| ErrorCode::FailToCompress { detail: e.to_string() })
}

/// 解压 zstd 数据。
///
/// # 参数
///
/// * `data` - 需要解压的 zstd 数据。
///
/// # 返回值
///
/// 返回解压后的字节数组；若失败则返回 ErrorCode::FailToDecompress。
pub fn decompress(data: &[u8]) -> Result<Vec<u8>, ErrorCode> {
    zstd::decode_all(data)
        .map_err(|e| ErrorCode::FailToDecompress { detail: e.to_string() })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// zstd 压缩与解压往返一致（随机数据）。
    #[test]
    fn test_zstd_roundtrip_random() {
        let data: Vec<u8> = (0..1024u32).map(|i| (i.wrapping_mul(251) % 256) as u8).collect();
        let compressed = compress(&data, 19).unwrap();
        let decompressed = decompress(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    /// zstd 压缩与解压往返一致（文本数据）。
    #[test]
    fn test_zstd_roundtrip_text() {
        let data = b"Some text data for zstd compression testing. ".repeat(50);
        let compressed = compress(&data, 3).unwrap();
        let decompressed = decompress(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    /// zstd 解压失败路径：非法数据返回 FailToDecompress。
    #[test]
    fn test_zstd_decompress_invalid() {
        let bad_data = vec![0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
        let result = decompress(&bad_data);
        assert!(matches!(result, Err(ErrorCode::FailToDecompress { .. })));
    }
}
