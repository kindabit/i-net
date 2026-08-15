use crate::business::user_database::state;
use crate::common::variable;
use crate::error_code::ErrorCode;

/// 按名称查询 registry 变量的值。
///
/// # 参数
/// - `name`: 变量名称。
///
/// # 返回值
/// 返回变量的值，不存在时返回 `None`；若发生错误则返回对应的 `ErrorCode`。
pub fn get(name: &str) -> Result<Option<String>, ErrorCode> {
    let connection = state::lock_connection();
    variable::service::get(&connection, name)
}
