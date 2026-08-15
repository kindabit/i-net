use super::codec::flac_codec;
use super::engine;
use super::param::CompressParam;

/// 已知已压缩格式（跳过压缩）。同时用于 magic 探测结果与文件扩展名匹配。
const COMPRESSED_FORMATS: &[&str] = &[
    // 压缩/归档容器（docx 等 Office Open XML 与 odt 等 ODF 本质是 zip）
    "zip", "docx", "xlsx", "pptx", "epub", "jar", "apk", "odt", "ods", "odp",
    "gz", "tgz", "bz2", "xz", "zst", "lz4", "7z", "rar",
    // 已压缩文档
    "pdf",
    // 已压缩图片
    "png", "jpg", "jpeg", "gif", "webp", "avif", "heic", "heif",
    // 已压缩音视频
    "mp3", "m4a", "aac", "ogg", "opus", "flac", "mp4", "mkv", "webm", "mov", "avi", "wmv", "flv",
    // 已压缩字体
    "woff", "woff2",
];

/// 文本类扩展名（brotli 内置文本字典，压缩率最佳）。
const TEXT_EXTENSIONS: &[&str] = &[
    "txt", "md", "markdown", "json", "xml", "csv", "tsv", "log", "yaml", "yml", "toml", "ini", "cfg", "conf",
    "html", "htm", "css", "js", "mjs", "cjs", "ts", "jsx", "tsx", "vue", "scss", "less",
    "sql", "rs", "py", "java", "c", "h", "cpp", "hpp", "cc", "cs", "go", "rb", "php", "swift", "kt",
    "sh", "bash", "bat", "ps1", "env",
];

/// 未压缩位图格式（lzma 对大块冗余位图数据压缩率高）。
const BITMAP_FORMATS: &[&str] = &["bmp", "tif", "tiff", "tga", "ppm", "pgm", "pbm", "xbm", "xpm"];

/// 未压缩但不做专门处理的音频扩展名（走 zstd 兜底）。
const AUDIO_EXTENSIONS: &[&str] = &["aif", "aiff", "au"];

/// 可执行文件的 magic 探测值（dll 与 exe 同为 MZ 头，infer 统一返回 "exe"；so 为 ELF）。
const EXECUTABLE_MAGIC_FORMATS: &[&str] = &["exe", "elf", "macho"];

/// 可执行文件扩展名（magic 未命中时兜底）。
const EXECUTABLE_EXTENSIONS: &[&str] = &["exe", "dll", "so", "dylib"];

/// 分拣路由：根据文件名与明文内容判定是否压缩以及压缩参数。
/// 返回 (compressed, param)；compressed 为 true 时 param 必为 Some。
///
/// # 参数
///
/// * `file_name` - 文件名（可能含扩展名）。
/// * `data` - 文件明文数据。
///
/// # 返回值
///
/// 返回 (是否压缩, 压缩参数) 元组；compressed 为 true 时 param 必为 Some。
pub fn classify(file_name: &str, data: &[u8]) -> (bool, Option<CompressParam>) {
    // 1. 空数据直通
    if data.is_empty() {
        return (false, None);
    }

    // 2. magic bytes 优先判定
    if let Some(detected) = engine::detect(data) {
        let ext = detected.extension;

        // 已压缩格式直通
        if COMPRESSED_FORMATS.contains(&ext) {
            return (false, None);
        }

        // WAV 校验分支
        if ext == "wav" {
            return match flac_codec::validate_wav(data) {
                Some(params) => (
                    true,
                    Some(CompressParam::Flac {
                        channels: params.channels,
                        bits_per_sample: params.bits_per_sample,
                        sample_rate: params.sample_rate,
                        data_len: params.data_len,
                    }),
                ),
                None => (true, Some(CompressParam::Zstd { level: 19 })),
            };
        }

        // 位图与可执行文件走 lzma
        if BITMAP_FORMATS.contains(&ext) || EXECUTABLE_MAGIC_FORMATS.contains(&ext) {
            return (true, Some(CompressParam::Lzma));
        }

        // 其它命中（如 ttf 等）走 zstd 兜底
        return (true, Some(CompressParam::Zstd { level: 19 }));
    }

    // 3. engine 未识别，取扩展名兜底
    let ext = std::path::Path::new(file_name)
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase());

    match ext.as_deref() {
        None => (true, Some(CompressParam::Zstd { level: 19 })),
        Some(e) if COMPRESSED_FORMATS.contains(&e) => (false, None),
        Some(e) if TEXT_EXTENSIONS.contains(&e) => {
            (true, Some(CompressParam::Brotli { quality: 11, window: 22 }))
        }
        Some("wav") => match flac_codec::validate_wav(data) {
            Some(params) => (
                true,
                Some(CompressParam::Flac {
                    channels: params.channels,
                    bits_per_sample: params.bits_per_sample,
                    sample_rate: params.sample_rate,
                    data_len: params.data_len,
                }),
            ),
            None => (true, Some(CompressParam::Zstd { level: 19 })),
        },
        Some(e) if AUDIO_EXTENSIONS.contains(&e) => {
            (true, Some(CompressParam::Zstd { level: 19 }))
        }
        Some(e) if BITMAP_FORMATS.contains(&e) || EXECUTABLE_EXTENSIONS.contains(&e) => {
            (true, Some(CompressParam::Lzma))
        }
        Some(_) => (true, Some(CompressParam::Zstd { level: 19 })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造不会被 infer 误判的普通数据（首字节 0x00 不命中任何 magic）。
    fn plain_data() -> Vec<u8> {
        (0..1024u32).map(|i| (i % 251) as u8).collect()
    }

    /// 空数据 → (false, None)。
    #[test]
    fn test_classify_empty_data() {
        assert_eq!(classify("x.txt", &[]), (false, None));
    }

    /// zip magic + "x.zip" → (false, None)。
    #[test]
    fn test_classify_zip_magic() {
        let mut data = vec![0x50, 0x4B, 0x03, 0x04, 0x14, 0, 0, 0, 8, 0];
        data.extend(vec![0u8; 22]);
        assert_eq!(classify("x.zip", &data), (false, None));
    }

    /// zip magic + "x.txt" → (false, None)（magic 优先）。
    #[test]
    fn test_classify_zip_magic_txt_name() {
        let mut data = vec![0x50, 0x4B, 0x03, 0x04, 0x14, 0, 0, 0, 8, 0];
        data.extend(vec![0u8; 22]);
        assert_eq!(classify("x.txt", &data), (false, None));
    }

    /// 普通数据 + "x.zip" → (false, None)（扩展名兜底直通）。
    #[test]
    fn test_classify_plain_data_zip_ext() {
        assert_eq!(classify("x.zip", &plain_data()), (false, None));
    }

    /// 普通数据 + "x.flac" → (false, None)。
    #[test]
    fn test_classify_plain_data_flac_ext() {
        assert_eq!(classify("x.flac", &plain_data()), (false, None));
    }

    /// 普通数据 + "x.txt" → Brotli。
    #[test]
    fn test_classify_plain_data_txt_ext() {
        let (compressed, param) = classify("x.txt", &plain_data());
        assert!(compressed);
        assert_eq!(param, Some(CompressParam::Brotli { quality: 11, window: 22 }));
    }

    /// 普通数据 + "X.TXT"（大写扩展名）→ Brotli。
    #[test]
    fn test_classify_uppercase_txt_ext() {
        let (compressed, param) = classify("X.TXT", &plain_data());
        assert!(compressed);
        assert_eq!(param, Some(CompressParam::Brotli { quality: 11, window: 22 }));
    }

    /// bmp magic + "x.bin" → Lzma。
    #[test]
    fn test_classify_bmp_magic() {
        let mut data = vec![0x42, 0x4D, 0, 0, 0, 0, 0, 0, 0, 0];
        data.extend(vec![0u8; 20]); // 补到 30 字节
        let (compressed, param) = classify("x.bin", &data);
        assert!(compressed);
        assert_eq!(param, Some(CompressParam::Lzma));
    }

    /// MZ magic + "x.exe" → Lzma。
    #[test]
    fn test_classify_mz_magic() {
        let mut data = vec![0x4D, 0x5A, 0x90, 0, 3, 0, 0, 0, 4, 0];
        data.extend(vec![0u8; 22]); // 补到 30 字节
        let (compressed, param) = classify("x.exe", &data);
        assert!(compressed);
        assert_eq!(param, Some(CompressParam::Lzma));
    }

    /// 标准 PCM WAV + "x.wav" → Flac。
    #[test]
    fn test_classify_standard_wav() {
        // 构造标准 WAV：1ch 16bit 44100Hz，32 字节 PCM = 16 个样本（满足 FLAC 最小 block size）
        let pcm: Vec<u8> = (0..16u16).flat_map(|i| i.to_le_bytes()).collect();
        let wav = make_wav(1, 44100, 16, &pcm);
        let (compressed, param) = classify("x.wav", &wav);
        assert!(compressed);
        assert!(matches!(param, Some(CompressParam::Flac { channels: 1, bits_per_sample: 16, sample_rate: 44100, data_len: 32 })));
    }

    /// 32-bit 标准 PCM WAV + "x.wav" → Zstd 降级兜底（flacenc 不支持 32-bit）。
    #[test]
    fn test_classify_32bit_wav_falls_back_to_zstd() {
        // 构造 bps=32 的标准 WAV，validate_wav 应拒绝，route 走扩展名分支降级到 Zstd
        let pcm = vec![0u8; 64];
        let mut wav = make_wav(1, 44100, 16, &pcm);
        // 将 bits_per_sample 改为 32，同时按 32 位深修正 byte_rate 和 block_align
        wav[34] = 32;
        wav[35] = 0;
        let byte_rate: u32 = 44100 * 1 * (32 / 8);
        wav[28..32].copy_from_slice(&byte_rate.to_le_bytes());
        let block_align: u16 = (1 * (32 / 8)) as u16;
        wav[32..34].copy_from_slice(&block_align.to_le_bytes());
        let (compressed, param) = classify("x.wav", &wav);
        assert!(compressed);
        assert_eq!(param, Some(CompressParam::Zstd { level: 19 }));
    }

    /// fmt_len != 16 的 WAV + "x.wav" → Zstd 降级兜底。
    #[test]
    fn test_classify_non_standard_wav() {
        // 构造一个 WAV 头但修改 fmt chunk 长度
        let pcm = vec![0u8; 8];
        let mut wav = make_wav(1, 44100, 16, &pcm);
        // 破坏 fmt chunk 长度使其不等于 16
        wav[16] = 18;
        let (compressed, param) = classify("x.wav", &wav);
        assert!(compressed);
        assert_eq!(param, Some(CompressParam::Zstd { level: 19 }));
    }

    /// 普通数据 + "x.bmp" → Lzma。
    #[test]
    fn test_classify_plain_data_bmp_ext() {
        let (compressed, param) = classify("x.bmp", &plain_data());
        assert!(compressed);
        assert_eq!(param, Some(CompressParam::Lzma));
    }

    /// 普通数据 + "x.dll" → Lzma。
    #[test]
    fn test_classify_plain_data_dll_ext() {
        let (compressed, param) = classify("x.dll", &plain_data());
        assert!(compressed);
        assert_eq!(param, Some(CompressParam::Lzma));
    }

    /// 普通数据 + "x.aiff" → Zstd 兜底。
    #[test]
    fn test_classify_plain_data_aiff_ext() {
        let (compressed, param) = classify("x.aiff", &plain_data());
        assert!(compressed);
        assert_eq!(param, Some(CompressParam::Zstd { level: 19 }));
    }

    /// 普通数据 + "x.dat" → Zstd 兜底。
    #[test]
    fn test_classify_plain_data_dat_ext() {
        let (compressed, param) = classify("x.dat", &plain_data());
        assert!(compressed);
        assert_eq!(param, Some(CompressParam::Zstd { level: 19 }));
    }

    /// 普通数据 + "noext" → Zstd 兜底（无扩展名）。
    #[test]
    fn test_classify_no_extension() {
        let (compressed, param) = classify("noext", &plain_data());
        assert!(compressed);
        assert_eq!(param, Some(CompressParam::Zstd { level: 19 }));
    }

    /// 辅助函数：构造标准 PCM WAV。
    fn make_wav(channels: u16, sample_rate: u32, bits_per_sample: u16, pcm: &[u8]) -> Vec<u8> {
        let bps = bits_per_sample as u32;
        let channels_u32 = channels as u32;
        let block_align = channels_u32 * (bps / 8);
        let byte_rate = sample_rate * channels_u32 * (bps / 8);
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
        wav.extend_from_slice(&bits_per_sample.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&data_len.to_le_bytes());
        wav.extend_from_slice(pcm);
        wav
    }
}
