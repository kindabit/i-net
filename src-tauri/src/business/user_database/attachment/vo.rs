use serde::{Deserialize, Serialize};

/// 附件值对象，附件在前后端之间传输的载体。
/// missing_file 由 service 层在 list 时经文件存在性检查填充，dao 层无此概念。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AttachmentVO {
    /// 附件 id（uuid）。
    pub id: String,
    /// 附件的原始文件名。
    pub file_name: String,
    /// 附件明文内容的大小，单位为字节。
    pub size: i64,
    /// 附件的导入时间，毫秒时间戳。
    pub create_time: i64,
    /// 附件文件是否丢失（元数据存在但附件目录中没有对应文件）。
    pub missing_file: bool,
}
