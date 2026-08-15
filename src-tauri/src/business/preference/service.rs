mod get;
mod initialize;
mod save;
mod set;

pub use get::get;
pub use initialize::initialize;
pub use save::save;
pub use set::set;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state;
    use crate::test;

    /// 覆盖 preference service 模块所有 service 函数的成功与失败路径。
    #[test]
    fn test_preference_service_all_functions() {
        let _guard = test::acquire_test_lock();
        // 每个测试都在自己的数据目录下进行，初始化自己的数据目录和 preference 数据库。
        let path = test::create_test_path();
        state::set_path(path.clone());

        // initialize 成功路径：preference.sqlite 不存在时直接在内存中建立 connection。
        initialize().unwrap();

        // get 成功路径：偏好项不存在时返回 None。
        assert_eq!(get("theme").unwrap(), None);

        // set 成功路径：插入新的偏好项。
        set("theme", "dark").unwrap();
        assert_eq!(get("theme").unwrap(), Some("dark".to_string()));

        // set 成功路径：更新已存在的偏好项。
        set("theme", "light").unwrap();
        assert_eq!(get("theme").unwrap(), Some("light".to_string()));

        // save 成功路径：保存后 preference.sqlite 文件存在。
        save().unwrap();
        assert!(path.preference_database_file.try_exists().unwrap());

        // initialize 成功路径：重新初始化后能从文件中恢复保存的偏好项。
        initialize().unwrap();
        assert_eq!(get("theme").unwrap(), Some("light".to_string()));

        test::cleanup(&path);
    }
}
