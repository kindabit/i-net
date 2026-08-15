use crate::business::user_database::field_type;
use crate::business::user_database::node::dao as node_dao;
use crate::business::user_database::node_field::dao;
use crate::business::user_database::node_field::vo::NodeFieldVO;
use crate::business::user_database::state;
use crate::error_code::ErrorCode;

/// 获取指定节点的全部字段，按存储顺序返回。
///
/// # 参数
/// - `node_id`: 节点 id。
///
/// # 返回值
/// 返回字段值对象列表；节点不存在时返回 `ErrorCode::NoNodeWithSuchId`，
/// 发生其他错误时返回对应的 `ErrorCode`。
pub fn get(node_id: &str) -> Result<Vec<NodeFieldVO>, ErrorCode> {
    let connection = state::lock_connection();
    node_dao::select_by_id(&connection, node_id)?.ok_or_else(|| {
        ErrorCode::NoNodeWithSuchId {
            id: node_id.to_string(),
        }
    })?;
    let fields = dao::select_by_node_id(&connection, node_id)?;
    let key = state::key();
    let mut vos = Vec::with_capacity(fields.len());
    for field in fields {
        let value = field_type::decode(&field.field_type, field.field_value, &key)?;
        let type_config = match field.type_config {
            Some(s) => Some(serde_json::from_str(&s).map_err(|e| {
                ErrorCode::FailToDeserializeNodeFieldValue {
                    detail: format!("Failed to deserialize type_config: {e}"),
                }
            })?),
            None => None,
        };
        vos.push(NodeFieldVO {
            name: field.name,
            field_type: field.field_type,
            type_config,
            value,
            dictionary_id: field.dictionary_id,
        });
    }
    Ok(vos)
}
