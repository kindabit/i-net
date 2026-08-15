use super::execute;
use super::param::CompressParam;
use super::route;
use crate::error_code::ErrorCode;

/// 压缩 guard 的输出。
#[derive(Debug, Clone, PartialEq)]
pub struct GuardOutput {
    /// 压缩或未压缩的二进制数据（调用方将其作为加密输入）。
    pub data: Vec<u8>,
    /// 数据是否经过压缩。
    pub compressed: bool,
    /// 压缩参数串（写入 attachment.compress_param）；compressed 为 false 时为空字符串。
    pub compress_param: String,
}

/// 压缩 guard：按文件名与明文内容分拣并（可能）压缩。
///
/// # 参数
///
/// * `file_name` - 文件名（可能含扩展名），用于路由判定。
/// * `plaintext` - 文件明文数据。
///
/// # 返回值
///
/// 返回 GuardOutput 包含处理后的数据、是否压缩标志及压缩参数串；
/// 若压缩失败则返回 ErrorCode::FailToCompress。
pub fn compress(file_name: &str, plaintext: Vec<u8>) -> Result<GuardOutput, ErrorCode> {
    let (compressed, param) = route::classify(file_name, &plaintext);
    if !compressed {
        return Ok(GuardOutput {
            data: plaintext,
            compressed: false,
            compress_param: String::new(),
        });
    }
    let param = param.expect("classify must return Some(param) when compressed is true");
    let data = execute::compress(&param, &plaintext)?;
    Ok(GuardOutput {
        data,
        compressed: true,
        compress_param: param.serialize(),
    })
}

/// 解压 guard：按参数串解压数据还原明文。
///
/// # 参数
///
/// * `compress_param` - 压缩参数串（来自 attachment.compress_param）。
/// * `data` - 压缩数据。
///
/// # 返回值
///
/// 返回解压后的明文数据；若参数串无效则返回 ErrorCode::FailToDecompress，
/// 若解压失败则返回 ErrorCode::FailToDecompress。
pub fn decompress(compress_param: &str, data: Vec<u8>) -> Result<Vec<u8>, ErrorCode> {
    let param = CompressParam::deserialize(compress_param)?;
    execute::decompress(&param, &data)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// compress("x.txt", 文本) → compressed=true、param 非空 → 往返一致。
    #[test]
    fn test_guard_compress_txt_roundtrip() {
        let plaintext = b"This is some text data for testing the compress guard. ".repeat(20);
        let output = compress("x.txt", plaintext.clone()).unwrap();
        assert!(output.compressed);
        assert!(!output.compress_param.is_empty());

        let decompressed = decompress(&output.compress_param, output.data).unwrap();
        assert_eq!(decompressed, plaintext);
    }

    /// compress("x.zip", zip magic 数据) → compressed=false、data==原文、param==""。
    #[test]
    fn test_guard_compress_zip_bypass() {
        let mut data = vec![0x50, 0x4B, 0x03, 0x04, 0x14, 0, 0, 0, 8, 0];
        data.extend(vec![0u8; 22]);
        let output = compress("x.zip", data.clone()).unwrap();
        assert!(!output.compressed);
        assert_eq!(output.data, data);
        assert_eq!(output.compress_param, "");
    }

    /// compress("x.bin", 空 vec) → compressed=false。
    #[test]
    fn test_guard_compress_empty() {
        let output = compress("x.bin", vec![]).unwrap();
        assert!(!output.compressed);
        assert_eq!(output.data, Vec::<u8>::new());
        assert_eq!(output.compress_param, "");
    }

    /// compress("x.wav", 标准 WAV) → compressed=true 且往返 bit-exact。
    #[test]
    fn test_guard_compress_wav_roundtrip() {
        // 构造标准 WAV（至少 32 个样本以满足 flacenc block_size >= 32）
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
        wav.extend_from_slice(&block_align.to_le_bytes());
        wav.extend_from_slice(&bps.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&data_len.to_le_bytes());
        wav.extend_from_slice(&pcm);

        let output = compress("x.wav", wav.clone()).unwrap();
        assert!(output.compressed);
        let decompressed = decompress(&output.compress_param, output.data).unwrap();
        assert_eq!(decompressed, wav);
    }

    /// decompress("", vec![...]) → Err(FailToDecompress)。
    #[test]
    fn test_guard_decompress_empty_param() {
        let result = decompress("", vec![1, 2, 3]);
        assert!(matches!(result, Err(ErrorCode::FailToDecompress { .. })));
    }

    /// decompress("garbage", vec![...]) → Err(FailToDecompress)。
    #[test]
    fn test_guard_decompress_garbage_param() {
        let result = decompress("garbage", vec![1, 2, 3]);
        assert!(matches!(result, Err(ErrorCode::FailToDecompress { .. })));
    }
}
