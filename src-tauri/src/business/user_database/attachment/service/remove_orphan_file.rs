use crate::business::user_database::state;
use crate::error_code::ErrorCode;
use crate::util::file_system_util;

/// 删除孤儿附件文件：物理删除附件目录中无元数据的附件文件。
/// 不动表、不记日志（该文件无元数据身份）；文件不存在时不视为错误。
///
/// # 参数
/// - `id`: 孤儿文件 id（command 层已做 uuid 往返校验，杜绝路径穿越）。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn remove_orphan_file(id: &str) -> Result<(), ErrorCode> {
    let path = crate::state::path();
    let file = path.user_attachment_file(&state::metadata().id, id);
    if file_system_util::try_exists(&file)? {
        file_system_util::remove_file(&file)?;
    }
    Ok(())
}
