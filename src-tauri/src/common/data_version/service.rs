mod process;

pub use process::process;

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use super::*;
    use crate::common::data_version::{constant, dao, entity::DataVersion};
    use crate::error_code::ErrorCode;

    /// 覆盖 data_version service 模块 process 函数的成功与失败路径。
    #[test]
    fn test_data_version_service_all_functions() {
        // process 成功路径：全新的 connection 没有 data_version 表，
        // process 会建表并插入当前数据版本。
        let connection = Connection::open_in_memory().unwrap();
        process(&connection).unwrap();
        assert_eq!(
            dao::select(&connection).unwrap(),
            vec![constant::DATA_VERSION]
        );

        // process 成功路径：已含正确数据版本的 connection 再次处理时幂等成功。
        process(&connection).unwrap();
        assert_eq!(dao::select(&connection).unwrap().len(), 1);

        // process 失败路径：表存在但版本不一致时报 DataVersionMismatch。
        let mismatched = Connection::open_in_memory().unwrap();
        dao::create_table(&mismatched).unwrap();
        let wrong_version = DataVersion {
            major: 9,
            minor: 9,
            patch: 9,
        };
        dao::insert(&mismatched, &wrong_version).unwrap();
        assert!(matches!(
            process(&mismatched),
            Err(ErrorCode::DataVersionMismatch { .. })
        ));

        // process 失败路径：表存在但没有任何数据时报 NoDataVersion。
        let empty = Connection::open_in_memory().unwrap();
        dao::create_table(&empty).unwrap();
        assert!(matches!(process(&empty), Err(ErrorCode::NoDataVersion)));

        // process 失败路径：表内存在多行数据时报 MultipleDataVersion。
        let multiple = Connection::open_in_memory().unwrap();
        dao::create_table(&multiple).unwrap();
        dao::insert(&multiple, &constant::DATA_VERSION).unwrap();
        dao::insert(&multiple, &constant::DATA_VERSION).unwrap();
        assert!(matches!(
            process(&multiple),
            Err(ErrorCode::MultipleDataVersion)
        ));
    }
}
