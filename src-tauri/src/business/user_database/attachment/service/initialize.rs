use crate::business::user_database::attachment::dao;
use crate::business::user_database::state;
use crate::error_code::ErrorCode;

/// 初始化 attachment 子业务模块：新建 attachment 表。不产生日志。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn initialize() -> Result<(), ErrorCode> {
    let connection = state::lock_connection();
    dao::create_table(&connection)
}
