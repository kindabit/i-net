use rusqlite::Connection;

use super::super::dao;
use crate::error_code::ErrorCode;

/// 向 variable 表插入或更新变量。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `name`: 变量名称。
/// - `value`: 变量值。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn set(connection: &Connection, name: &str, value: &str) -> Result<(), ErrorCode> {
    dao::upsert(connection, name, value)
}
