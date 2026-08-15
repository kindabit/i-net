use crate::business::preference::state;
use crate::common::connection;
use crate::error_code::ErrorCode;

/// 保存 preference connection 至文件。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn save() -> Result<(), ErrorCode> {
    let path = crate::state::path();
    let connection = state::lock_connection();
    connection::service::save_file(&path.preference_database_file, &connection)
}
