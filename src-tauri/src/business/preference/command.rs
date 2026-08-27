pub mod get;
pub mod save;
pub mod set;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::business::preference::service;
    use crate::error_code::ErrorCode;
    use crate::state;
    use crate::test;

    /// 覆盖 preference command 模块所有 preprocess 函数的成功与失败路径。
    #[test]
    fn test_preference_command_all_functions() {
        let _guard = test::acquire_test_lock();
        // 每个测试都在自己的数据目录下进行，初始化自己的数据目录和 preference 数据库。
        let path = test::create_test_path();
        state::set_path(path.clone());
        service::initialize().unwrap();

        // get::preprocess 失败路径：名称为空时报 EmptyPreferenceName。
        assert!(matches!(
            get::preprocess("  ".to_string()),
            Err(ErrorCode::EmptyPreferenceName)
        ));

        // set::preprocess 失败路径：名称为空时报 EmptyPreferenceName。
        assert!(matches!(
            set::preprocess("".to_string(), "dark".to_string()),
            Err(ErrorCode::EmptyPreferenceName)
        ));

        // set::preprocess 成功路径：设置偏好项。
        set::preprocess("theme".to_string(), "dark".to_string()).unwrap();

        // get::preprocess 成功路径：读取到刚设置的偏好项，
        // 名称两侧空白字符应被裁剪。
        assert_eq!(
            get::preprocess(" theme ".to_string()).unwrap(),
            Some("dark".to_string())
        );

        // save::preprocess 成功路径：保存后 preference.sqlite 文件存在。
        save::preprocess().unwrap();
        assert!(path.preference_database_file.try_exists().unwrap());

        test::cleanup(&path);
    }
}
