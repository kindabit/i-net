use rusqlite::Connection;

use super::super::dao;
use crate::error_code::ErrorCode;

/// 从 variable 表中按名称查询变量的值。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `name`: 变量名称。
///
/// # 返回值
/// 返回变量的值，不存在时返回 `None`；若发生错误则返回对应的 `ErrorCode`。
pub fn get(connection: &Connection, name: &str) -> Result<Option<String>, ErrorCode> {
    dao::select_by_name(connection, name)
}
