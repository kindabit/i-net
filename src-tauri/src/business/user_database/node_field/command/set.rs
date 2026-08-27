use crate::business::user_database::node_field::service;
use crate::business::user_database::node_field::vo::NodeFieldVO;
use crate::error_code::ErrorCode;
use crate::util::preprocess_util;

/// 设置指定节点的字段集合（全量覆盖）。
///
/// # 参数
/// - `node_id`: 节点 id。
/// - `fields`: 要设置的字段列表。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_node_field_set(
    node_id: String,
    fields: Vec<NodeFieldVO>,
) -> Result<(), ErrorCode> {
    preprocess(node_id, fields)
}

/// `user_database_node_field_set` 的 preprocess 函数：校验参数后接入 service 层的 set 函数。
///
/// 字段名称 trim 后回写；dictionary_id 在 Some 时校验 uuid 格式。
pub fn preprocess(
    node_id: String,
    mut fields: Vec<NodeFieldVO>,
) -> Result<(), ErrorCode> {
    let node_id = preprocess_util::preprocess_node_id(node_id)?;
    for f in &mut fields {
        f.name = f.name.trim().to_string();
        if f.name.is_empty() {
            return Err(ErrorCode::EmptyNodeFieldName);
        }
        if let Some(ref dict_id) = f.dictionary_id {
            match uuid::Uuid::parse_str(dict_id) {
                Ok(uuid) if uuid.to_string() == *dict_id => {}
                _ => {
                    return Err(ErrorCode::InvalidDictionaryId {
                        id: dict_id.clone(),
                    })
                }
            }
        }
    }
    service::set(&node_id, &fields)
}
