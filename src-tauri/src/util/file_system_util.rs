use std::path::Path;

use crate::error_code::ErrorCode;

/// 判断指定路径是否存在。
///
/// # 参数
///
/// * `path` - 需要检查的路径。
///
/// # 返回值
///
/// 返回路径是否存在的布尔值；若发生错误则返回对应的 `ErrorCode`。
pub fn try_exists(path: &Path) -> Result<bool, ErrorCode> {
    path.try_exists().map_err(|e| ErrorCode::FailToTryExists {
        path: path.to_string_lossy().to_string(),
        detail: e.to_string(),
    })
}

/// 读取指定目录下的条目。
///
/// # 参数
///
/// * `path` - 需要读取的目录路径。
///
/// # 返回值
///
/// 返回目录迭代器；若发生错误则返回对应的 `ErrorCode`。
pub fn read_dir(path: &Path) -> Result<std::fs::ReadDir, ErrorCode> {
    std::fs::read_dir(path).map_err(|e| ErrorCode::FailToReadDirectory {
        path: path.to_string_lossy().to_string(),
        detail: e.to_string(),
    })
}

/// 递归创建指定目录及其所有父目录。
///
/// # 参数
///
/// * `path` - 需要创建的目录路径。
///
/// # 返回值
///
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn create_dir_all(path: &Path) -> Result<(), ErrorCode> {
    std::fs::create_dir_all(path).map_err(|e| ErrorCode::FailToCreateDirectory {
        path: path.to_string_lossy().to_string(),
        detail: e.to_string(),
    })
}

/// 删除指定文件。
///
/// # 参数
///
/// * `path` - 需要删除的文件路径。
///
/// # 返回值
///
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn remove_file(path: &Path) -> Result<(), ErrorCode> {
    std::fs::remove_file(path).map_err(|e| ErrorCode::FailToRemoveFile {
        path: path.to_string_lossy().to_string(),
        detail: e.to_string(),
    })
}

/// 将二进制数据写入指定文件。
///
/// # 参数
///
/// * `path` - 目标文件路径。
/// * `data` - 需要写入的字节数据。
///
/// # 返回值
///
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn write(path: &Path, data: &[u8]) -> Result<(), ErrorCode> {
    std::fs::write(path, data).map_err(|e| ErrorCode::FailToWriteFile {
        path: path.to_string_lossy().to_string(),
        detail: e.to_string(),
    })
}

/// 递归删除指定目录及其所有内容。
///
/// # 参数
///
/// * `path` - 需要删除的目录路径。
///
/// # 返回值
///
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn remove_dir_all(path: &Path) -> Result<(), ErrorCode> {
    std::fs::remove_dir_all(path).map_err(|e| ErrorCode::FailToRemoveDirectory {
        path: path.to_string_lossy().to_string(),
        detail: e.to_string(),
    })
}

/// 读取指定文件的全部内容。
///
/// # 参数
///
/// * `path` - 需要读取的文件路径。
///
/// # 返回值
///
/// 返回文件内容的字节数组；若发生错误则返回对应的 `ErrorCode`。
pub fn read(path: &Path) -> Result<Vec<u8>, ErrorCode> {
    std::fs::read(path).map_err(|e| ErrorCode::FailToReadFile {
        path: path.to_string_lossy().to_string(),
        detail: e.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test;

    /// 覆盖 file_system_util 模块所有文件系统辅助函数的成功与失败路径。
    #[test]
    fn test_file_system_util_all_functions() {
        let path_state = test::create_test_path();
        let dir = &path_state.data_directory;

        // try_exists 成功路径：文件不存在时返回 false。
        let file = dir.join("test.txt");
        assert!(!try_exists(&file).unwrap());

        // write 成功路径：写入字节数据后文件存在。
        write(&file, b"hello").unwrap();
        assert!(try_exists(&file).unwrap());

        // read 成功路径：读取内容与写入内容一致。
        let content = read(&file).unwrap();
        assert_eq!(content, b"hello");

        // read 失败路径：读取不存在的文件返回 FailToReadFile。
        let missing_file = dir.join("does-not-exist.txt");
        assert!(matches!(
            read(&missing_file),
            Err(ErrorCode::FailToReadFile { .. })
        ));

        // remove_file 失败路径：删除不存在的文件返回 FailToRemoveFile。
        assert!(matches!(
            remove_file(&missing_file),
            Err(ErrorCode::FailToRemoveFile { .. })
        ));

        // remove_file 成功路径：删除后文件不存在。
        remove_file(&file).unwrap();
        assert!(!try_exists(&file).unwrap());

        // create_dir_all 成功路径：递归创建多级目录。
        let nested = dir.join("a").join("b").join("c");
        create_dir_all(&nested).unwrap();
        assert!(nested.try_exists().unwrap());

        // read_dir 成功路径：读取目录返回正确的条目数量。
        let sub = dir.join("read_dir_test");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(sub.join("f1.txt"), b"1").unwrap();
        std::fs::write(sub.join("f2.txt"), b"2").unwrap();
        let entries: Vec<_> = read_dir(&sub).unwrap().collect();
        assert_eq!(entries.len(), 2);

        // read_dir 失败路径：读取不存在的目录返回 FailToReadDirectory。
        let missing_dir = dir.join("missing-dir");
        assert!(matches!(
            read_dir(&missing_dir),
            Err(ErrorCode::FailToReadDirectory { .. })
        ));

        // remove_dir_all 失败路径：删除不存在的目录返回 FailToRemoveDirectory。
        assert!(matches!(
            remove_dir_all(&missing_dir),
            Err(ErrorCode::FailToRemoveDirectory { .. })
        ));

        // remove_dir_all 成功路径：递归删除目录及其内容。
        remove_dir_all(&dir.join("a")).unwrap();
        assert!(!nested.try_exists().unwrap());

        test::cleanup(&path_state);
    }
}
