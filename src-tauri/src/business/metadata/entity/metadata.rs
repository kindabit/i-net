use serde::{Deserialize, Serialize};

/// 用户数据库元数据实体类，每个字段都不能为 null。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    /// 数据库 id（uuid），主键。
    pub id: String,
    /// 数据库名称，唯一键。
    pub name: String,
    /// 是否归档。
    pub archived: bool,
    /// 创建时间，毫秒时间戳。
    pub create_time: i64,
    /// 修改时间，毫秒时间戳。
    pub modify_time: i64,
    /// 最后打开时间，毫秒时间戳。
    pub last_open_time: i64,
}
