use rusqlite::Connection;

use crate::common::data_version::{constant, dao};
use crate::error_code::ErrorCode;

/// 处理 connection 的数据版本：data_version 表不存在时新建表并插入当前数据版本，
/// 表已存在时校验表内有且只有一行与当前版本一致的数据。
///
/// # 参数
/// - `connection`: 数据库连接。
///
/// # 返回值
/// 成功时返回 `Ok(())`；数据版本校验失败时返回对应的 `ErrorCode`。
pub fn process(connection: &Connection) -> Result<(), ErrorCode> {
    if !dao::exist_table(connection)? {
        dao::create_table(connection)?;
        dao::insert(connection, &constant::DATA_VERSION)?;
        return Ok(());
    }
    let versions = dao::select(connection)?;
    match versions.len() {
        0 => Err(ErrorCode::NoDataVersion),
        1 => {
            let actual = versions[0];
            if actual == constant::DATA_VERSION {
                Ok(())
            } else {
                Err(ErrorCode::DataVersionMismatch {
                    expected: constant::DATA_VERSION,
                    actual,
                })
            }
        }
        _ => Err(ErrorCode::MultipleDataVersion),
    }
}
