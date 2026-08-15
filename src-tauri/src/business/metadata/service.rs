mod archive;
mod initialize;
mod list;
mod physical_delete;
mod register;
mod save;

pub use archive::archive;
pub use initialize::initialize;
pub use list::list;
pub use physical_delete::physical_delete;
pub use register::register;
pub use save::save;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::business::metadata::dao;
    use crate::common::connection;
    use crate::error_code::ErrorCode;
    use crate::state;
    use crate::test;
    use crate::util::file_system_util;

    /// 覆盖 metadata service 模块所有 service 函数的成功与失败路径。
    #[test]
    fn test_metadata_service_all_functions() {
        let _guard = test::acquire_test_lock();
        // 每个测试都在自己的数据目录下进行，初始化自己的数据目录和 metadata 数据库。
        let path = test::create_test_path();
        state::set_path(path.clone());

        // initialize 成功路径：metadata.sqlite 不存在时直接在内存中建立 connection。
        initialize().unwrap();

        // register 成功路径：注册数据库后返回的 Metadata 字段符合预期，
        // 且 register 只添加记录，不会实际创建数据库文件夹。
        let first = register("db-1".to_string()).unwrap();
        assert_eq!(first.name, "db-1");
        assert!(!first.archived);
        assert!(first.create_time > 0);
        assert_eq!(first.create_time, first.modify_time);
        assert_eq!(first.create_time, first.last_open_time);
        assert!(!file_system_util::try_exists(&path.user_database_directory(&first.id)).unwrap());

        // register 失败路径：数据库名称重复时报 DatabaseNameAlreadyExists。
        assert!(matches!(
            register("db-1".to_string()),
            Err(ErrorCode::DatabaseNameAlreadyExists { .. })
        ));

        // list 成功路径：未归档列表包含全部未归档数据库。
        let second = register("db-2".to_string()).unwrap();
        let unarchived = list(false).unwrap();
        assert_eq!(unarchived.len(), 2);
        assert!(list(true).unwrap().is_empty());

        // archive 失败路径：id 不存在时报 NoDatabaseWithSuchId。
        assert!(matches!(
            archive("no-such-id", true),
            Err(ErrorCode::NoDatabaseWithSuchId { .. })
        ));

        // archive 成功路径：归档后数据库从未归档列表移动到归档列表。
        archive(&first.id, true).unwrap();
        let unarchived = list(false).unwrap();
        assert_eq!(unarchived.len(), 1);
        assert_eq!(unarchived[0].id, second.id);
        let archived = list(true).unwrap();
        assert_eq!(archived.len(), 1);
        assert_eq!(archived[0].id, first.id);
        assert!(archived[0].archived);

        // physical_delete 失败路径：数据库未归档时报 DatabaseMustBeArchivedBeforeDelete。
        assert!(matches!(
            physical_delete(&second.id, test::test_key()),
            Err(ErrorCode::DatabaseMustBeArchivedBeforeDelete)
        ));

        // physical_delete 失败路径：id 不存在时报 NoDatabaseWithSuchId。
        assert!(matches!(
            physical_delete("no-such-id", test::test_key()),
            Err(ErrorCode::NoDatabaseWithSuchId { .. })
        ));

        // physical_delete 失败路径：密钥无法正确解密该数据库时报 FailToDecrypt。
        // 先为该数据库实际创建一个加密的用户数据库文件。
        let database_directory = path.user_database_directory(&first.id);
        let database_file = path.user_database_file(&first.id);
        file_system_util::create_dir_all(&database_directory).unwrap();
        let user_database = connection::service::open_file(&database_file).unwrap();
        connection::service::save_file_encrypt(&database_file, &user_database, test::test_key())
            .unwrap();
        drop(user_database);
        assert!(matches!(
            physical_delete(&first.id, [2u8; 32]),
            Err(ErrorCode::FailToDecrypt { .. })
        ));

        // physical_delete 成功路径：密钥正确时同步删除数据库目录和元数据记录。
        physical_delete(&first.id, test::test_key()).unwrap();
        assert!(!file_system_util::try_exists(&database_directory).unwrap());
        let connection = crate::business::metadata::state::lock_connection();
        assert!(dao::select_by_id(&connection, &first.id).unwrap().is_none());
        drop(connection);

        // archive 成功路径：archived 传 false 时解除归档，数据库回到未归档列表。
        archive(&second.id, true).unwrap();
        assert_eq!(list(true).unwrap().len(), 1);
        archive(&second.id, false).unwrap();
        assert!(list(true).unwrap().is_empty());
        assert_eq!(list(false).unwrap().len(), 1);

        // save 成功路径：保存后 metadata.sqlite 文件存在。
        save().unwrap();
        assert!(path.metadata_database_file.try_exists().unwrap());

        // initialize 成功路径：重新初始化后能从文件中恢复保存的元数据。
        initialize().unwrap();
        let unarchived = list(false).unwrap();
        assert_eq!(unarchived.len(), 1);
        assert_eq!(unarchived[0].name, "db-2");

        test::cleanup(&path);
    }
}
