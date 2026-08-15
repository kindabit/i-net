use crate::error_code::ErrorCode;

/// 标准 PCM WAV 参数。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WavParams {
    /// 声道数。
    pub channels: u32,
    /// 采样位深（8/16/24；flacenc 不支持 32-bit）。
    pub bits_per_sample: u32,
    /// 采样率。
    pub sample_rate: u32,
    /// 原始 PCM 数据字节数（WAV data chunk 长度）。
    pub data_len: u64,
}

/// 严格校验 data 是否为标准 PCM WAV（44 字节头、fmt+data 两 chunk、PCM 编码、位深 8/16/24）。
/// 满足时返回参数，否则返回 None（调用方应降级到兜底算法）。
/// 注意：flacenc 0.5.1 不支持 32-bit（上限 25 bps），32-bit WAV 应由调用方降级到兜底算法。
///
/// # 参数
///
/// * `data` - 待校验的 WAV 字节数据。
///
/// # 返回值
///
/// 校验通过返回 Some(WavParams)，否则返回 None。
pub fn validate_wav(data: &[u8]) -> Option<WavParams> {
    // 最小长度 44 字节（标准 PCM WAV 头）
    if data.len() < 44 {
        return None;
    }

    // 校验四个 magic：RIFF、WAVE、fmt、data
    if data[0..4] != *b"RIFF" {
        return None;
    }
    if data[8..12] != *b"WAVE" {
        return None;
    }
    if data[12..16] != *b"fmt " {
        return None;
    }
    if data[36..40] != *b"data" {
        return None;
    }

    // RIFF chunk 大小
    let riff_size = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
    if riff_size != (data.len() - 8) as u32 {
        return None;
    }

    // fmt chunk 长度必须为 16（PCM）
    let fmt_len = u32::from_le_bytes([data[16], data[17], data[18], data[19]]);
    if fmt_len != 16 {
        return None;
    }

    // audio format 必须为 1（PCM）
    let audio_format = u16::from_le_bytes([data[20], data[21]]);
    if audio_format != 1 {
        return None;
    }

    // 声道数 >= 1
    let channels = u16::from_le_bytes([data[22], data[23]]) as u32;
    if channels < 1 {
        return None;
    }

    // 采样率 > 0
    let sample_rate = u32::from_le_bytes([data[24], data[25], data[26], data[27]]);
    if sample_rate == 0 {
        return None;
    }

    // 位深必须在允许集合内（flacenc 0.5.1 不支持 32-bit，上限 25 bps）
    let bits_per_sample = u16::from_le_bytes([data[34], data[35]]) as u32;
    if !matches!(bits_per_sample, 8 | 16 | 24) {
        return None;
    }

    // byte_rate = sample_rate * channels * (bps/8)
    let byte_rate = u32::from_le_bytes([data[28], data[29], data[30], data[31]]);
    let expected_byte_rate = sample_rate * channels * (bits_per_sample / 8);
    if byte_rate != expected_byte_rate {
        return None;
    }

    // block_align = channels * (bps/8)
    let block_align = u16::from_le_bytes([data[32], data[33]]) as u32;
    let expected_block_align = channels * (bits_per_sample / 8);
    if block_align != expected_block_align {
        return None;
    }

    // data chunk 长度校验
    let data_len = u32::from_le_bytes([data[40], data[41], data[42], data[43]]) as u64;
    if data_len != (data.len() - 44) as u64 {
        return None;
    }
    if data_len == 0 {
        return None;
    }
    if data_len % block_align as u64 != 0 {
        return None;
    }

    Some(WavParams {
        channels,
        bits_per_sample,
        sample_rate,
        data_len,
    })
}

/// 将标准 PCM WAV 压缩为 FLAC 字节流。调用前必须先经 validate_wav 校验。
///
/// # 参数
///
/// * `data` - 标准 PCM WAV 字节数据（包含 44 字节头）。
/// * `params` - 由 validate_wav 返回的 WAV 参数。
///
/// # 返回值
///
/// 返回 FLAC 编码的字节数组；若失败则返回 ErrorCode::FailToCompress。
pub fn compress(data: &[u8], params: &WavParams) -> Result<Vec<u8>, ErrorCode> {
    use flacenc::component::BitRepr;
    use flacenc::error::Verify;

    // 提取 PCM 字节（跳过 44 字节头）
    let pcm = &data[44..];
    let bps = params.bits_per_sample;
    let bytes_per_sample = (bps / 8) as usize;

    // 将 PCM 字节转换为交错 i32 样本
    let mut samples: Vec<i32> = Vec::with_capacity(pcm.len() / bytes_per_sample);
    let mut offset = 0;
    while offset + bytes_per_sample <= pcm.len() {
        let sample = match bps {
            8 => {
                // WAV 8-bit 为 unsigned，FLAC 为 signed
                (pcm[offset] as i32) - 128
            }
            16 => {
                i16::from_le_bytes([pcm[offset], pcm[offset + 1]]) as i32
            }
            24 => {
                // 3 字节小端符号扩展为 i32
                let b0 = pcm[offset] as i32;
                let b1 = pcm[offset + 1] as i32;
                let b2 = pcm[offset + 2] as i32;
                let v = b0 | (b1 << 8) | (b2 << 16);
                if v & 0x800000 != 0 {
                    v | (0xFF000000u32 as i32)
                } else {
                    v
                }
            }
            _ => {
                return Err(ErrorCode::FailToCompress {
                    detail: format!("Unsupported bits_per_sample: {bps}"),
                })
            }
        };
        samples.push(sample);
        offset += bytes_per_sample;
    }

    let config = flacenc::config::Encoder::default()
        .into_verified()
        .map_err(|e| ErrorCode::FailToCompress {
            detail: format!("Invalid FLAC encoder config: {e:?}"),
        })?;
    let block_size = config.block_size;
    let source = flacenc::source::MemSource::from_samples(
        &samples,
        params.channels as usize,
        params.bits_per_sample as usize,
        params.sample_rate as usize,
    );
    let stream = flacenc::encode_with_fixed_block_size(&config, source, block_size).map_err(
        |e| ErrorCode::FailToCompress {
            detail: format!("FLAC encode failed: {e:?}"),
        },
    )?;
    let mut sink = flacenc::bitsink::ByteSink::new();
    stream.write(&mut sink).map_err(|e| ErrorCode::FailToCompress {
        detail: format!("FLAC write failed: {e:?}"),
    })?;
    Ok(sink.as_slice().to_vec())
}

/// 将 FLAC 字节流解压并重建为标准 PCM WAV（与压缩前 bit-exact 一致）。
///
/// # 参数
///
/// * `data` - FLAC 编码的字节数据。
/// * `params` - 用于重建 WAV 头的参数。
///
/// # 返回值
///
/// 返回重建的标准 PCM WAV 字节数组；若失败则返回 ErrorCode::FailToDecompress。
pub fn decompress(data: &[u8], params: &WavParams) -> Result<Vec<u8>, ErrorCode> {
    let cursor = std::io::Cursor::new(data);
    let mut reader = claxon::FlacReader::new(cursor).map_err(|e| {
        ErrorCode::FailToDecompress {
            detail: format!("FLAC decode failed: {e}"),
        }
    })?;
    let mut samples: Vec<i32> = Vec::new();
    for sample in reader.samples() {
        samples.push(sample.map_err(|e| ErrorCode::FailToDecompress {
            detail: format!("FLAC decode failed: {e}"),
        })?);
    }

    // 校验样本数量与 data_len 一致（样本已含声道交错）
    let bps = params.bits_per_sample;
    let expected_total_bytes = params.data_len;
    let actual_total_bytes = samples.len() as u64 * (bps / 8) as u64;
    if actual_total_bytes != expected_total_bytes {
        return Err(ErrorCode::FailToDecompress {
            detail: format!(
                "Sample count mismatch: got {actual_total_bytes} bytes expected {expected_total_bytes}"
            ),
        });
    }

    // i32 样本转回 PCM 字节
    let mut pcm: Vec<u8> = Vec::with_capacity(expected_total_bytes as usize);
    for s in &samples {
        match bps {
            8 => {
                // 8-bit: (s + 128) as u8
                pcm.push((*s + 128) as u8);
            }
            16 => {
                pcm.extend_from_slice(&(*s as i16).to_le_bytes());
            }
            24 => {
                pcm.push((*s & 0xFF) as u8);
                pcm.push(((*s >> 8) & 0xFF) as u8);
                pcm.push(((*s >> 16) & 0xFF) as u8);
            }
            _ => {
                return Err(ErrorCode::FailToDecompress {
                    detail: format!("Unsupported bits_per_sample: {bps}"),
                })
            }
        }
    }

    // 重建 44 字节标准 WAV 头
    let data_len = params.data_len;
    let channels = params.channels;
    let sample_rate = params.sample_rate;
    let block_align = channels * (bps / 8);
    let byte_rate = sample_rate * channels * (bps / 8);

    let mut header = Vec::with_capacity(44);
    // RIFF header
    header.extend_from_slice(b"RIFF");
    header.extend_from_slice(&(36 + data_len as u32).to_le_bytes());
    header.extend_from_slice(b"WAVE");
    // fmt chunk
    header.extend_from_slice(b"fmt ");
    header.extend_from_slice(&16u32.to_le_bytes()); // fmt chunk size
    header.extend_from_slice(&1u16.to_le_bytes()); // audio format = PCM
    header.extend_from_slice(&(channels as u16).to_le_bytes());
    header.extend_from_slice(&sample_rate.to_le_bytes());
    header.extend_from_slice(&byte_rate.to_le_bytes());
    header.extend_from_slice(&(block_align as u16).to_le_bytes());
    header.extend_from_slice(&(bps as u16).to_le_bytes());
    // data chunk
    header.extend_from_slice(b"data");
    header.extend_from_slice(&(data_len as u32).to_le_bytes());

    let mut result = header;
    result.extend_from_slice(&pcm);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造标准 PCM WAV 数据。
    fn make_wav(channels: u16, sample_rate: u32, bits_per_sample: u16, pcm: &[u8]) -> Vec<u8> {
        let bps = bits_per_sample as u32;
        let channels_u32 = channels as u32;
        let block_align = channels_u32 * (bps / 8);
        let byte_rate = sample_rate * channels_u32 * (bps / 8);
        let data_len = pcm.len() as u32;

        let mut wav = Vec::with_capacity(44 + pcm.len());
        // RIFF header
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&(36 + data_len).to_le_bytes());
        wav.extend_from_slice(b"WAVE");
        // fmt chunk
        wav.extend_from_slice(b"fmt ");
        wav.extend_from_slice(&16u32.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes()); // PCM
        wav.extend_from_slice(&channels.to_le_bytes());
        wav.extend_from_slice(&sample_rate.to_le_bytes());
        wav.extend_from_slice(&byte_rate.to_le_bytes());
        wav.extend_from_slice(&(block_align as u16).to_le_bytes());
        wav.extend_from_slice(&bits_per_sample.to_le_bytes());
        // data chunk
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&data_len.to_le_bytes());
        wav.extend_from_slice(pcm);
        wav
    }

    /// validate_wav 成功路径：16-bit 双声道标准 WAV。
    #[test]
    fn test_validate_wav_accepts_standard() {
        // 2ch 16bit: block_align = 4, data_len 必须是 4 的倍数
        // 32 u16 values = 64 bytes, for 2ch 16bit that's 16 frames (64 / 4)
        let pcm: Vec<u8> = (0..32u16).flat_map(|i| i.to_le_bytes()).collect();
        let wav = make_wav(2, 44100, 16, &pcm);
        let params = validate_wav(&wav);
        assert!(params.is_some());
        let p = params.unwrap();
        assert_eq!(p.channels, 2);
        assert_eq!(p.bits_per_sample, 16);
        assert_eq!(p.sample_rate, 44100);
        assert_eq!(p.data_len, 64);
    }

    /// validate_wav 拒绝：数据太短（10 字节）。
    #[test]
    fn test_validate_wav_reject_too_short() {
        let data = vec![0u8; 10];
        assert!(validate_wav(&data).is_none());
    }

    /// validate_wav 拒绝：非 RIFF 头。
    #[test]
    fn test_validate_wav_reject_not_riff() {
        let pcm = vec![0u8; 8];
        let mut wav = make_wav(2, 44100, 16, &pcm);
        wav[0] = 0xFF; // 破坏 RIFF 头
        assert!(validate_wav(&wav).is_none());
    }

    /// validate_wav 拒绝：fmt 长度 != 16。
    #[test]
    fn test_validate_wav_reject_fmt_len() {
        let pcm = vec![0u8; 8];
        let mut wav = make_wav(2, 44100, 16, &pcm);
        wav[16] = 18; // 修改 fmt chunk 长度
        assert!(validate_wav(&wav).is_none());
    }

    /// validate_wav 拒绝：audio format == 3（IEEE float）。
    #[test]
    fn test_validate_wav_reject_float_format() {
        let pcm = vec![0u8; 8];
        let mut wav = make_wav(2, 44100, 16, &pcm);
        wav[20] = 3; // audio format = 3 (IEEE float)
        assert!(validate_wav(&wav).is_none());
    }

    /// validate_wav 拒绝：bps == 12（非法位深）。
    #[test]
    fn test_validate_wav_reject_invalid_bps() {
        let pcm = vec![0u8; 4];
        let mut wav = make_wav(1, 44100, 16, &pcm);
        // 修改 bits_per_sample 为 12，同时调整 byte_rate 和 block_align
        wav[34] = 12;
        wav[35] = 0;
        // byte_rate = 44100 * 1 * (12/8) = 44100 * 1 * 1 = 44100 (integer division)
        let byte_rate: u32 = 44100 * 1 * (12 / 8);
        wav[28..32].copy_from_slice(&byte_rate.to_le_bytes());
        let block_align: u16 = (1 * (12 / 8)) as u16;
        wav[32..34].copy_from_slice(&block_align.to_le_bytes());
        assert!(validate_wav(&wav).is_none());
    }

    /// validate_wav 拒绝：bps == 32（flacenc 不支持 32-bit，上限 25 bps，应降级兜底）。
    #[test]
    fn test_validate_wav_reject_32bit() {
        // 构造标准 WAV 但 bps=32，byte_rate 和 block_align 按 32 位深正确填写
        let pcm = vec![0u8; 64];
        let mut wav = make_wav(1, 44100, 16, &pcm);
        // 将 bits_per_sample 改为 32
        wav[34] = 32;
        wav[35] = 0;
        // 按 bps=32 重新计算 byte_rate 和 block_align
        let byte_rate: u32 = 44100 * 1 * (32 / 8);
        wav[28..32].copy_from_slice(&byte_rate.to_le_bytes());
        let block_align: u16 = (1 * (32 / 8)) as u16;
        wav[32..34].copy_from_slice(&block_align.to_le_bytes());
        assert!(validate_wav(&wav).is_none());
    }

    /// validate_wav 拒绝：data_len 与文件长度不符。
    #[test]
    fn test_validate_wav_reject_data_len_mismatch() {
        let pcm = vec![0u8; 8];
        let mut wav = make_wav(2, 44100, 16, &pcm);
        wav[40] = 0xFF; // 修改 data_len
        assert!(validate_wav(&wav).is_none());
    }

    /// validate_wav 拒绝：data_len == 0。
    #[test]
    fn test_validate_wav_reject_zero_data() {
        // 构造一个 data_len = 0 的 WAV
        let wav = make_wav(2, 44100, 16, &[]);
        assert!(validate_wav(&wav).is_none());
    }

    /// FLAC 往返 bit-exact：16-bit 双声道。
    #[test]
    fn test_flac_roundtrip_16bit_stereo() {
        // 2ch 16bit: 每声道至少 32 帧以满足 flacenc block_size >= 32 要求
        // 64 u16 values = 128 bytes = 32 frames * 2 channels * 2 bytes
        let pcm: Vec<u8> = (0..64u16).flat_map(|i| i.to_le_bytes()).collect();
        let wav = make_wav(2, 44100, 16, &pcm);
        let params = validate_wav(&wav).unwrap();
        let flac_data = compress(&wav, &params).unwrap();
        let restored = decompress(&flac_data, &params).unwrap();
        assert_eq!(restored, wav);
    }

    /// FLAC 往返 bit-exact：8-bit 单声道。
    #[test]
    fn test_flac_roundtrip_8bit_mono() {
        // 8-bit unsigned: 范围 0..=255，至少 32 个样本以满足 flacenc block_size >= 32
        let pcm: Vec<u8> = (0..64u16).map(|i| (i % 256) as u8).collect(); // 64 samples
        let wav = make_wav(1, 22050, 8, &pcm);
        let params = validate_wav(&wav).unwrap();
        let flac_data = compress(&wav, &params).unwrap();
        let restored = decompress(&flac_data, &params).unwrap();
        assert_eq!(restored, wav);
    }

    /// FLAC 往返 bit-exact：24-bit 单声道。
    #[test]
    fn test_flac_roundtrip_24bit_mono() {
        // 24-bit: 每样本 3 字节，data_len 必须是 3 的倍数，至少 32 个样本
        let pcm: Vec<u8> = (0..96u16).map(|i| (i % 251) as u8).collect(); // 96 bytes = 32 samples
        let wav = make_wav(1, 48000, 24, &pcm);
        let params = validate_wav(&wav).unwrap();
        let flac_data = compress(&wav, &params).unwrap();
        let restored = decompress(&flac_data, &params).unwrap();
        assert_eq!(restored, wav);
    }

    /// FLAC 往返 bit-exact：24-bit 双声道（flacenc 最大支持 25 bps，故用 24-bit 代替 32-bit）。
    #[test]
    fn test_flac_roundtrip_24bit_stereo() {
        // 24-bit: 每样本 3 字节，双声道 => block_align = 6
        // 至少 32 帧每声道：32 * 2 * 3 = 192 bytes
        let pcm: Vec<u8> = (0..192u16).map(|i| (i % 251) as u8).collect(); // 192 bytes = 32 frames * 2 ch * 3 bytes
        let wav = make_wav(2, 96000, 24, &pcm);
        let params = validate_wav(&wav).unwrap();
        let flac_data = compress(&wav, &params).unwrap();
        let restored = decompress(&flac_data, &params).unwrap();
        assert_eq!(restored, wav);
    }

    /// FLAC 解压失败路径：非法 FLAC 数据。
    #[test]
    fn test_flac_decompress_invalid() {
        let bad_data = vec![0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
        let params = WavParams {
            channels: 1,
            bits_per_sample: 16,
            sample_rate: 44100,
            data_len: 2,
        };
        let result = decompress(&bad_data, &params);
        assert!(matches!(result, Err(ErrorCode::FailToDecompress { .. })));
    }
}
