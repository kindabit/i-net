use crate::business::user_database::node::dao as node_dao;
use crate::business::user_database::node_field::dao;
use crate::business::user_database::node_field::vo::NodeFieldVO;
use crate::business::user_database::state;
use crate::error_code::ErrorCode;

/// 将字段值密文解密为明文字符串；blob 为 None 时返回 None。
/// 解密成功但明文不是合法 UTF-8 时返回 `ErrorCode::DataCorruptionNodeFieldValueInvalidUtf8`。
fn decrypt_value(
    node_id: &str,
    name: &str,
    blob: Option<Vec<u8>>,
    key: &[u8; 32],
) -> Result<Option<String>, ErrorCode> {
    let blob = match blob {
        Some(b) => b,
        None => return Ok(None),
    };
    let plaintext = crate::security::aes::decrypt(blob, *key)?;
    let value = String::from_utf8(plaintext).map_err(|_| {
        ErrorCode::DataCorruptionNodeFieldValueInvalidUtf8 {
            node_id: node_id.to_string(),
            name: name.to_string(),
        }
    })?;
    Ok(Some(value))
}

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
        let value = decrypt_value(node_id, &field.name, field.field_value, &key)?;
        vos.push(NodeFieldVO {
            name: field.name,
            field_type: field.field_type,
            value,
            dictionary_id: field.dictionary_id,
        });
    }
    Ok(vos)
}
