use std::path::Path;

use rusqlite::Connection;

use crate::error_code::ErrorCode;
use crate::util::file_system_util;

/// 保存未加密的 sqlite 文件：将 connection 序列化之后写入目标文件。
///
/// # 参数
/// - `path`: sqlite 文件路径。
/// - `connection`: 数据库连接。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn save_file(path: &Path, connection: &Connection) -> Result<(), ErrorCode> {
    let data = connection.serialize(rusqlite::MAIN_DB).map_err(|e| {
        ErrorCode::FailToSerializeDatabase {
            detail: e.to_string(),
        }
    })?;
    file_system_util::write(path, &data)
}
