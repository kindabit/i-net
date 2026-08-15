use crate::business::user_database::attachment::service;
use crate::error_code::ErrorCode;
use crate::util::preprocess_util;

/// 删除孤儿附件文件：物理删除附件目录中无元数据的附件文件，不可恢复。
///
/// # 参数
/// - `id`: 孤儿文件 id。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_attachment_remove_orphan_file(id: String) -> Result<(), ErrorCode> {
    preprocess(id)
}

/// `user_database_attachment_remove_orphan_file` 的 preprocess 函数：校验 id 后接入 service 层的
/// remove_orphan_file 函数；id 经 uuid 往返校验，杜绝路径穿越。
pub fn preprocess(id: String) -> Result<(), ErrorCode> {
    let id = preprocess_util::preprocess_attachment_id(id)?;
    service::remove_orphan_file(&id)
}
