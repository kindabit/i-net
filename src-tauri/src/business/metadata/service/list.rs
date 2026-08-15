use crate::business::metadata::entity::Metadata;
use crate::business::metadata::{dao, state};
use crate::error_code::ErrorCode;

/// 按归档状态查询用户数据库，按最后打开时间从大到小排序，
/// 最后打开时间相同的按 name 排序。
///
/// # 参数
/// - `archived`: 归档状态，`false` 查询未归档的数据库，`true` 查询已归档的数据库。
///
/// # 返回值
/// 返回查询到的元数据列表；若发生错误则返回对应的 `ErrorCode`。
pub fn list(archived: bool) -> Result<Vec<Metadata>, ErrorCode> {
    let connection = state::lock_connection();
    dao::select_by_archived(&connection, archived)
}
