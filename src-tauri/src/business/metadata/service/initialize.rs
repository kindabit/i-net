use crate::business::metadata::{dao, state};
use crate::common::connection;
use crate::error_code::ErrorCode;

/// 初始化 metadata 模块：读取 metadata 数据库，构造 connection 并将其存入
/// metadata 的 state。程序启动时调用该函数初始化 metadata state。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn initialize() -> Result<(), ErrorCode> {
    let path = crate::state::path();
    let connection = connection::service::open_file(&path.metadata_database_file)?;
    if !dao::exist_table(&connection)? {
        dao::create_table(&connection)?;
    }
    state::set_connection(connection);
    Ok(())
}
