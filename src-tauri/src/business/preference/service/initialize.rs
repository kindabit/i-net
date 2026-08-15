use crate::business::preference::state;
use crate::common::connection;
use crate::error_code::ErrorCode;

/// 初始化 preference 模块：读取 preference 数据库，构造 connection 并将其存入
/// preference 的 state。程序启动时调用该函数初始化 preference state。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn initialize() -> Result<(), ErrorCode> {
    let path = crate::state::path();
    let connection = connection::service::open_file(&path.preference_database_file)?;
    if !crate::common::variable::dao::exist_table(&connection)? {
        crate::common::variable::dao::create_table(&connection)?;
    }
    state::set_connection(connection);
    Ok(())
}
