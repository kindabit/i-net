use std::path::Path;

use crate::business::file_system::vo::{DirectoryEntryVO, DirectoryListingVO};
use crate::error_code::ErrorCode;
use crate::util::file_system_util;

/// 读取指定目录下的条目。
///
/// 条目排序规则为目录优先，其次按名称忽略大小写升序，名称相同时保持读取顺序。
///
/// # 参数
///
/// * `path` - 需要读取的目录路径。
///
/// # 返回值
///
/// 返回包含当前目录路径、父目录路径与条目列表的目录读取结果；
/// 路径不存在、不是目录或读取过程中发生错误时返回
/// [`ErrorCode::FailToReadDirectory`]。
pub fn list_directory(path: &Path) -> Result<DirectoryListingVO, ErrorCode> {
    let read_dir = file_system_util::read_dir(path)?;

    let mut entries = Vec::new();
    for entry in read_dir {
        let entry = entry.map_err(|e| ErrorCode::FailToReadDirectory {
            path: path.to_string_lossy().to_string(),
            detail: e.to_string(),
        })?;
        let is_directory = entry
            .file_type()
            .map_err(|e| ErrorCode::FailToReadDirectory {
                path: path.to_string_lossy().to_string(),
                detail: e.to_string(),
            })?
            .is_dir();
        entries.push(DirectoryEntryVO {
            name: entry.file_name().to_string_lossy().to_string(),
            path: entry.path().to_string_lossy().to_string(),
            is_directory,
        });
    }

    // 目录优先，其次名称忽略大小写升序；sort_by 为稳定排序，名称相同时保持读取顺序。
    entries.sort_by(|a, b| {
        b.is_directory
            .cmp(&a.is_directory)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    // 根目录没有父目录；相对路径无上级时 parent 为空路径，同样视为没有父目录。
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .map(|parent| parent.to_string_lossy().to_string());

    Ok(DirectoryListingVO {
        path: path.to_string_lossy().to_string(),
        parent,
        entries,
    })
}
