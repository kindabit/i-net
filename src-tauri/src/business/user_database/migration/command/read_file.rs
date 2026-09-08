use crate::error_code::ErrorCode;
use crate::util::preprocess_util;

/// 读取迁移源文件的全部字节，文件内容对后端完全透明（由前端负责解密与解析）。
///
/// # 参数
/// - `path`: 文件路径。
///
/// # 返回值
/// 返回文件的全部字节；读取失败时返回 `ErrorCode::FailToReadFile`。
pub fn read_file_bytes(path: &str) -> Result<Vec<u8>, ErrorCode> {
    std::fs::read(path).map_err(|e| ErrorCode::FailToReadFile {
        path: path.to_string(),
        detail: e.to_string(),
    })
}

/// 读取迁移源文件的全部字节并经原始二进制 IPC 通道返回给前端，
/// 文件内容由前端解密与解析（见概念地图·数据迁移（KeePass 2.0 导入））。
///
/// # 参数
/// - `path`: 文件路径。
///
/// # 返回值
/// 成功时返回包含文件全部字节的 IPC 响应；路径为空时返回 `ErrorCode::EmptyFilePath`，
/// 扩展名不是 .kdbx（不区分大小写）时返回 `ErrorCode::InvalidKdbxFileExtension`，
/// 读取失败时返回 `ErrorCode::FailToReadFile`。
#[tauri::command]
pub fn user_database_migration_read_file(path: String) -> Result<tauri::ipc::Response, ErrorCode> {
    preprocess(path)
}

/// `user_database_migration_read_file` 的 preprocess 函数：校验路径非空且扩展名为
/// .kdbx（不区分大小写）后读取文件字节。该接口不是通用文件读取器，扩展名白名单
/// 限制其只能读取 KeePass 2.0 数据库文件。
///
/// # 参数
/// - `path`: 文件路径。
///
/// # 返回值
/// 成功时返回包含文件全部字节的 IPC 响应；路径为空时返回 `ErrorCode::EmptyFilePath`，
/// 扩展名不是 .kdbx（不区分大小写）时返回 `ErrorCode::InvalidKdbxFileExtension`，
/// 读取失败时返回 `ErrorCode::FailToReadFile`。
pub fn preprocess(path: String) -> Result<tauri::ipc::Response, ErrorCode> {
    let path = preprocess_util::preprocess_file_path(path)?;
    let is_kdbx = std::path::Path::new(&path)
        .extension()
        .map(|ext| ext.eq_ignore_ascii_case("kdbx"))
        .unwrap_or(false);
    if !is_kdbx {
        return Err(ErrorCode::InvalidKdbxFileExtension { path });
    }
    let bytes = read_file_bytes(&path)?;
    Ok(tauri::ipc::Response::new(bytes))
}
