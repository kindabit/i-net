use super::codec::flac_codec::WavParams;
use super::codec::{brotli_codec, flac_codec, lzma_codec, zstd_codec};
use super::param::CompressParam;
use crate::error_code::ErrorCode;

/// 压缩执行路由：按 param 指定的算法与参数压缩数据。
///
/// # 参数
///
/// * `param` - 压缩参数，决定使用的算法与相关配置。
/// * `data` - 需要压缩的原始数据。
///
/// # 返回值
///
/// 返回压缩后的字节数组；若失败则返回 ErrorCode::FailToCompress。
pub fn compress(param: &CompressParam, data: &[u8]) -> Result<Vec<u8>, ErrorCode> {
    match param {
        CompressParam::Brotli { quality, window } => brotli_codec::compress(data, *quality, *window),
        CompressParam::Zstd { level } => zstd_codec::compress(data, *level),
        CompressParam::Lzma => lzma_codec::compress(data),
        CompressParam::Flac { channels, bits_per_sample, sample_rate, data_len } => {
            flac_codec::compress(data, &WavParams {
                channels: *channels,
                bits_per_sample: *bits_per_sample,
                sample_rate: *sample_rate,
                data_len: *data_len,
            })
        }
    }
}

/// 解压执行路由：按 param 指定的算法与参数解压数据。
///
/// # 参数
///
/// * `param` - 压缩参数，决定使用的算法与相关配置。
/// * `data` - 需要解压的压缩数据。
///
/// # 返回值
///
/// 返回解压后的字节数组；若失败则返回 ErrorCode::FailToDecompress。
pub fn decompress(param: &CompressParam, data: &[u8]) -> Result<Vec<u8>, ErrorCode> {
    match param {
        CompressParam::Brotli { .. } => brotli_codec::decompress(data),
        CompressParam::Zstd { .. } => zstd_codec::decompress(data),
        CompressParam::Lzma => lzma_codec::decompress(data),
        CompressParam::Flac { channels, bits_per_sample, sample_rate, data_len } => {
            flac_codec::decompress(data, &WavParams {
                channels: *channels,
                bits_per_sample: *bits_per_sample,
                sample_rate: *sample_rate,
                data_len: *data_len,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Brotli 压缩与解压往返一致。
    #[test]
    fn test_execute_brotli_roundtrip() {
        let data: Vec<u8> = (0..512u32).map(|i| (i % 251) as u8).collect();
        let param = CompressParam::Brotli { quality: 5, window: 18 };
        let compressed = compress(&param, &data).unwrap();
        let decompressed = decompress(&param, &compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    /// Zstd 压缩与解压往返一致。
    #[test]
    fn test_execute_zstd_roundtrip() {
        let data: Vec<u8> = (0..512u32).map(|i| (i % 251) as u8).collect();
        let param = CompressParam::Zstd { level: 19 };
        let compressed = compress(&param, &data).unwrap();
        let decompressed = decompress(&param, &compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    /// Lzma 压缩与解压往返一致。
    #[test]
    fn test_execute_lzma_roundtrip() {
        let data: Vec<u8> = (0..512u32).map(|i| (i % 251) as u8).collect();
        let param = CompressParam::Lzma;
        let compressed = compress(&param, &data).unwrap();
        let decompressed = decompress(&param, &compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    /// Flac 压缩与解压往返一致（标准 WAV）。
    #[test]
    fn test_execute_flac_roundtrip() {
        // 构造标准 WAV：1ch 16bit 44100Hz，64 字节 PCM = 32 个样本（满足 flacenc block_size >= 32）
        let pcm: Vec<u8> = (0..32u16).flat_map(|i| i.to_le_bytes()).collect();
        let bps = 16u16;
        let channels = 1u16;
        let sample_rate = 44100u32;
        let block_align = (channels as u32) * (bps as u32 / 8);
        let byte_rate = sample_rate * (channels as u32) * (bps as u32 / 8);
        let data_len = pcm.len() as u32;

        let mut wav = Vec::with_capacity(44 + pcm.len());
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&(36 + data_len).to_le_bytes());
        wav.extend_from_slice(b"WAVE");
        wav.extend_from_slice(b"fmt ");
        wav.extend_from_slice(&16u32.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&channels.to_le_bytes());
        wav.extend_from_slice(&sample_rate.to_le_bytes());
        wav.extend_from_slice(&byte_rate.to_le_bytes());
        wav.extend_from_slice(&(block_align as u16).to_le_bytes());
        wav.extend_from_slice(&bps.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&data_len.to_le_bytes());
        wav.extend_from_slice(&pcm);

        let param = CompressParam::Flac {
            channels: 1,
            bits_per_sample: 16,
            sample_rate: 44100,
            data_len: 64,
        };
        let compressed = compress(&param, &wav).unwrap();
        let decompressed = decompress(&param, &compressed).unwrap();
        assert_eq!(decompressed, wav);
    }
}
