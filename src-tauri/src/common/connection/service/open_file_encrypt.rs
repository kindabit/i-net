use std::path::Path;

use rusqlite::Connection;

use crate::common::data_version;
use crate::error_code::ErrorCode;
use crate::security::aes;
use crate::util::file_system_util;

/// 读取加密的 sqlite 文件。
///
/// 功能和 [`super::open_file`] 相同，不过多接受一个 key 用于解密，
/// 数据解密后在内存中保持明文状态。
///
/// # 参数
/// - `path`: sqlite 文件路径。
/// - `key`: 32 字节的解密密钥。
///
/// # 返回值
/// 返回建立好的 connection；若发生错误则返回对应的 `ErrorCode`。
pub fn open_file_encrypt(path: &Path, key: [u8; 32]) -> Result<Connection, ErrorCode> {
    let mut connection =
        Connection::open_in_memory().map_err(|e| ErrorCode::FailToOpenConnection {
            detail: e.to_string(),
        })?;
    if file_system_util::try_exists(path)? {
        let data = aes::decrypt(file_system_util::read(path)?, key)?;
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
