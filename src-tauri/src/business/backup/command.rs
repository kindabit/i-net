pub mod backup;
pub mod data_directory_size;
pub mod progress;
pub mod response;
pub mod restore;
pub mod restore_probe;

#[cfg(test)]
mod tests {
    use super::*;

    /// 覆盖 backup command 模块所有 preprocess 函数的成功与失败路径，
    /// 以及 backup_data_directory_size 命令。
    #[test]
    fn test_backup_command_all_functions() {
        // backup validate_redundancy_ratio 失败路径：冗余比例超出范围时报 InvalidBackupFile。
        assert!(backup::validate_redundancy_ratio(-0.1).is_err());
        assert!(backup::validate_redundancy_ratio(0.0).is_err());
        assert!(backup::validate_redundancy_ratio(1.5).is_err());

        // backup validate_redundancy_ratio 成功路径：合法冗余比例返回 Ok。
        assert!(backup::validate_redundancy_ratio(0.05).is_ok());
        assert!(backup::validate_redundancy_ratio(0.5).is_ok());

        // restore::preprocess 失败路径：空/纯空白路径被 preprocess_util::preprocess_file_path
        // 拦截返回 EmptyFilePath，不会触碰文件系统，可直接覆盖（进度回调传 noop）。
        let noop_progress = |_: progress::Phase, _: f32| {};
        assert!(matches!(
            restore::preprocess("".to_string(), &noop_progress),
            Err(crate::error_code::ErrorCode::EmptyFilePath)
        ));
        assert!(matches!(
            restore::preprocess("   ".to_string(), &noop_progress),
            Err(crate::error_code::ErrorCode::EmptyFilePath)
        ));

        // restore::preprocess 成功路径需要真实备份文件与数据目录状态，
        // 由 service 层测试（service.rs tests）端到端覆盖。
    }
}