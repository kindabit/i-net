/// 文件类型识别引擎的封装层。当前实现基于 infer crate。
/// route 模块只依赖本模块的 detect 函数，更换识别引擎时只需修改本文件内部实现，对外契约不变。

/// 识别出的文件类型。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DetectedFileType {
    /// 类型的典型扩展名（如 "zip"、"png"）。
    pub extension: &'static str,
    /// 类型的 MIME（如 "application/zip"）。
    pub mime: &'static str,
}

/// 通过 magic bytes 探测数据类型；无法识别时返回 None。
pub fn detect(data: &[u8]) -> Option<DetectedFileType> {
    infer::get(data).map(|t| DetectedFileType {
        extension: t.extension(),
        mime: t.mime_type(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// detect 成功路径：zip magic bytes 识别为 zip。
    #[test]
    fn test_detect_zip() {
        let mut data = vec![0x50, 0x4B, 0x03, 0x04, 0x14, 0, 0, 0, 8, 0];
        data.extend(vec![0u8; 22]); // 补零到 30 字节以上
        let result = detect(&data);
        assert!(result.is_some());
        assert_eq!(result.unwrap().extension, "zip");
    }

    /// detect 成功路径：gzip magic bytes 识别为 gz。
    #[test]
    fn test_detect_gzip() {
        let data = vec![0x1F, 0x8B, 0x08, 0, 0, 0, 0, 0, 0, 0];
        let result = detect(&data);
        assert!(result.is_some());
        assert_eq!(result.unwrap().extension, "gz");
    }

    /// detect 失败路径：全零数据无法识别。
    #[test]
    fn test_detect_all_zeros() {
        let data = vec![0u8; 256];
        assert!(detect(&data).is_none());
    }
}
