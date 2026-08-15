use std::io::Read;
use std::io::Write;

use crate::error_code::ErrorCode;

/// 模块使用 lzma-rust 的 raw LZMA2 流（无 XZ 容器头）。压缩解压双侧使用相同的固定 dict_size。
/// dict_size 取自 LZMA2Options::DICT_SIZE_DEFAULT（with_preset(6) 的字典大小恰为 8MB），
/// 确保压缩端实际使用的字典大小与解压端一致。
/// 警告：raw LZMA2 流不自描述 dict_size，解压端给的 dict_size 必须 ≥ 压缩端实际使用的字典大小，
/// 否则解码失败。若未来调整 with_preset 档位，必须同步核对压缩端实际字典大小与 DICT_SIZE 的一致性。
const DICT_SIZE: u32 = lzma_rust::LZMA2Options::DICT_SIZE_DEFAULT;

/// 使用 LZMA2（raw 流，无 XZ 容器头）压缩数据。
///
/// # 参数
///
/// * `data` - 需要压缩的明文数据。
///
/// # 返回值
///
/// 返回压缩后的字节数组；若失败则返回 ErrorCode::FailToCompress。
pub fn compress(data: &[u8]) -> Result<Vec<u8>, ErrorCode> {
    let options = lzma_rust::LZMA2Options::with_preset(6);
    let mut out: Vec<u8> = Vec::new();
    {
        let counting = lzma_rust::CountingWriter::new(&mut out);
        let mut writer = lzma_rust::LZMA2Writer::new(counting, &options);
        writer
            .write_all(data)
            .map_err(|e| ErrorCode::FailToCompress { detail: e.to_string() })?;
        writer
            .finish()
            .map_err(|e| ErrorCode::FailToCompress { detail: e.to_string() })?;
    }
    Ok(out)
}

/// 解压 raw LZMA2 数据。
///
/// # 参数
///
/// * `data` - 需要解压的 LZMA2 数据。
///
/// # 返回值
///
/// 返回解压后的字节数组；若失败则返回 ErrorCode::FailToDecompress。
pub fn decompress(data: &[u8]) -> Result<Vec<u8>, ErrorCode> {
    let mut reader = lzma_rust::LZMA2Reader::new(data, DICT_SIZE, None);
    let mut out = Vec::new();
    reader
        .read_to_end(&mut out)
        .map_err(|e| ErrorCode::FailToDecompress { detail: e.to_string() })?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// lzma2 压缩与解压往返一致（随机数据）。
    #[test]
    fn test_lzma_roundtrip_random() {
        let data: Vec<u8> = (0..1024u32).map(|i| (i.wrapping_mul(251) % 256) as u8).collect();
        let compressed = compress(&data).unwrap();
        let decompressed = decompress(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    /// lzma2 压缩与解压往返一致（大块冗余数据，lzma2 对此类数据压缩率高）。
    #[test]
    fn test_lzma_roundtrip_redundant() {
        let data = vec![0xAAu8; 8192];
        let compressed = compress(&data).unwrap();
        assert!(compressed.len() < data.len());
        let decompressed = decompress(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    /// lzma2 解压失败路径：非法数据返回 FailToDecompress。
    #[test]
    fn test_lzma_decompress_invalid() {
        let bad_data = vec![0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
        let result = decompress(&bad_data);
        assert!(matches!(result, Err(ErrorCode::FailToDecompress { .. })));
    }
}
