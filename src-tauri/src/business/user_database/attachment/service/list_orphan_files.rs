use std::collections::HashSet;

use crate::business::user_database::attachment::dao;
use crate::business::user_database::state;
use crate::error_code::ErrorCode;
use crate::util::file_system_util;

/// 列出孤儿附件文件：附件目录中存在、但 attachment 表中没有对应元数据的文件。
/// 只上报不清理；文件名主干无法解析为 uuid 的文件也纳入，原样返回文件名，
/// 便于发现异常文件。附件目录不存在时返回空列表。不产生日志。
///
/// # 返回值
/// 返回孤儿文件的 id（解析不出 uuid 时为原文件名）；若发生错误则返回对应的 `ErrorCode`。
pub fn list_orphan_files() -> Result<Vec<String>, ErrorCode> {
    let path = crate::state::path();
    let directory = path.user_attachment_directory(&state::metadata().id);
    if !file_system_util::try_exists(&directory)? {
        return Ok(Vec::new());
    }
    let mut file_ids = Vec::new();
    for entry in file_system_util::read_dir(&directory)? {
        let entry = entry.map_err(|e| ErrorCode::FailToReadDirectory {
            path: directory.to_string_lossy().to_string(),
            detail: e.to_string(),
        })?;
        let file_name = entry.file_name().to_string_lossy().to_string();
        // 附件文件命名为 <uuid>.bin：取主干解析 uuid（解析出的 uuid 规范化为标准格式，
        // 以便与表内 id 比对），解析不出时原样保留文件名。
        let id = file_name
            .strip_suffix(".bin")
            .and_then(|stem| uuid::Uuid::parse_str(stem).ok())
            .map(|uuid| uuid.to_string())
            .unwrap_or(file_name);
        file_ids.push(id);
    }
    let connection = state::lock_connection();
    let existing: HashSet<String> = dao::select_all_ids(&connection)?.into_iter().collect();
    Ok(file_ids
        .into_iter()
        .filter(|id| !existing.contains(id))
        .collect())
}
