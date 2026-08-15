use serde::{Deserialize, Serialize};

/// 数据版本实体类，对应语义化版本的三个数字。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataVersion {
    /// 主版本号。
    pub major: i64,
    /// 次版本号。
    pub minor: i64,
    /// 修订号。
    pub patch: i64,
}
