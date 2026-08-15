use std::io::Read;

use crate::error_code::ErrorCode;

/// 使用 brotli 压缩数据。
///
/// # 参数
///
/// * `data` - 需要压缩的明文数据。
/// * `quality` - 压缩质量档位（0-11），越高质量越好但越慢。
/// * `window` - 窗口大小参数 lgwin，控制压缩窗口。
///
/// # 返回值
///
/// 返回压缩后的字节数组；若失败则返回 ErrorCode::FailToCompress。
pub fn compress(data: &[u8], quality: u32, window: u32) -> Result<Vec<u8>, ErrorCode> {
    let mut reader = brotli::CompressorReader::new(data, 4096, quality, window);
    let mut out = Vec::new();
    reader
        .read_to_end(&mut out)
        .map_err(|e| ErrorCode::FailToCompress { detail: e.to_string() })?;
    Ok(out)
}

/// 解压 brotli 数据。
///
/// # 参数
///
/// * `data` - 需要解压的 brotli 数据。
///
/// # 返回值
///
/// 返回解压后的字节数组；若失败则返回 ErrorCode::FailToDecompress。
pub fn decompress(data: &[u8]) -> Result<Vec<u8>, ErrorCode> {
    let mut reader = brotli::Decompressor::new(data, 4096);
    let mut out = Vec::new();
    reader
        .read_to_end(&mut out)
        .map_err(|e| ErrorCode::FailToDecompress { detail: e.to_string() })?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// brotli 压缩与解压往返一致（文本数据）。
    #[test]
    fn test_brotli_roundtrip_text() {
        let data = b"Hello world, this is a test of brotli compression for text data. ".repeat(100);
        let compressed = compress(&data, 11, 22).unwrap();
        assert!(compressed.len() < data.len(), "compressed should be smaller for repetitive text");
        let decompressed = decompress(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    /// brotli 压缩与解压往返一致（随机数据）。
    #[test]
    fn test_brotli_roundtrip_random() {
        let data: Vec<u8> = (0..1024u32).map(|i| (i.wrapping_mul(251) % 256) as u8).collect();
        let compressed = compress(&data, 5, 18).unwrap();
        let decompressed = decompress(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    /// brotli 解压失败路径：非法数据返回 FailToDecompress。
    #[test]
    fn test_brotli_decompress_invalid() {
        let bad_data = vec![0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
        let result = decompress(&bad_data);
        assert!(matches!(result, Err(ErrorCode::FailToDecompress { .. })));
    }
}
