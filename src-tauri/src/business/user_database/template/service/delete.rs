use crate::business::user_database::entity::Action;
use crate::business::user_database::template::dao;
use crate::business::user_database::{log, state};
use crate::error_code::ErrorCode;

/// 物理删除指定模板；其全部字段定义由外键 ON DELETE CASCADE 随模板行的删除一并删除。
///
/// 产生 TemplateDelete 日志，载荷为模板名称。
///
/// # 参数
/// - `id`: 模板 id。
///
/// # 返回值
/// 成功时返回 `Ok(())`；模板不存在时返回 `ErrorCode::NoTemplateWithSuchId`，
/// 发生其他错误时返回对应的 `ErrorCode`。
pub fn delete(id: &str) -> Result<(), ErrorCode> {
    let connection = state::lock_connection();
    let template = dao::select_by_id(&connection, id)?
        .ok_or_else(|| ErrorCode::NoTemplateWithSuchId {
            id: id.to_string(),
        })?;
    let name = template.name;
    dao::delete_by_id(&connection, id)?;
    log::service::create(
        id,
        Action::TemplateDelete { name },
    )?;
    Ok(())
}
