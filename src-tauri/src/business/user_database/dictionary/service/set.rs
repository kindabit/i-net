use std::collections::HashSet;

use crate::business::user_database::dictionary::dao;
use crate::business::user_database::entity::{Action, Dictionary};
use crate::business::user_database::{log, node_field, state, template};
use crate::error_code::ErrorCode;

/// 设置字典条目集合（全量覆盖）：先清空再批量插入。
/// 完成后清理 node_field 和 template_field 中引用已不存在字典条目的悬空 id。
/// 产生 DictionarySet 日志。
///
/// # 参数
/// - `entries`: 要设置的字典条目列表。
///
/// # 返回值
/// 成功时返回 `Ok(())`；发生错误时返回对应的 `ErrorCode`。
pub fn set(entries: &[Dictionary]) -> Result<(), ErrorCode> {
    let connection = state::lock_connection();

    let mut seen = HashSet::new();
    for entry in entries {
        if !seen.insert(&entry.id) {
            return Err(ErrorCode::DuplicateDictionaryId {
                id: entry.id.clone(),
            });
        }
    }

    let id_set: HashSet<&String> = entries.iter().map(|e| &e.id).collect();
    for entry in entries {
        if let Some(ref pid) = entry.parent_id {
            if !id_set.contains(pid) {
                return Err(ErrorCode::NoDictionaryEntryWithSuchId {
                    id: pid.clone(),
                });
            }
        }
    }

    dao::delete_all(&connection)?;

    dao::batch_insert(&connection, entries)?;

    node_field::dao::clear_dangling_dictionary_ids(&connection)?;
    template::dao::clear_dangling_field_dictionary_ids(&connection)?;

    let entry_count = entries.len() as i64;

    log::service::create(
        "dictionary",
        Action::DictionarySet { entry_count },
    )?;

    Ok(())
}
