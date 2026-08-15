use crate::business::metadata::{dao, state};
use crate::error_code::ErrorCode;

/// 设置指定 id 的用户数据库的归档状态。
///
/// # 参数
/// - `id`: 数据库 id。
/// - `archived`: 归档状态，`true` 表示归档，`false` 表示解除归档。
///
/// # 返回值
/// 成功时返回 `Ok(())`；id 不存在时返回 `ErrorCode::NoDatabaseWithSuchId`，
/// 发生其他错误时返回对应的 `ErrorCode`。
pub fn archive(id: &str, archived: bool) -> Result<(), ErrorCode> {
    let connection = state::lock_connection();
    let mut metadata = dao::select_by_id(&connection, id)?
        .ok_or_else(|| ErrorCode::NoDatabaseWithSuchId { id: id.to_string() })?;
    metadata.archived = archived;
    dao::update(&connection, &metadata)
}
