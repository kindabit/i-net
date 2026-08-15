use crate::business::user_database::entity::{Action, Template};
use crate::business::user_database::template::dao;
use crate::business::user_database::{log, state};
use crate::error_code::ErrorCode;

/// 新建一个模板。
///
/// 产生 TemplateCreate 日志，载荷为模板名称。
///
/// # 参数
/// - `name`: 模板名称。
///
/// # 返回值
/// 返回新建的模板；名称已存在时返回 `ErrorCode::TemplateNameAlreadyExists`，
/// 发生其他错误时返回对应的 `ErrorCode`。
pub fn create(name: String) -> Result<Template, ErrorCode> {
    let connection = state::lock_connection();
    if dao::select_by_name(&connection, &name)?.is_some() {
        return Err(ErrorCode::TemplateNameAlreadyExists { name });
    }
    let id = uuid::Uuid::new_v4().to_string();
    let order = dao::max_order(&connection)? + 1;
    let template = Template { id: id.clone(), name: name.clone(), order };
    dao::insert(&connection, &template)?;
    log::service::create(
        &id,
        Action::TemplateCreate { name },
    )?;
    Ok(template)
}
