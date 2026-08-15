use crate::business::user_database::entity::Action;
use crate::business::user_database::template::dao;
use crate::business::user_database::{log, state};
use crate::error_code::ErrorCode;

/// 重命名指定模板。
///
/// 产生 TemplateRename 日志，载荷为旧名称和新名称。
///
/// # 参数
/// - `id`: 模板 id。
/// - `new_name`: 新模板名称。
///
/// # 返回值
/// 成功时返回 `Ok(())`；模板不存在时返回 `ErrorCode::NoTemplateWithSuchId`，
/// 新名称与其他模板重复时返回 `ErrorCode::TemplateNameAlreadyExists`，
/// 发生其他错误时返回对应的 `ErrorCode`。
pub fn rename(id: &str, new_name: String) -> Result<(), ErrorCode> {
    let connection = state::lock_connection();
    let template = dao::select_by_id(&connection, id)?
        .ok_or_else(|| ErrorCode::NoTemplateWithSuchId {
            id: id.to_string(),
        })?;
    let old_name = template.name;
    if let Some(existing) = dao::select_by_name(&connection, &new_name)? {
        if existing.id != id {
            return Err(ErrorCode::TemplateNameAlreadyExists {
                name: new_name,
            });
        }
    }
    if old_name == new_name {
        return Ok(());
    }
    dao::update_name(&connection, id, &new_name)?;
    log::service::create(
        id,
        Action::TemplateRename {
            old_name,
            new_name,
        },
    )?;
    Ok(())
}
