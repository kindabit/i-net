use std::collections::{HashMap, HashSet};

use crate::business::user_database::entity::{Action, NodeField, NodeFieldChange};
use crate::business::user_database::field_type::{self, FieldValue};
use crate::business::user_database::node::dao as node_dao;
use crate::business::user_database::node_field::dao;
use crate::business::user_database::node_field::vo::NodeFieldVO;
use crate::business::user_database::{dictionary, log, state};
use crate::error_code::ErrorCode;

/// 设置指定节点的字段集合（全量覆盖）：先删除旧字段再逐条插入新字段。
/// 生成 NodeFieldsModify 日志记录逐字段变更（Added / Modified / Removed），
/// 无变更时不产生日志。type_config / dictionary_id / order 的变化不纳入 diff。
///
/// # 参数
/// - `node_id`: 节点 id。
/// - `fields`: 要设置的字段列表，顺序即存储顺序。
///
/// # 返回值
/// 成功时返回 `Ok(())`；节点不存在时返回 `ErrorCode::NoNodeWithSuchId`，影子节点时返回 `ErrorCode::NodeIsShadow`，
/// 发生其他错误时返回对应的 `ErrorCode`。
pub fn set(node_id: &str, fields: &[NodeFieldVO]) -> Result<(), ErrorCode> {
    let connection = state::lock_connection();
    let node = node_dao::select_by_id(&connection, node_id)?.ok_or_else(|| {
        ErrorCode::NoNodeWithSuchId {
            id: node_id.to_string(),
        }
    })?;
    // 影子节点不允许此操作（展示数据从原始节点拉取，生命周期由边管理）。
    if node.shadow_id.is_some() {
        return Err(ErrorCode::NodeIsShadow);
    }
    let node_title = node.title.clone();

    let mut seen = HashSet::new();
    for f in fields {
        if !seen.insert(&f.name) {
            return Err(ErrorCode::DuplicateNodeFieldName {
                name: f.name.clone(),
            });
        }
    }

    for f in fields {
        let def = field_type::field_type_def(&f.field_type)?;
        field_type::validate_type_config(def, &f.type_config)?;
        field_type::validate_field_value(def, &f.name, &f.value)?;

        if let Some(ref dict_id) = f.dictionary_id {
            if !def.supports_dictionary {
                return Err(ErrorCode::FieldTypeNotSupportDictionary {
                    field_type: f.field_type.clone(),
                });
            }
            if !dictionary::dao::exist_by_id(&connection, dict_id)? {
                return Err(ErrorCode::NoDictionaryEntryWithSuchId {
                    id: dict_id.clone(),
                });
            }
        }
    }

    let key = state::key();
    let old_fields = dao::select_by_node_id(&connection, node_id)?;
    let mut old_map: HashMap<&str, (String, FieldValue)> = HashMap::new();
    for old in &old_fields {
        let value = field_type::decode(&old.field_type, old.field_value.clone(), &key)?;
        old_map.insert(old.name.as_str(), (old.field_type.clone(), value));
    }

    dao::delete_by_node_id(&connection, node_id)?;

    for (i, f) in fields.iter().enumerate() {
        let field_value = field_type::encode(&f.value, &key)?;
        let type_config = match &f.type_config {
            Some(v) => {
                Some(serde_json::to_string(v).map_err(|e| {
                    ErrorCode::FailToDeserializeNodeFieldValue {
                        detail: format!("Failed to serialize type_config: {e}"),
                    }
                })?)
            }
            None => None,
        };
        let node_field = NodeField {
            node_id: node_id.to_string(),
            name: f.name.clone(),
            field_type: f.field_type.clone(),
            type_config,
            field_value,
            order: i as i64,
            dictionary_id: f.dictionary_id.clone(),
        };
        dao::insert(&connection, &node_field)?;
    }

    let new_names: HashSet<&str> = fields.iter().map(|f| f.name.as_str()).collect();
    let mut changes: Vec<NodeFieldChange> = Vec::new();

    for old in &old_fields {
        if !new_names.contains(old.name.as_str()) {
            let (field_type, old_value) = old_map.remove(old.name.as_str()).unwrap();
            changes.push(NodeFieldChange::Removed {
                name: old.name.clone(),
                field_type,
                old_value,
            });
        }
    }

    for f in fields {
        if let Some((old_field_type, old_value)) = old_map.get(f.name.as_str()) {
            if old_field_type != &f.field_type || &f.value != old_value {
                changes.push(NodeFieldChange::Modified {
                    name: f.name.clone(),
                    old_field_type: old_field_type.clone(),
                    new_field_type: f.field_type.clone(),
                    old_value: old_value.clone(),
                    new_value: f.value.clone(),
                });
            }
        } else {
            changes.push(NodeFieldChange::Added {
                name: f.name.clone(),
                field_type: f.field_type.clone(),
                value: f.value.clone(),
            });
        }
    }

    if !changes.is_empty() {
        log::service::create(
            node_id,
            Action::NodeFieldsModify {
                node_title,
                changes,
            },
        )?;
    }

    Ok(())
}
