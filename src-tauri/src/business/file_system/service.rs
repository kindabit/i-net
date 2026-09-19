//! 只读文件系统查询的业务逻辑实现。
//!
//! - [`home_directory`]：获取当前用户主目录。
//! - [`list_directory`]：读取指定目录下的条目。
//! - [`roots`]：获取文件系统根路径列表。
//!
//! 全部为只读操作。

mod home_directory;
mod list_directory;
mod roots;

pub use home_directory::home_directory;
pub use list_directory::list_directory;
pub use roots::roots;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error_code::ErrorCode;
    use crate::test;

    /// 成功路径：混合目录与文件、大小写混合名称时目录优先、名称升序，
    /// 条目的名称、路径、是否为目录以及列表的当前路径与父路径均正确。
    #[test]
    fn test_list_directory_success() {
        let path_state = test::create_test_path();
        let root = &path_state.data_directory;
        let directory = root.join("listing");

        // 构造被测目录：两个子目录（大小写混合）与两个文件。
        std::fs::create_dir_all(directory.join("beta")).unwrap();
        std::fs::create_dir_all(directory.join("Alpha")).unwrap();
        std::fs::write(directory.join("zeta.txt"), b"z").unwrap();
        std::fs::write(directory.join("apple.txt"), b"a").unwrap();

        let listing = list_directory(&directory).unwrap();

        // 排序：目录优先，其次名称忽略大小写升序。
        let names: Vec<&str> = listing
            .entries
            .iter()
            .map(|entry| entry.name.as_str())
            .collect();
        assert_eq!(names, vec!["Alpha", "beta", "apple.txt", "zeta.txt"]);

        // 是否为目录：前两个条目为目录，后两个为文件。
        assert!(listing.entries[0].is_directory);
        assert!(listing.entries[1].is_directory);
        assert!(!listing.entries[2].is_directory);
        assert!(!listing.entries[3].is_directory);

        // 条目路径为目录下的完整路径，名称为条目本身的名称。
        assert_eq!(
            listing.entries[0].path,
            directory.join("Alpha").to_string_lossy().to_string()
        );
        assert_eq!(
            listing.entries[3].path,
            directory.join("zeta.txt").to_string_lossy().to_string()
        );

        // 当前路径为传入的目录路径，父路径为临时根目录。
        assert_eq!(listing.path, directory.to_string_lossy().to_string());
        assert_eq!(listing.parent, Some(root.to_string_lossy().to_string()));

        test::cleanup(&path_state);
    }

    /// 失败路径：路径不存在或路径不是目录时返回 FailToReadDirectory。
    #[test]
    fn test_list_directory_failure() {
        let path_state = test::create_test_path();
        let root = &path_state.data_directory;

        // 路径不存在。
        assert!(matches!(
            list_directory(&root.join("missing")),
            Err(ErrorCode::FailToReadDirectory { .. })
        ));

        // 路径是文件而不是目录。
        let file = root.join("plain.txt");
        std::fs::write(&file, b"x").unwrap();
        assert!(matches!(
            list_directory(&file),
            Err(ErrorCode::FailToReadDirectory { .. })
        ));

        test::cleanup(&path_state);
    }

    /// 成功路径：主目录存在且为绝对路径。
    #[test]
    fn test_home_directory_is_absolute() {
        let home = home_directory().unwrap();
        assert!(home.is_absolute());
    }

    /// 成功路径：根路径列表非空且不含空字符串；
    /// 非 Windows 平台进一步断言仅包含 `/`。
    #[test]
    fn test_roots() {
        let roots = roots().unwrap();
        assert!(!roots.is_empty());
        assert!(roots.iter().all(|root| !root.is_empty()));

        #[cfg(not(windows))]
        assert_eq!(roots, vec!["/".to_string()]);
    }
}
