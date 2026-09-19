use crate::error_code::ErrorCode;

/// 获取文件系统根路径列表。
///
/// # 参数
///
/// 无。
///
/// # 返回值
///
/// Windows 平台返回各磁盘挂载点（形如 `C:\`），枚举结果为空时依次回退到
/// 系统盘环境变量与 `C:\`；其它平台返回 `/`。
#[cfg(windows)]
pub fn roots() -> Result<Vec<String>, ErrorCode> {
    let disks = sysinfo::Disks::new_with_refreshed_list();
    let mut roots: Vec<String> = disks
        .list()
        .iter()
        .map(|disk| disk.mount_point().to_string_lossy().to_string())
        .collect();

    // 枚举结果为空时回退到系统盘环境变量，环境变量缺失时再回退到 C:\。
    if roots.is_empty() {
        let fallback = std::env::var("SystemDrive")
            .map(|system_drive| format!("{system_drive}\\"))
            .unwrap_or_else(|_| "C:\\".to_string());
        roots.push(fallback);
    }

    Ok(roots)
}

/// 获取文件系统根路径列表。
///
/// # 参数
///
/// 无。
///
/// # 返回值
///
/// 返回唯一的根目录 `/`。
#[cfg(not(windows))]
pub fn roots() -> Result<Vec<String>, ErrorCode> {
    Ok(vec!["/".to_string()])
}
