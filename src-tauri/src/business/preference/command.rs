pub mod preference_get;
pub mod preference_save;
pub mod preference_set;

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

        // preference_get::preprocess 失败路径：名称为空时报 EmptyPreferenceName。
        assert!(matches!(
            preference_get::preprocess("  ".to_string()),
            Err(ErrorCode::EmptyPreferenceName)
        ));

        // preference_set::preprocess 失败路径：名称为空时报 EmptyPreferenceName。
        assert!(matches!(
            preference_set::preprocess("".to_string(), "dark".to_string()),
            Err(ErrorCode::EmptyPreferenceName)
        ));

        // preference_set::preprocess 成功路径：设置偏好项。
        preference_set::preprocess("theme".to_string(), "dark".to_string()).unwrap();

        // preference_get::preprocess 成功路径：读取到刚设置的偏好项，
        // 名称两侧空白字符应被裁剪。
        assert_eq!(
            preference_get::preprocess(" theme ".to_string()).unwrap(),
            Some("dark".to_string())
        );

        // preference_save::preprocess 成功路径：保存后 preference.sqlite 文件存在。
        preference_save::preprocess().unwrap();
        assert!(path.preference_database_file.try_exists().unwrap());

        test::cleanup(&path);
    }
}
