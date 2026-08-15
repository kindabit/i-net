use std::path::Path;

use rusqlite::Connection;

use crate::error_code::ErrorCode;
use crate::security::aes;
use crate::util::file_system_util;

/// 保存加密的 sqlite 文件。
///
/// 功能和 [`super::save_file`] 相同，不过多接受一个 key 用于加密。
///
/// # 参数
/// - `path`: sqlite 文件路径。
/// - `connection`: 数据库连接。
/// - `key`: 32 字节的加密密钥。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn save_file_encrypt(
    path: &Path,
    connection: &Connection,
    key: [u8; 32],
) -> Result<(), ErrorCode> {
    let data = connection.serialize(rusqlite::MAIN_DB).map_err(|e| {
        ErrorCode::FailToSerializeDatabase {
            detail: e.to_string(),
        }
    })?;
    let data = aes::encrypt(data.to_vec(), key)?;
    file_system_util::write(path, &data)
}
