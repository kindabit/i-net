use crate::business::user_database::dictionary::service;
use crate::business::user_database::entity::Dictionary;
use crate::error_code::ErrorCode;

/// 设置字典条目集合（全量覆盖）。
///
/// # 参数
/// - `entries`: 要设置的字典条目列表。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_dictionary_set(entries: Vec<Dictionary>) -> Result<(), ErrorCode> {
    preprocess(entries)
}

/// `user_database_dictionary_set` 的 preprocess 函数：校验参数后接入 service 层的 set 函数。
///
/// 条目 id 校验 uuid 格式；parent_id 在 Some 时校验 uuid 格式；value trim 后回写。
pub fn preprocess(mut entries: Vec<Dictionary>) -> Result<(), ErrorCode> {
    for entry in &mut entries {
        match uuid::Uuid::parse_str(&entry.id) {
            Ok(uuid) if uuid.to_string() == entry.id => {}
            _ => {
                return Err(ErrorCode::InvalidDictionaryId {
                    id: entry.id.clone(),
                })
            }
        }
        if let Some(ref pid) = entry.parent_id {
            match uuid::Uuid::parse_str(pid) {
                Ok(uuid) if uuid.to_string() == *pid => {}
                _ => {
                    return Err(ErrorCode::InvalidDictionaryId {
                        id: pid.clone(),
                    })
                }
            }
        }
        entry.value = entry.value.trim().to_string();
        if entry.value.is_empty() {
            return Err(ErrorCode::EmptyDictionaryValue);
        }
    }
    service::set(&entries)
}
