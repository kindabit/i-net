use crate::business::metadata::{dao, state};
use crate::common::connection;
use crate::error_code::ErrorCode;
use crate::util::file_system_util;

/// 物理删除一个用户数据库：如果该数据库处于归档状态，并且使用传入的密钥
/// 能正确解密该数据库，则对该数据库执行物理删除，同步删除该数据库的目录。
///
/// # 参数
/// - `id`: 数据库 id。
/// - `key`: 32 字节的解密密钥。
///
/// # 返回值
/// 成功时返回 `Ok(())`；id 不存在时返回 `ErrorCode::NoDatabaseWithSuchId`，
/// 数据库未归档时返回 `ErrorCode::DatabaseMustBeArchivedBeforeDelete`，
/// 密钥无法正确解密时返回 `ErrorCode::FailToDecrypt`，
/// 发生其他错误时返回对应的 `ErrorCode`。
pub fn physical_delete(id: &str, key: [u8; 32]) -> Result<(), ErrorCode> {
    let connection = state::lock_connection();
    let metadata = dao::select_by_id(&connection, id)?
        .ok_or_else(|| ErrorCode::NoDatabaseWithSuchId { id: id.to_string() })?;
    if !metadata.archived {
        return Err(ErrorCode::DatabaseMustBeArchivedBeforeDelete);
    }
    let path = crate::state::path();
    // 通过实际解密一遍来验证密钥；数据库文件不存在时无需验证。
    let database_file = path.user_database_file(&metadata.id);
    if file_system_util::try_exists(&database_file)? {
        connection::service::open_file_encrypt(&database_file, key)?;
    }
    let database_directory = path.user_database_directory(&metadata.id);
    if file_system_util::try_exists(&database_directory)? {
        file_system_util::remove_dir_all(&database_directory)?;
    }
    dao::delete_by_id(&connection, id)
}
