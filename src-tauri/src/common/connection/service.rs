mod open_file;
mod open_file_encrypt;
mod save_file;
mod save_file_encrypt;

pub use open_file::open_file;
pub use open_file_encrypt::open_file_encrypt;
pub use save_file::save_file;
pub use save_file_encrypt::save_file_encrypt;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::data_version;
    use crate::error_code::ErrorCode;
    use crate::test;
    use crate::util::file_system_util;

    /// 覆盖 connection service 模块所有函数的成功与失败路径。
    #[test]
    fn test_connection_service_all_functions() {
        let path = test::create_test_path();
        let plain_file = path.data_directory.join("plain.sqlite");
        let encrypt_file = path.data_directory.join("encrypt.sqlite");

        // open_file 成功路径：路径不存在时直接在内存中建立 connection，
        // 并由 data_version 建表插入当前版本。
        let connection = open_file(&plain_file).unwrap();
        assert_eq!(
            data_version::dao::select(&connection).unwrap(),
            vec![data_version::constant::DATA_VERSION]
        );

        // save_file 成功路径：保存后文件存在，且文件中包含 connection 内的数据。
        connection
            .execute("CREATE TABLE t (value TEXT NOT NULL) STRICT", [])
            .unwrap();
        connection
            .execute("INSERT INTO t (value) VALUES ('hello')", [])
            .unwrap();
        save_file(&plain_file, &connection).unwrap();
        assert!(file_system_util::try_exists(&plain_file).unwrap());
        let reopened = open_file(&plain_file).unwrap();
        let value: String = reopened
            .query_row("SELECT value FROM t", [], |row| row.get(0))
            .unwrap();
        assert_eq!(value, "hello");

        // open_file 失败路径：data_version 被篡改后再次打开时报 DataVersionMismatch。
        reopened
            .execute("UPDATE data_version SET major = 9", [])
            .unwrap();
        save_file(&plain_file, &reopened).unwrap();
        assert!(matches!(
            open_file(&plain_file),
            Err(ErrorCode::DataVersionMismatch { .. })
        ));

        // open_file 失败路径：文件内容不是合法的 sqlite 数据时，
        // 反序列化不报错，错误在 data_version 的 dao 查询处暴露，
        // dao 层构造的 DatabaseError 自然穿过 service 层返回。
        file_system_util::write(&plain_file, b"not a sqlite file").unwrap();
        assert!(matches!(
            open_file(&plain_file),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // open_file_encrypt 成功路径：路径不存在时直接在内存中建立 connection。
        let key = test::test_key();
        let connection = open_file_encrypt(&encrypt_file, key).unwrap();
        connection
            .execute("CREATE TABLE t (value TEXT NOT NULL) STRICT", [])
            .unwrap();
        connection
            .execute("INSERT INTO t (value) VALUES ('secret')", [])
            .unwrap();

        // save_file_encrypt 成功路径：保存后文件存在，且文件内容不是明文 sqlite，
        // 直接以未加密方式打开时报 dao 层自然传播的 DatabaseError。
        save_file_encrypt(&encrypt_file, &connection, key).unwrap();
        assert!(file_system_util::try_exists(&encrypt_file).unwrap());
        assert!(matches!(
            open_file(&encrypt_file),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // open_file_encrypt 失败路径：使用错误密钥打开时报 FailToDecrypt。
        assert!(matches!(
            open_file_encrypt(&encrypt_file, [2u8; 32]),
            Err(ErrorCode::FailToDecrypt { .. })
        ));

        // open_file_encrypt 成功路径：使用正确密钥打开后能读到保存的数据。
        let reopened = open_file_encrypt(&encrypt_file, key).unwrap();
        let value: String = reopened
            .query_row("SELECT value FROM t", [], |row| row.get(0))
            .unwrap();
        assert_eq!(value, "secret");

        drop(reopened);
        test::cleanup(&path);
    }
}
