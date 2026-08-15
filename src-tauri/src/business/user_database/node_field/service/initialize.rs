use crate::business::user_database::node_field::dao;
use crate::business::user_database::state;
use crate::error_code::ErrorCode;

/// 初始化 node_field 子业务模块：新建 node_field 表。不产生日志。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn initialize() -> Result<(), ErrorCode> {
    let connection = state::lock_connection();
    dao::create_table(&connection)
}
