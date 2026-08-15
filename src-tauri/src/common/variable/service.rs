mod get;
mod set;

pub use get::get;
pub use set::set;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error_code::ErrorCode;
    use rusqlite::Connection;

    /// 覆盖 variable service 模块 get / set 两个函数的成功与失败路径。
    #[test]
    fn test_variable_service_all_functions() {
        let connection = Connection::open_in_memory().unwrap();

        // get 失败路径：表不存在时报 DatabaseError。
        assert!(matches!(
            get(&connection, "theme"),
            Err(ErrorCode::DatabaseError { .. })
        ));

        // set 失败路径：表不存在时报 DatabaseError。
        assert!(matches!(
            set(&connection, "theme", "dark"),
            Err(ErrorCode::DatabaseError { .. })
        ));

        crate::common::variable::dao::create_table(&connection).unwrap();

        // get 成功路径：变量不存在时返回 None。
        assert!(get(&connection, "theme").unwrap().is_none());

        // set 成功路径：写入变量后 get 读到相同的值。
        set(&connection, "theme", "dark").unwrap();
        assert_eq!(
            get(&connection, "theme").unwrap(),
            Some("dark".to_string())
        );

        // set 成功路径：对已存在的变量执行更新。
        set(&connection, "theme", "light").unwrap();
        assert_eq!(
            get(&connection, "theme").unwrap(),
            Some("light".to_string())
        );
    }
}
