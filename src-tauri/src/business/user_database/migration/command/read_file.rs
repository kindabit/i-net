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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test;

    /// 迁移源文件字节读取接口：成功路径（读出的字节与写入一致、.kdbx 与小写扩展名 .kdbx /
    /// 大写 .KDBX 均通过 preprocess 校验）与失败路径（非 .kdbx 扩展名、无扩展名、空路径、
    /// 文件不存在的 .kdbx 路径）。
    #[test]
    fn test_read_file() {
        let _guard = test::acquire_test_lock();

        let path = test::create_test_path();

        // ===== 迁移源文件字节读取接口：成功与失败路径 =====
        // 向测试数据目录写一个已知内容的 .kdbx 临时文件，读取的字节与写入一致
        //（IPC 响应体无私有字段访问器，字节断言经读取函数完成，command 仅做包装）。
        let file_path = path.data_directory.join("migration-source.kdbx");
        let content = b"i-net migration read file test bytes".to_vec();
        std::fs::write(&file_path, &content).unwrap();
        assert_eq!(
            read_file_bytes(file_path.to_str().unwrap()).unwrap(),
            content
        );
        // command 层 preprocess 成功路径：有效 .kdbx 路径返回 Ok。
        assert!(
            preprocess(file_path.to_string_lossy().to_string()).is_ok()
        );

        // 成功路径：大写扩展名 .KDBX 同样接受（Windows 文件系统大小写不敏感）。
        let upper_path = path.data_directory.join("migration-source-upper.KDBX");
        std::fs::write(&upper_path, &content).unwrap();
        assert!(
            preprocess(upper_path.to_string_lossy().to_string()).is_ok()
        );

        // 失败路径：扩展名不是 .kdbx 时返回 InvalidKdbxFileExtension；
        // 文件实际存在且可读，证明拒绝来自扩展名校验而非读取失败。
        let wrong_ext_path = path.data_directory.join("migration-source.bin");
        std::fs::write(&wrong_ext_path, &content).unwrap();
        assert!(matches!(
            preprocess(wrong_ext_path.to_string_lossy().to_string()),
            Err(ErrorCode::InvalidKdbxFileExtension { .. })
        ));
        // 失败路径：无扩展名同样返回 InvalidKdbxFileExtension。
        let no_ext_path = path.data_directory.join("migration-source");
        std::fs::write(&no_ext_path, &content).unwrap();
        assert!(matches!(
            preprocess(no_ext_path.to_string_lossy().to_string()),
            Err(ErrorCode::InvalidKdbxFileExtension { .. })
        ));

        // 失败路径：空路径（空串与纯空白）返回 EmptyFilePath。
        assert!(matches!(
            preprocess(String::new()),
            Err(ErrorCode::EmptyFilePath)
        ));
        assert!(matches!(
            preprocess("  ".to_string()),
            Err(ErrorCode::EmptyFilePath)
        ));

        // 失败路径：不存在的 .kdbx 路径返回 FailToReadFile（扩展名校验通过后的读取失败）。
        let missing_path = path.data_directory.join("no-such-file.kdbx");
        assert!(matches!(
            preprocess(missing_path.to_string_lossy().to_string()),
            Err(ErrorCode::FailToReadFile { .. })
        ));
        assert!(matches!(
            read_file_bytes(missing_path.to_str().unwrap()),
            Err(ErrorCode::FailToReadFile { .. })
        ));

        test::cleanup(&path);
    }
}
