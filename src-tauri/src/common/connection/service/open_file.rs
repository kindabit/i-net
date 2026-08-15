use std::path::Path;

use rusqlite::Connection;

use crate::common::data_version;
use crate::error_code::ErrorCode;
use crate::util::file_system_util;

/// 读取未加密的 sqlite 文件。
///
/// 路径存在时将文件读入内存并通过反序列化建立 connection，
/// 路径不存在时直接在内存中建立 connection，
/// 返回 connection 前由 data_version 处理一遍。
///
/// # 参数
/// - `path`: sqlite 文件路径。
///
/// # 返回值
/// 返回建立好的 connection；若发生错误则返回对应的 `ErrorCode`。
pub fn open_file(path: &Path) -> Result<Connection, ErrorCode> {
    let mut connection =
        Connection::open_in_memory().map_err(|e| ErrorCode::FailToOpenConnection {
            detail: e.to_string(),
        })?;
    if file_system_util::try_exists(path)? {
        let data = file_system_util::read(path)?;
        connection
            .deserialize_read_exact(rusqlite::MAIN_DB, data.as_slice(), data.len(), false)
            .map_err(|e| ErrorCode::FailToDeserializeDatabase {
                detail: e.to_string(),
            })?;
    }
    connection
        .execute_batch("PRAGMA foreign_keys = ON;")
        .map_err(|e| ErrorCode::FailToOpenConnection {
            detail: e.to_string(),
        })?;
    data_version::service::process(&connection)?;
    Ok(connection)
}
