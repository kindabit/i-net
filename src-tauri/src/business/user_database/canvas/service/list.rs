use crate::business::user_database::canvas::dao;
use crate::business::user_database::entity::Canvas;
use crate::business::user_database::state;
use crate::error_code::ErrorCode;

/// 返回正常画布或者已经逻辑删除的画布。不产生日志。
///
/// # 参数
/// - `deleted`: 逻辑删除标志，false 返回正常画布，true 返回已逻辑删除的画布。
///
/// # 返回值
/// 返回查询到的画布列表；若发生错误则返回对应的 `ErrorCode`。
pub fn list(deleted: bool) -> Result<Vec<Canvas>, ErrorCode> {
    let connection = state::lock_connection();
    dao::select_by_deleted(&connection, deleted)
}
