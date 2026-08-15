use crate::business::user_database::state;
use crate::error_code::ErrorCode;

/// 插入或更新 registry 变量。
///
/// # 参数
/// - `name`: 变量名称。
/// - `value`: 变量值。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn set(name: &str, value: &str) -> Result<(), ErrorCode> {
    let connection = state::lock_connection();
    crate::common::variable::service::set(&connection, name, value)
}
