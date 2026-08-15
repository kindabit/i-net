use crate::business::user_database::dictionary::dao;
use crate::business::user_database::entity::Dictionary;
use crate::business::user_database::state;
use crate::error_code::ErrorCode;

/// 获取字典条目全量列表，按存储顺序返回。
///
/// # 返回值
/// 返回字典条目列表；若发生错误则返回对应的 `ErrorCode`。
pub fn list() -> Result<Vec<Dictionary>, ErrorCode> {
    let connection = state::lock_connection();
    dao::select_all(&connection)
}
