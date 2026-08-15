use crate::business::metadata::entity::Metadata;
use crate::business::metadata::{dao, state};
use crate::error_code::ErrorCode;
use crate::util::time_util;

/// 注册一个用户数据库：只在数据库中添加有关这个数据库的记录，
/// 但不会实际创建数据库文件夹。
///
/// # 参数
/// - `name`: 数据库名称。
///
/// # 返回值
/// 返回新建记录的元数据；名称重复时返回 `ErrorCode::DatabaseNameAlreadyExists`，
/// 发生其他错误时返回对应的 `ErrorCode`。
pub fn register(name: String) -> Result<Metadata, ErrorCode> {
    let connection = state::lock_connection();
    if dao::select_by_name(&connection, &name)?.is_some() {
        return Err(ErrorCode::DatabaseNameAlreadyExists { name });
    }
    let now = time_util::now();
    let metadata = Metadata {
        id: uuid::Uuid::new_v4().to_string(),
        name,
        archived: false,
        create_time: now,
        modify_time: now,
        last_open_time: now,
    };
    dao::insert(&connection, &metadata)?;
    Ok(metadata)
}
