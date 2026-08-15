pub mod metadata_archive;
pub mod metadata_list;
pub mod metadata_physical_delete;
pub mod metadata_register;
pub mod metadata_save;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::business::metadata::service;
    use crate::common::connection;
    use crate::error_code::ErrorCode;
    use crate::state;
    use crate::test;
    use crate::util::{file_system_util, preprocess_util};

    /// 覆盖 metadata command 模块所有 preprocess 函数的成功与失败路径。
    #[test]
    fn test_metadata_command_all_functions() {
        let _guard = test::acquire_test_lock();
        // 每个测试都在自己的数据目录下进行，初始化自己的数据目录和 metadata 数据库。
        let path = test::create_test_path();
        state::set_path(path.clone());
        service::initialize().unwrap();

        // metadata_register::preprocess 失败路径：名称为空时报 EmptyUserDatabaseName。
        assert!(matches!(
            metadata_register::preprocess("  ".to_string()),
            Err(ErrorCode::EmptyUserDatabaseName)
        ));

        // metadata_register::preprocess 成功路径：注册数据库，名称两侧空白字符应被裁剪。
        let first = metadata_register::preprocess(" db-1 ".to_string()).unwrap();
        assert_eq!(first.name, "db-1");

        // metadata_register::preprocess 失败路径：名称重复时报 DatabaseNameAlreadyExists。
        assert!(matches!(
            metadata_register::preprocess("db-1".to_string()),
            Err(ErrorCode::DatabaseNameAlreadyExists { .. })
        ));

        // metadata_list::preprocess 成功路径：未归档列表包含刚注册的数据库。
        let unarchived = metadata_list::preprocess(false).unwrap();
        assert_eq!(unarchived.len(), 1);
        assert_eq!(unarchived[0].id, first.id);

        // metadata_archive::preprocess 失败路径：id 不是合法的 uuid 格式时报 InvalidUserDatabaseId。
        assert!(matches!(
            metadata_archive::preprocess("no-such-id".to_string(), true),
            Err(ErrorCode::InvalidUserDatabaseId { .. })
        ));

        // metadata_archive::preprocess 失败路径：parse_str 能解析的宽松格式
        // （无连字符、大写、花括号）同样应被拒绝。
        let uuid = uuid::Uuid::new_v4();
        for lenient in [
            uuid.simple().to_string(),
            uuid.to_string().to_uppercase(),
            uuid.braced().to_string(),
        ] {
            assert!(matches!(
                metadata_archive::preprocess(lenient, true),
                Err(ErrorCode::InvalidUserDatabaseId { .. })
            ));
        }

        // metadata_archive::preprocess 失败路径：id 格式合法但不存在时报 NoDatabaseWithSuchId。
        assert!(matches!(
            metadata_archive::preprocess(uuid::Uuid::new_v4().to_string(), true),
            Err(ErrorCode::NoDatabaseWithSuchId { .. })
        ));

        // metadata_archive::preprocess 成功路径：归档后数据库出现在归档列表。
        metadata_archive::preprocess(first.id.clone(), true).unwrap();
        assert_eq!(metadata_list::preprocess(true).unwrap().len(), 1);
        assert!(metadata_list::preprocess(false).unwrap().is_empty());

        // metadata_physical_delete::preprocess 失败路径：id 不是合法的 uuid 格式时
        // 报 InvalidUserDatabaseId。
        assert!(matches!(
            metadata_physical_delete::preprocess("no-such-id".to_string(), "password".to_string()),
            Err(ErrorCode::InvalidUserDatabaseId { .. })
        ));

        // metadata_physical_delete::preprocess 失败路径：密码为空时报 EmptyPassword。
        assert!(matches!(
            metadata_physical_delete::preprocess(first.id.clone(), "".to_string()),
            Err(ErrorCode::EmptyPassword)
        ));

        // metadata_physical_delete::preprocess 失败路径：密码错误时报 FailToDecrypt。
        // 先用正确密码对应的密钥为该数据库实际创建一个加密的用户数据库文件。
        let key = preprocess_util::preprocess_password("password".to_string()).unwrap();
        let database_directory = path.user_database_directory(&first.id);
        let database_file = path.user_database_file(&first.id);
        file_system_util::create_dir_all(&database_directory).unwrap();
        let user_database = connection::service::open_file(&database_file).unwrap();
        connection::service::save_file_encrypt(&database_file, &user_database, key).unwrap();
        drop(user_database);
        assert!(matches!(
            metadata_physical_delete::preprocess(first.id.clone(), "wrong".to_string()),
            Err(ErrorCode::FailToDecrypt { .. })
        ));

        // metadata_physical_delete::preprocess 失败路径：数据库未归档时
        // 报 DatabaseMustBeArchivedBeforeDelete。
        let second = metadata_register::preprocess("db-2".to_string()).unwrap();
        assert!(matches!(
            metadata_physical_delete::preprocess(second.id.clone(), "password".to_string()),
            Err(ErrorCode::DatabaseMustBeArchivedBeforeDelete)
        ));

        // metadata_physical_delete::preprocess 成功路径：密码正确时删除数据库目录和记录。
        metadata_physical_delete::preprocess(first.id.clone(), "password".to_string()).unwrap();
        assert!(!file_system_util::try_exists(&database_directory).unwrap());
        assert!(metadata_list::preprocess(true).unwrap().is_empty());

        // metadata_save::preprocess 成功路径：保存后 metadata.sqlite 文件存在。
        metadata_save::preprocess().unwrap();
        assert!(path.metadata_database_file.try_exists().unwrap());

        test::cleanup(&path);
    }
}
