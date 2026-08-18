//! 计算当前数据目录的大小（递归求和所有非 `logs/` 文件），
//! 用于前端预估备份体积。

use std::path::Path;

use walkdir::WalkDir;

use crate::error_code::ErrorCode;

/// 计算当前数据目录的大小（递归求和所有非 `logs/` 文件），用于前端预估备份大小。
///
/// # 参数
///
/// * `data_directory` - 应用数据根目录。
///
/// # 返回值
///
/// 成功时返回所有非 `logs/` 文件的字节总数；若遍历或读取元数据失败则返回对应的 `ErrorCode`。
pub fn data_directory_size(data_directory: &Path) -> Result<u64, ErrorCode> {
    let iter = WalkDir::new(data_directory).into_iter().filter_entry(|e| {
        // 根目录自身不参与过滤，避免误伤；其余条目按文件名剔除 logs 子目录。
        if e.depth() == 0 {
            return true;
        }
        e.file_name() != "logs"
    });
    let mut total = 0u64;
    for entry in iter {
        let entry = entry.map_err(|e| ErrorCode::FailToPackBackup {
            detail: format!("failed to iterate: {}", e),
        })?;
        if !entry.file_type().is_file() {
            continue;
        }
        let len = entry
            .metadata()
            .map_err(|e| ErrorCode::FailToPackBackup {
                detail: format!("failed to read metadata: {}", e),
            })?
            .len();
        total += len;
    }
    Ok(total)
}