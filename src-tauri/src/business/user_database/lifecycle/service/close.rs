use crate::business::user_database::state;
use crate::error_code::ErrorCode;

/// 关闭当前打开的用户数据库：清空 state 中的连接、元信息和密钥。
///
/// # 返回值
/// 成功时返回 `Ok(())`；用户数据库未打开时返回 `ErrorCode::UserDatabaseNotOpen`，
/// 发生其他错误时返回对应的 `ErrorCode`。
pub fn close() -> Result<(), ErrorCode> {
    if !state::is_open() {
        return Err(ErrorCode::UserDatabaseNotOpen);
    }
    state::clear();
    Ok(())
}
