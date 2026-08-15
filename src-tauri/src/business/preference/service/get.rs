use crate::business::preference::state;
use crate::error_code::ErrorCode;

/// 从 preference 数据库中查询对应的偏好项。
///
/// # 参数
/// - `name`: 偏好项名称。
///
/// # 返回值
/// 返回偏好项的值，不存在时返回 `None`；若发生错误则返回对应的 `ErrorCode`。
pub fn get(name: &str) -> Result<Option<String>, ErrorCode> {
    let connection = state::lock_connection();
    crate::common::variable::service::get(&connection, name)
}
