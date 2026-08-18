pub mod metadata;
pub mod preference;
pub mod user_database;

#[cfg(test)]
mod tests {
    use crate::business::metadata::dao as metadata_dao;
    use crate::business::metadata::entity::Metadata;
    use crate::business::preference::state as preference_state;
    use crate::common::connection;
    use crate::common::variable::dao as variable_dao;
    use crate::state;
    use crate::test;
    use crate::util::file_system_util;

    /// 验证 reclaim 在「磁盘被外部替换、内存还是旧数据」的场景下能正确恢复。
    /// 该测试复现并固化了引入全量还原后「exit-save 覆盖还原结果」的 bug。
    #[test]
    fn test_reclaim_refreshes_in_memory_state_after_disk_replacement() {
        let _guard = test::acquire_test_lock();
        let path = test::create_test_path();
        state::set_path(path.clone());

        // 1. 初始化 metadata + preference（in-memory 空，只有 data_version）。
        crate::business::metadata::service::initialize().unwrap();
        crate::business::preference::service::initialize().unwrap();

        // 2. 准备一份「已知内容」的 metadata.sqlite 和 preference.sqlite，
        //    直接写到对应路径（模拟 restore 已经把磁盘替换好）。
        let meta_prepared = Metadata {
            id: "restored-id".to_string(),
            name: "restored-db".to_string(),
            archived: false,
            create_time: 100,
            modify_time: 100,
            last_open_time: 100,
        };
        {
            let conn = rusqlite::Connection::open_in_memory().unwrap();
            crate::common::data_version::service::process(&conn).unwrap();
            metadata_dao::create_table(&conn).unwrap();
            metadata_dao::insert(&conn, &meta_prepared).unwrap();
            connection::service::save_file(&path.metadata_database_file, &conn).unwrap();
        }
        {
            let conn = rusqlite::Connection::open_in_memory().unwrap();
            crate::common::data_version::service::process(&conn).unwrap();
            variable_dao::create_table(&conn).unwrap();
            conn.execute(
                "INSERT INTO variable (name, value) VALUES ('theme', 'restored-theme')",
                [],
            )
            .unwrap();
            connection::service::save_file(&path.preference_database_file, &conn).unwrap();
        }

        // 3. 此时磁盘已经是「还原后的内容」，但 in-memory 还是步骤 1 的空状态。
        //    关键：不能重新调用 initialize()，因为这正是 bug 修复要避免的依赖路径。

        // 4. 调用 reclaim：metadata + preference。
        super::metadata::reclaim_metadata().unwrap();
        super::preference::reclaim_preference().unwrap();

        // 5. 验证 in-memory 现在指向磁盘上的「还原后的内容」。
        {
            let conn = crate::business::metadata::state::lock_connection();
            let row = metadata_dao::select_by_name(&conn, "restored-db").unwrap();
            assert!(row.is_some(), "metadata in-memory should reflect restored data");
            assert_eq!(row.unwrap().id, "restored-id");
        }
        {
            let conn = preference_state::lock_connection();
            let value: String = conn
                .query_row(
                    "SELECT value FROM variable WHERE name = 'theme'",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(value, "restored-theme");
        }

        // sanity: 文件确实还在（reclaim 没误删）
        assert!(file_system_util::try_exists(&path.metadata_database_file).unwrap());
        assert!(file_system_util::try_exists(&path.preference_database_file).unwrap());

        crate::test::cleanup(&path);
    }

    /// 验证 reclaim_user_database 在「没有 user_database 打开」时是无害的 no-op。
    #[test]
    fn test_reclaim_user_database_is_noop_when_nothing_open() {
        let _guard = test::acquire_test_lock();
        let path = test::create_test_path();
        state::set_path(path.clone());

        // 不初始化任何 user_database，直接 reclaim。
        super::user_database::reclaim_user_database().unwrap();

        crate::test::cleanup(&path);
    }
}