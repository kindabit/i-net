use serde::Serialize;

use crate::business::user_database::entity::Action;

/// 日志列表的响应结构：行为数据已解密并重组反序列化，用于向前端返回日志内容。
#[derive(Debug, Clone, Serialize)]
pub struct LogListResponse {
    /// 日志 id（uuid）。
    pub id: String,
    /// 被操作对象的 id。
    pub object_id: String,
    /// 行为（含数据载荷）。
    pub action: Action,
    /// 时间，毫秒时间戳。
    pub time: i64,
}
