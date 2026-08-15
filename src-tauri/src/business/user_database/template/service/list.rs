use crate::business::user_database::entity::Template;
use crate::business::user_database::template::dao;
use crate::business::user_database::state;
use crate::error_code::ErrorCode;

/// 查询全部模板，按 order 升序返回。
///
/// # 参数
/// 无。
///
/// # 返回值
/// 返回查询到的模板列表；若发生错误则返回对应的 `ErrorCode`。
pub fn list() -> Result<Vec<Template>, ErrorCode> {
    let connection = state::lock_connection();
    dao::select_all(&connection)
}
