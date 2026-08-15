use crate::business::preference::state;
use crate::error_code::ErrorCode;

/// 向 preference 数据库插入或更新偏好项。
///
/// # 参数
/// - `name`: 偏好项名称。
/// - `value`: 偏好项值。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn set(name: &str, value: &str) -> Result<(), ErrorCode> {
    let connection = state::lock_connection();
    crate::common::variable::service::set(&connection, name, value)
}
