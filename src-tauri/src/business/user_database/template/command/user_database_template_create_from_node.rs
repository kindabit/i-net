use crate::business::user_database::entity::Template;
use crate::business::user_database::template::service;
use crate::error_code::ErrorCode;
use crate::util::preprocess_util;

/// 从指定节点的字段结构创建模板。
///
/// # 参数
/// - `node_id`: 源节点 id。
/// - `name`: 模板名称。
///
/// # 返回值
/// 返回新建的模板；若发生错误则返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_template_create_from_node(
    node_id: String,
    name: String,
) -> Result<Template, ErrorCode> {
    preprocess(node_id, name)
}

/// `user_database_template_create_from_node` 的 preprocess 函数：校验参数后接入 service 层的 create_from_node 函数。
pub fn preprocess(node_id: String, name: String) -> Result<Template, ErrorCode> {
    let node_id = preprocess_util::preprocess_node_id(node_id)?;
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(ErrorCode::EmptyTemplateName);
    }
    service::create_from_node(&node_id, name)
}
