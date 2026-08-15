use serde::{Deserialize, Serialize};

use crate::error_code::ErrorCode;

/// 压缩参数：记录压缩算法名称与相关参数。序列化为 JSON 字符串存入 attachment 表的
/// compress_param 列，由压缩模块生产、由压缩模块消费，其它模块一律视为不透明字符串。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "algorithm", rename_all = "lowercase")]
pub enum CompressParam {
    /// brotli 压缩（文本类文件），quality 为质量档位（0-11），window 为窗口大小参数 lgwin。
    Brotli { quality: u32, window: u32 },
    /// zstd 压缩（通用兜底），level 为压缩等级。
    Zstd { level: i32 },
    /// lzma 压缩（xz 容器格式，未压缩位图与可执行文件），无参数。
    Lzma,
    /// FLAC 压缩（标准 PCM WAV 文件），携带重建 WAV 头所需的参数。
    Flac {
        /// 声道数。
        channels: u32,
        /// 采样位深（8/16/24）。
        bits_per_sample: u32,
        /// 采样率。
        sample_rate: u32,
        /// 原始 PCM 数据字节数（WAV data chunk 长度）。
        data_len: u64,
    },
}

impl CompressParam {
    /// 序列化为 JSON 字符串。
    pub fn serialize(&self) -> String {
        serde_json::to_string(self).expect("Failed to serialize CompressParam")
    }

    /// 从 JSON 字符串反序列化；失败时返回 ErrorCode::FailToDecompress。
    pub fn deserialize(s: &str) -> Result<Self, ErrorCode> {
        serde_json::from_str(s).map_err(|e| ErrorCode::FailToDecompress {
            detail: format!("Invalid compress param: {e}"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 覆盖 CompressParam 四个变体的序列化与反序列化往返一致性。
    #[test]
    fn test_compress_param_roundtrip() {
        let cases = vec![
            CompressParam::Brotli { quality: 11, window: 22 },
            CompressParam::Zstd { level: 19 },
            CompressParam::Lzma,
            CompressParam::Flac {
                channels: 2,
                bits_per_sample: 16,
                sample_rate: 44100,
                data_len: 1024,
            },
        ];

        for param in cases {
            let json = param.serialize();
            let deserialized = CompressParam::deserialize(&json).unwrap();
            assert_eq!(deserialized, param, "roundtrip failed for {:?}", param);
        }

        // 验证 JSON 格式符合预期
        let brotli_json = CompressParam::Brotli { quality: 11, window: 22 }.serialize();
        assert!(brotli_json.contains("\"algorithm\":\"brotli\""));
        assert!(brotli_json.contains("\"quality\":11"));
        assert!(brotli_json.contains("\"window\":22"));

        let lzma_json = CompressParam::Lzma.serialize();
        assert_eq!(lzma_json, "{\"algorithm\":\"lzma\"}");
    }

    /// deserialize 失败路径：非法 JSON 返回 FailToDecompress。
    #[test]
    fn test_compress_param_deserialize_invalid() {
        assert!(matches!(
            CompressParam::deserialize("not-json"),
            Err(ErrorCode::FailToDecompress { .. })
        ));
        assert!(matches!(
            CompressParam::deserialize(""),
            Err(ErrorCode::FailToDecompress { .. })
        ));
        assert!(matches!(
            CompressParam::deserialize("{\"algorithm\":\"unknown\"}"),
            Err(ErrorCode::FailToDecompress { .. })
        ));
    }
}
