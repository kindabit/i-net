use crate::business::user_database::attachment::dao;
use crate::business::user_database::state;
use crate::error_code::ErrorCode;
use crate::security::aes;
use crate::util::compress;
use crate::util::file_system_util;

/// 加载附件明文：读取附件文件并解密；compressed 为 true 时再经解压 guard 还原后返回明文内容。
/// 不检查逻辑删除标志（回收站中的附件允许预览/导出，
/// 便于用户确认内容后再决定恢复或物理删除）。不产生日志。
///
/// # 参数
/// - `id`: 附件 id。
///
/// # 返回值
/// 返回附件明文内容；附件不存在时返回 `ErrorCode::NoAttachmentWithSuchId`，
/// 附件文件缺失时返回 `ErrorCode::FailToReadFile`，发生其他错误时返回对应的 `ErrorCode`。
pub fn load(id: &str) -> Result<Vec<u8>, ErrorCode> {
    let connection = state::lock_connection();
    let attachment = dao::select_by_id(&connection, id)?.ok_or_else(|| {
        ErrorCode::NoAttachmentWithSuchId { id: id.to_string() }
    })?;
    let path = crate::state::path();
    let file = path.user_attachment_file(&state::metadata().id, id);
    if !file_system_util::try_exists(&file)? {
        return Err(ErrorCode::FailToReadFile {
            path: file.to_string_lossy().to_string(),
            detail: "Attachment file not found".to_string(),
        });
    }
    let ciphertext = file_system_util::read(&file)?;
    let plaintext = aes::decrypt(ciphertext, state::key())?;
    if !attachment.compressed {
        return Ok(plaintext);
    }
    compress::decompress(&attachment.compress_param, plaintext)
}
