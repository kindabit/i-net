pub mod archive;
pub mod list;
pub mod physical_delete;
pub mod register;
pub mod save;

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

        // register::preprocess 失败路径：名称为空时报 EmptyUserDatabaseName。
        assert!(matches!(
            register::preprocess("  ".to_string()),
            Err(ErrorCode::EmptyUserDatabaseName)
        ));

        // register::preprocess 成功路径：注册数据库，名称两侧空白字符应被裁剪。
        let first = register::preprocess(" db-1 ".to_string()).unwrap();
        assert_eq!(first.name, "db-1");

        // register::preprocess 失败路径：名称重复时报 DatabaseNameAlreadyExists。
        assert!(matches!(
            register::preprocess("db-1".to_string()),
            Err(ErrorCode::DatabaseNameAlreadyExists { .. })
        ));

        // list::preprocess 成功路径：未归档列表包含刚注册的数据库。
        let unarchived = list::preprocess(false).unwrap();
        assert_eq!(unarchived.len(), 1);
        assert_eq!(unarchived[0].id, first.id);

        // archive::preprocess 失败路径：id 不是合法的 uuid 格式时报 InvalidUserDatabaseId。
        assert!(matches!(
            archive::preprocess("no-such-id".to_string(), true),
            Err(ErrorCode::InvalidUserDatabaseId { .. })
        ));

        // archive::preprocess 失败路径：parse_str 能解析的宽松格式
        // （无连字符、大写、花括号）同样应被拒绝。
        let uuid = uuid::Uuid::new_v4();
        for lenient in [
            uuid.simple().to_string(),
            uuid.to_string().to_uppercase(),
            uuid.braced().to_string(),
        ] {
            assert!(matches!(
                archive::preprocess(lenient, true),
                Err(ErrorCode::InvalidUserDatabaseId { .. })
            ));
        }

        // archive::preprocess 失败路径：id 格式合法但不存在时报 NoDatabaseWithSuchId。
        assert!(matches!(
            archive::preprocess(uuid::Uuid::new_v4().to_string(), true),
            Err(ErrorCode::NoDatabaseWithSuchId { .. })
        ));

        // archive::preprocess 成功路径：归档后数据库出现在归档列表。
        archive::preprocess(first.id.clone(), true).unwrap();
        assert_eq!(list::preprocess(true).unwrap().len(), 1);
        assert!(list::preprocess(false).unwrap().is_empty());

        // physical_delete::preprocess 失败路径：id 不是合法的 uuid 格式时
        // 报 InvalidUserDatabaseId。
        assert!(matches!(
            physical_delete::preprocess("no-such-id".to_string(), "password".to_string()),
            Err(ErrorCode::InvalidUserDatabaseId { .. })
        ));

        // physical_delete::preprocess 失败路径：密码为空时报 EmptyPassword。
        assert!(matches!(
            physical_delete::preprocess(first.id.clone(), "".to_string()),
            Err(ErrorCode::EmptyPassword)
        ));

        // physical_delete::preprocess 失败路径：密码错误时报 FailToDecrypt。
        // 先用正确密码对应的密钥为该数据库实际创建一个加密的用户数据库文件。
        let key = preprocess_util::preprocess_password("password".to_string()).unwrap();
        let database_directory = path.user_database_directory(&first.id);
        let database_file = path.user_database_file(&first.id);
        file_system_util::create_dir_all(&database_directory).unwrap();
        let user_database = connection::service::open_file(&database_file).unwrap();
        connection::service::save_file_encrypt(&database_file, &user_database, key).unwrap();
        drop(user_database);
        assert!(matches!(
            physical_delete::preprocess(first.id.clone(), "wrong".to_string()),
            Err(ErrorCode::FailToDecrypt { .. })
        ));

        // physical_delete::preprocess 失败路径：数据库未归档时
        // 报 DatabaseMustBeArchivedBeforeDelete。
        let second = register::preprocess("db-2".to_string()).unwrap();
        assert!(matches!(
            physical_delete::preprocess(second.id.clone(), "password".to_string()),
            Err(ErrorCode::DatabaseMustBeArchivedBeforeDelete)
        ));

        // physical_delete::preprocess 成功路径：密码正确时删除数据库目录和记录。
        physical_delete::preprocess(first.id.clone(), "password".to_string()).unwrap();
        assert!(!file_system_util::try_exists(&database_directory).unwrap());
        assert!(list::preprocess(true).unwrap().is_empty());

        // save::preprocess 成功路径：保存后 metadata.sqlite 文件存在。
        save::preprocess().unwrap();
        assert!(path.metadata_database_file.try_exists().unwrap());

        test::cleanup(&path);
    }
}
