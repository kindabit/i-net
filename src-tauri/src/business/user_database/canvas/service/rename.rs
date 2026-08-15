use crate::business::user_database::canvas::dao;
use crate::business::user_database::entity::Action;
use crate::business::user_database::{log, state};
use crate::error_code::ErrorCode;

/// 修改指定画布的名称，先检测新名称是否与其它画布重复。
///
/// 产生 CanvasRename 日志，载荷内记录画布的旧名称和新名称。
///
/// # 参数
/// - `id`: 画布 id。
/// - `new_name`: 新名称。
///
/// # 返回值
/// 成功时返回 `Ok(())`；画布不存在时返回 `ErrorCode::NoCanvasWithSuchId`，
/// 新名称与其它画布重复时返回 `ErrorCode::CanvasNameAlreadyExists`，
/// 发生其他错误时返回对应的 `ErrorCode`。
pub fn rename(id: &str, new_name: String) -> Result<(), ErrorCode> {
    let connection = state::lock_connection();
    let mut canvas = dao::select_by_id(&connection, id)?
        .ok_or_else(|| ErrorCode::NoCanvasWithSuchId { id: id.to_string() })?;
    if let Some(existing) = dao::select_by_name(&connection, &new_name)? {
        if existing.id != id {
            return Err(ErrorCode::CanvasNameAlreadyExists { name: new_name });
        }
    }
    let old_name = std::mem::replace(&mut canvas.name, new_name);
    dao::update(&connection, &canvas)?;
    log::service::create(
        id,
        Action::CanvasRename {
            old_name,
            new_name: canvas.name,
        },
    )?;
    Ok(())
}
