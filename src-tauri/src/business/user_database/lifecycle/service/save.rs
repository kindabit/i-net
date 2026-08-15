use crate::business::user_database::state;
use crate::common::connection;
use crate::error_code::ErrorCode;

/// 保存当前打开的用户数据库：将内存中的数据库加密后写入数据库文件。
///
/// # 返回值
/// 成功时返回 `Ok(())`；用户数据库未打开时返回 `ErrorCode::UserDatabaseNotOpen`，
/// 发生其他错误时返回对应的 `ErrorCode`。
pub fn save() -> Result<(), ErrorCode> {
    if !state::is_open() {
        return Err(ErrorCode::UserDatabaseNotOpen);
    }
    let path = crate::state::path();
    let database_file = path.user_database_file(&state::metadata().id);
    connection::service::save_file_encrypt(&database_file, &state::lock_connection(), state::key())
}
