use crate::business::user_database::entity::{Action, Template, TemplateField};
use crate::business::user_database::node::dao as node_dao;
use crate::business::user_database::node_field::dao as node_field_dao;
use crate::business::user_database::template::dao;
use crate::business::user_database::{log, state};
use crate::error_code::ErrorCode;

/// 从指定节点的字段结构创建模板。
///
/// 产生 TemplateCreateFromNode 日志，载荷为模板名称和节点标题。
///
/// # 参数
/// - `node_id`: 源节点 id。
/// - `name`: 模板名称。
///
/// # 返回值
/// 返回新建的模板；节点不存在时返回 `ErrorCode::NoNodeWithSuchId`，
/// 模板名称已存在时返回 `ErrorCode::TemplateNameAlreadyExists`，
/// 发生其他错误时返回对应的 `ErrorCode`。
pub fn create_from_node(node_id: &str, name: String) -> Result<Template, ErrorCode> {
    let connection = state::lock_connection();
    let node = node_dao::select_by_id(&connection, node_id)?
        .ok_or_else(|| ErrorCode::NoNodeWithSuchId {
            id: node_id.to_string(),
        })?;
    if dao::select_by_name(&connection, &name)?.is_some() {
        return Err(ErrorCode::TemplateNameAlreadyExists { name });
    }
    let node_fields = node_field_dao::select_by_node_id(&connection, node_id)?;
    let template_id = uuid::Uuid::new_v4().to_string();
    let order = dao::max_order(&connection)? + 1;
    let template = Template {
        id: template_id.clone(),
        name: name.clone(),
        order,
    };
    dao::insert(&connection, &template)?;
    for (i, nf) in node_fields.iter().enumerate() {
        let field = TemplateField {
            template_id: template_id.clone(),
            name: nf.name.clone(),
            field_type: nf.field_type.clone(),
            type_config: nf.type_config.clone(),
            order: i as i64,
            dictionary_id: nf.dictionary_id.clone(),
        };
        dao::insert_field(&connection, &field)?;
    }
    log::service::create(
        &template_id,
        Action::TemplateCreateFromNode {
            template_name: name,
            node_title: node.title,
        },
    )?;
    Ok(template)
}
