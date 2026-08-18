//! 备份与还原的业务流程实现。
//!
//! - [`pack`]：持久化 → 打包 → 编码 → 写入目标文件。
//! - [`unpack`]：读取 → 校验 → 解码 → 解压 → 替换。
//! - [`probe`]：仅做 header/shard 校验并返回结构化结果，不修改数据。
//! - [`data_directory_size`]：递归求和数据目录大小（跳过 `logs/`），用于前端预估备份体积。

mod data_directory_size;
mod pack;
mod probe;
mod unpack;

pub use data_directory_size::data_directory_size;
pub use pack::pack;
pub use probe::probe;
pub use unpack::unpack;

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::{Read as _, Seek as _, Write as _};

    use sha2::{Digest, Sha256};

    use super::*;
    use crate::business::backup::codec::{compute_shard_params, encode_shards, reconstruct_shards};
    use crate::business::backup::format::{Header, HEADER_SIZE};
    use crate::business::backup::progress::Phase;
    use crate::error_code::ErrorCode;
    use crate::state;

    // 子模块中以 `pub(super)` 暴露的内部 helper，集中测试通过 super::<mod>::<fn> 访问。
    use super::pack::{build_tar, compute_shard_checksums, write_backup_file};
    use super::unpack::verify_shard_region;

    /// 不关注进度断言时使用的 noop 进度回调。
    fn noop_progress(_: Phase, _: f32) {}

    /// 覆盖完整 pack → 损坏中段 shard → probe → 修复 → 校验的端到端流程。
    #[test]
    fn test_pack_and_unpack_round_trip_with_corruption() {
        let _guard = crate::test::acquire_test_lock();
        let path = crate::test::create_test_path();
        state::set_path(path.clone());

        // 在数据目录里准备若干文件（含一个子目录与一个非日志子目录）。
        std::fs::create_dir_all(path.user_database_set_directory.join("fake-db"))
            .unwrap();
        std::fs::write(
            path.user_database_set_directory.join("fake-db").join("a.sqlite"),
            b"hello world",
        )
        .unwrap();
        std::fs::write(
            &path.metadata_database_file,
            b"some metadata bytes",
        )
        .unwrap();
        std::fs::create_dir_all(path.log_directory.join("nested")).unwrap();
        std::fs::write(
            path.log_directory.join("nested").join("today.log"),
            b"ignore me",
        )
        .unwrap();

        let backup_path = path.data_directory.join("test.ibackup");

        // pack 需要进度回调，本测试关注格式与校验链路而非进度，
        // 因此直接复用内部 helper 手工构造备份文件（与 pack 内第 2~5 步一致）。
        let tar_bytes = build_tar(&path.data_directory).unwrap();
        let original_sha = Sha256::digest(&tar_bytes);
        let mut sha_arr = [0u8; 32];
        sha_arr.copy_from_slice(&original_sha);
        let params = compute_shard_params(tar_bytes.len() as u64, 0.5).unwrap();
        let shards = encode_shards(&tar_bytes, params).unwrap();
        let checksum_table = compute_shard_checksums(&shards);
        let header = Header {
            original_size: tar_bytes.len() as u64,
            data_shards: params.data_shards,
            parity_shards: params.parity_shards,
            shard_size: params.shard_size,
            redundancy_ratio: 0.5,
            original_sha256: sha_arr,
        };
        write_backup_file(&backup_path, &header, &shards, &checksum_table).unwrap();

        // 读取 shard 区并人为损坏一个数据 shard（替换前几个字节）。
        // shard 区紧跟 Header + 校验和表，所以起始偏移为 HEADER_SIZE + 校验和表大小。
        let shard_region_start = HEADER_SIZE + header.shard_checksum_table_size();
        let shard_size = header.shard_size as usize;
        let mut file = File::open(&backup_path).unwrap();
        file.seek(std::io::SeekFrom::Start(shard_region_start as u64))
            .unwrap();
        let mut buf = vec![0u8; shard_size];
        file.read_exact(&mut buf).unwrap();
        buf[0] ^= 0xFF;
        buf[1] ^= 0xFF;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .open(&backup_path)
            .unwrap();
        file.seek(std::io::SeekFrom::Start(shard_region_start as u64))
            .unwrap();
        file.write_all(&buf).unwrap();
        drop(file);

        // probe 应报告可恢复。
        let (recoverable, lost, limit) = probe(&backup_path).unwrap();
        assert!(recoverable);
        assert_eq!(lost, 1);
        assert!(limit >= 1);

        // 校验 shard 区域：第 0 块应被标记为 None。
        let mut file = File::open(&backup_path).unwrap();
        file.seek(std::io::SeekFrom::Start(shard_region_start as u64))
            .unwrap();
        let mut shard_bytes = vec![0u8; header.shard_region_size()];
        file.read_exact(&mut shard_bytes).unwrap();
        let mut checksum_table = vec![0u8; header.shard_checksum_table_size()];
        let checksums_start = HEADER_SIZE;
        file.seek(std::io::SeekFrom::Start(checksums_start as u64))
            .unwrap();
        file.read_exact(&mut checksum_table).unwrap();
        drop(file);

        let params_struct = crate::business::backup::codec::ShardParams {
            data_shards: header.data_shards,
            parity_shards: header.parity_shards,
            shard_size: header.shard_size,
        };
        let verified = verify_shard_region(&shard_bytes, params_struct, &checksum_table).unwrap();
        let missing_idx: Vec<usize> = verified
            .iter()
            .enumerate()
            .filter_map(|(i, s)| if s.is_none() { Some(i) } else { None })
            .collect();
        assert_eq!(missing_idx.len(), 1);
        assert_eq!(missing_idx[0], 0);

        // 重建应得到完整 shard。
        let restored = reconstruct_shards(verified, params_struct).unwrap();
        assert_eq!(restored.len(), params_struct.data_shards as usize + params_struct.parity_shards as usize);

        // 拼接数据 shard 应等于原始字节流。
        let mut recovered_tar = Vec::new();
        for chunk in restored.iter().take(header.data_shards as usize) {
            recovered_tar.extend_from_slice(chunk);
        }
        recovered_tar.truncate(header.original_size as usize);
        assert_eq!(recovered_tar, tar_bytes);

        crate::test::cleanup(&path);
    }

    /// 覆盖 data_directory_size：日志目录应被忽略。
    #[test]
    fn test_data_directory_size_excludes_logs() {
        let _guard = crate::test::acquire_test_lock();
        let path = crate::test::create_test_path();
        state::set_path(path.clone());
        std::fs::create_dir_all(&path.user_database_set_directory).unwrap();
        std::fs::write(path.user_database_set_directory.join("x.sqlite"), b"12345").unwrap();
        std::fs::create_dir_all(&path.log_directory).unwrap();
        std::fs::write(path.log_directory.join("today.log"), b"a".repeat(1_000_000)).unwrap();

        let total = data_directory_size(&path.data_directory).unwrap();
        assert_eq!(total, 5);

        crate::test::cleanup(&path);
    }

    /// 覆盖 unpack 完整端到端：pack → 损坏中段 shard → unpack 还原。
    /// 临时目录位于系统 temp，验证解压 → 清空 → move → 清理完整链路。
    #[test]
    fn test_unpack_end_to_end() {
        let _guard = crate::test::acquire_test_lock();
        let path = crate::test::create_test_path();
        state::set_path(path.clone());
        // 初始化 preference 与 metadata 连接，pack 内部持久化步骤需要它们。
        crate::business::preference::service::initialize().unwrap();
        crate::business::metadata::service::initialize().unwrap();

        // 在数据目录里准备若干文件。
        std::fs::create_dir_all(path.user_database_set_directory.join("fake-db"))
            .unwrap();
        std::fs::write(
            path.user_database_set_directory.join("fake-db").join("a.sqlite"),
            b"hello world",
        )
        .unwrap();
        std::fs::write(
            path.user_database_set_directory.join("b.sqlite"),
            b"another database",
        )
        .unwrap();
        // 写一个独立文件用于验证还原（不属于 SQLite 数据库）。
        std::fs::write(path.data_directory.join("extra.txt"), b"plain-text-bytes")
            .unwrap();
        std::fs::create_dir_all(&path.log_directory).unwrap();
        std::fs::write(path.log_directory.join("today.log"), b"should be kept")
            .unwrap();

        // 记录关键内容用于校验还原后保留。
        let extra_bytes = std::fs::read(path.data_directory.join("extra.txt")).unwrap();
        let log_bytes = std::fs::read(path.log_directory.join("today.log")).unwrap();

        // 用收集闭包跑 pack/unpack，顺带验证进度回调契约：阶段边界成对上报（0.0 → 1.0）。
        let progress_log = std::cell::RefCell::new(Vec::new());
        let collect_progress = |phase: Phase, progress: f32| {
            progress_log.borrow_mut().push((phase, progress));
        };

        // 跑 pack（进度回调收集到 progress_log）。
        let backup_path = path.data_directory.join("test.ibackup");
        pack(&backup_path, 0.5, &collect_progress).unwrap();

        // 读取 header 获取参数，损坏 shard 0 的前几字节。
        let mut file = File::open(&backup_path).unwrap();
        let mut header_bytes = [0u8; HEADER_SIZE];
        file.read_exact(&mut header_bytes).unwrap();
        let header = Header::from_bytes(&header_bytes).unwrap();
        drop(file);

        let shard_region_start = HEADER_SIZE + header.shard_checksum_table_size();
        let shard_size = header.shard_size as usize;
        let mut file = File::open(&backup_path).unwrap();
        file.seek(std::io::SeekFrom::Start(shard_region_start as u64))
            .unwrap();
        let mut buf = vec![0u8; shard_size];
        file.read_exact(&mut buf).unwrap();
        buf[0] ^= 0xFF;
        buf[1] ^= 0xFF;
        drop(file);
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .open(&backup_path)
            .unwrap();
        file.seek(std::io::SeekFrom::Start(shard_region_start as u64))
            .unwrap();
        file.write_all(&buf).unwrap();
        drop(file);

        // 跑 unpack：临时目录在系统 temp，整流完成后应被清理。
        unpack(&backup_path, &collect_progress).unwrap();

        // 验证进度回调契约：pack 上报 BackupPack/BackupEncode/BackupWrite，
        // unpack 上报 RestoreReadHeader/RestoreVerify/RestoreDecode/RestoreUnpack/RestoreClear/RestoreMove
        // （本测试损坏了 1 块 shard，因此包含 RestoreDecode），每个阶段均为 (0.0, 1.0) 成对出现。
        let expected_progress: Vec<(Phase, f32)> = [
            Phase::BackupPack,
            Phase::BackupEncode,
            Phase::BackupWrite,
            Phase::RestoreReadHeader,
            Phase::RestoreVerify,
            Phase::RestoreDecode,
            Phase::RestoreUnpack,
            Phase::RestoreClear,
            Phase::RestoreMove,
        ]
        .iter()
        .flat_map(|phase| [(*phase, 0.0), (*phase, 1.0)])
        .collect();
        assert_eq!(*progress_log.borrow(), expected_progress);

        // 验证数据目录里的非 SQLite 内容与原始一致（含子目录）。
        let restored_a = std::fs::read(
            path.user_database_set_directory.join("fake-db").join("a.sqlite"),
        )
        .unwrap();
        assert_eq!(restored_a, b"hello world");
        let restored_b =
            std::fs::read(path.user_database_set_directory.join("b.sqlite")).unwrap();
        assert_eq!(restored_b, b"another database");
        // extra.txt 是不被 SQLite 处理的纯文本，验证字节级一致。
        let restored_extra = std::fs::read(path.data_directory.join("extra.txt")).unwrap();
        assert_eq!(restored_extra, extra_bytes);

        // 验证 logs/ 目录被保留。
        assert!(path.log_directory.join("today.log").exists());
        let restored_log =
            std::fs::read(path.log_directory.join("today.log")).unwrap();
        assert_eq!(restored_log, log_bytes);

        // 验证系统 temp 下没有残留的 inet-restore-* 临时目录。
        let temp_root = std::env::temp_dir();
        let leftover: Vec<_> = std::fs::read_dir(&temp_root)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_name()
                    .to_string_lossy()
                    .starts_with("inet-restore-")
            })
            .collect();
        assert!(
            leftover.is_empty(),
            "leftover temp dirs: {:?}",
            leftover.iter().map(|e| e.path()).collect::<Vec<_>>()
        );

        crate::test::cleanup(&path);
    }

    /// 覆盖 unpack 后 preference 与 metadata 能被重新打开并读到正确内容。
    ///
    /// 模拟用户实际使用场景：先设置 preference 和注册数据库，持久化后打包；
    /// 然后解包，再次打开数据库，验证内容完整可读。
    #[test]
    fn test_unpack_preserves_preference_and_metadata() {
        let _guard = crate::test::acquire_test_lock();
        let path = crate::test::create_test_path();
        state::set_path(path.clone());

        // 初始化 preference 和 metadata 并写入"用户数据"。
        crate::business::preference::service::initialize().unwrap();
        crate::business::metadata::service::initialize().unwrap();
        crate::business::preference::service::set("theme", "dark").unwrap();
        crate::business::preference::service::set("locale", "zh-CN").unwrap();
        let meta =
            crate::business::metadata::service::register("test-db".to_string()).unwrap();
        crate::business::preference::service::save().unwrap();
        crate::business::metadata::service::save().unwrap();

        // Pack：pack 内部已持久化 preference/metadata；本测试不断言进度，用 noop 回调。
        let backup_path = path.data_directory.join("test.ibackup");
        pack(&backup_path, 0.5, &noop_progress).unwrap();

        // 释放所有当前 connection guard，确保后续 initialize 时 RefCell borrow 不冲突。
        drop(crate::business::preference::state::lock_connection());
        drop(crate::business::metadata::state::lock_connection());

        // Unpack。
        unpack(&backup_path, &noop_progress).unwrap();

        // 模拟"应用重启"：重新打开连接。文件已被 unpack 替换为恢复后的版本。
        crate::business::preference::service::initialize().unwrap();
        crate::business::metadata::service::initialize().unwrap();

        // 验证 preference 内容。
        assert_eq!(
            crate::business::preference::service::get("theme").unwrap(),
            Some("dark".to_string())
        );
        assert_eq!(
            crate::business::preference::service::get("locale").unwrap(),
            Some("zh-CN".to_string())
        );

        // 验证 metadata 内容。
        let list = crate::business::metadata::service::list(false).unwrap();
        assert_eq!(list.len(), 1, "metadata list should have 1 entry");
        assert_eq!(list[0].id, meta.id);
        assert_eq!(list[0].name, "test-db");

        crate::test::cleanup(&path);
    }

    /// 覆盖 unpack 在解压失败错误路径下不残留临时目录、且不清空数据目录。
    ///
    /// 手工构造一个内容非合法 tar 流（32 字节，远不足 512 字节的 tar 头块大小，
    /// tar::Archive::unpack 必报错）的备份文件，验证：
    /// 1. unpack 返回 `FailToUnpackBackup`；
    /// 2. 系统 temp 目录下没有 `inet-restore-*` 残留目录；
    /// 3. 数据目录里的内容保持原样（extract 在 clear 之前，失败时数据目录未被动）。
    /// 不需要调用 pack，因此不初始化 preference/metadata 的内存 connection。
    #[test]
    fn test_unpack_failure_cleans_up_temp_directory() {
        let _guard = crate::test::acquire_test_lock();
        let path = crate::test::create_test_path();
        state::set_path(path.clone());

        // 数据目录里放一个见证文件，验证失败路径未被动过。
        let extra_path = path.data_directory.join("extra.txt");
        std::fs::write(&extra_path, b"witness-bytes").unwrap();

        // 手工构造备份文件：32 字节非合法 tar 流。
        let payload: &[u8] = b"this is not a valid tar stream!!";
        let mut sha_arr = [0u8; 32];
        let original_sha = Sha256::digest(payload);
        sha_arr.copy_from_slice(&original_sha);
        let params = compute_shard_params(payload.len() as u64, 0.5).unwrap();
        let shards = encode_shards(payload, params).unwrap();
        let checksums = compute_shard_checksums(&shards);
        let header = Header {
            original_size: payload.len() as u64,
            data_shards: params.data_shards,
            parity_shards: params.parity_shards,
            shard_size: params.shard_size,
            redundancy_ratio: 0.5,
            original_sha256: sha_arr,
        };
        let backup_path = path.data_directory.join("bad.ibackup");
        write_backup_file(&backup_path, &header, &shards, &checksums).unwrap();

        // 解压必须失败（payload 太短，tar 头块不完整）。
        let result = unpack(&backup_path, &noop_progress);
        assert!(
            matches!(result, Err(ErrorCode::FailToUnpackBackup { .. })),
            "expected FailToUnpackBackup, got {:?}",
            result
        );

        // 临时目录必须被 RAII 守卫清理。
        let temp_root = std::env::temp_dir();
        let leftover: Vec<_> = std::fs::read_dir(&temp_root)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_name()
                    .to_string_lossy()
                    .starts_with("inet-restore-")
            })
            .collect();
        assert!(
            leftover.is_empty(),
            "leftover temp dirs: {:?}",
            leftover.iter().map(|e| e.path()).collect::<Vec<_>>()
        );

        // 数据目录里的见证文件未被破坏（extract 在 clear 之前失败，未走到 clear）。
        let preserved = std::fs::read(&extra_path).unwrap();
        assert_eq!(preserved, b"witness-bytes");

        crate::test::cleanup(&path);
    }

    /// 覆盖尾部截断在 parity 冗余容量内的成功路径：
    /// 从尾部截断恰好 `parity_shards * shard_size` 字节（校验和表前置不受影响，
    /// 仅丢失全部校验 shard，数据 shard 完好），probe 应报告可恢复且丢失数等于 parity，
    /// unpack 端到端还原后数据逐字节一致、logs/ 被保留。
    #[test]
    fn test_unpack_tolerates_tail_truncation_within_parity() {
        let _guard = crate::test::acquire_test_lock();
        let path = crate::test::create_test_path();
        state::set_path(path.clone());
        // 初始化 preference 与 metadata 连接，pack 内部持久化步骤需要它们。
        crate::business::preference::service::initialize().unwrap();
        crate::business::metadata::service::initialize().unwrap();

        // 在数据目录里准备若干文件（含一个日志文件用于验证 logs/ 保留）。
        std::fs::create_dir_all(path.user_database_set_directory.join("fake-db")).unwrap();
        std::fs::write(
            path.user_database_set_directory.join("fake-db").join("a.sqlite"),
            b"hello world",
        )
        .unwrap();
        std::fs::write(path.data_directory.join("extra.txt"), b"plain-text-bytes").unwrap();
        std::fs::create_dir_all(&path.log_directory).unwrap();
        std::fs::write(path.log_directory.join("today.log"), b"should be kept").unwrap();

        let backup_path = path.data_directory.join("test.ibackup");
        pack(&backup_path, 0.5, &noop_progress).unwrap();

        // 读取 header，从尾部截断 parity_shards * shard_size 字节。
        let mut file = File::open(&backup_path).unwrap();
        let mut header_bytes = [0u8; HEADER_SIZE];
        file.read_exact(&mut header_bytes).unwrap();
        let header = Header::from_bytes(&header_bytes).unwrap();
        let truncated_len = file.metadata().unwrap().len()
            - header.parity_shards as u64 * header.shard_size as u64;
        drop(file);
        let file = std::fs::OpenOptions::new()
            .write(true)
            .open(&backup_path)
            .unwrap();
        file.set_len(truncated_len).unwrap();
        drop(file);

        // probe 应报告可恢复，丢失数恰好等于 parity（校验和表完整，仅尾部校验 shard 缺失）。
        let (recoverable, lost, limit) = probe(&backup_path).unwrap();
        assert!(recoverable);
        assert_eq!(lost, header.parity_shards as usize);
        assert_eq!(limit, header.parity_shards as usize);

        // unpack 应成功，数据目录内容逐字节还原。
        unpack(&backup_path, &noop_progress).unwrap();
        let restored_a = std::fs::read(
            path.user_database_set_directory.join("fake-db").join("a.sqlite"),
        )
        .unwrap();
        assert_eq!(restored_a, b"hello world");
        let restored_extra = std::fs::read(path.data_directory.join("extra.txt")).unwrap();
        assert_eq!(restored_extra, b"plain-text-bytes");
        let restored_log = std::fs::read(path.log_directory.join("today.log")).unwrap();
        assert_eq!(restored_log, b"should be kept");

        crate::test::cleanup(&path);
    }

    /// 覆盖尾部截断超出 parity 冗余容量的失败路径：
    /// 截断 `(parity_shards + 1) * shard_size` 字节（丢失全部校验 shard + 1 个数据 shard），
    /// probe 应报告不可恢复，unpack 应返回 `BackupTooManyShardsLost`，
    /// 且 decode 失败发生在 clear 之前，数据目录保持原样。
    #[test]
    fn test_unpack_rejects_tail_truncation_beyond_parity() {
        let _guard = crate::test::acquire_test_lock();
        let path = crate::test::create_test_path();
        state::set_path(path.clone());
        crate::business::preference::service::initialize().unwrap();
        crate::business::metadata::service::initialize().unwrap();

        // 放一个见证文件，验证失败路径未被动过。
        std::fs::create_dir_all(&path.user_database_set_directory).unwrap();
        std::fs::write(
            path.user_database_set_directory.join("witness.sqlite"),
            b"witness-bytes",
        )
        .unwrap();

        let backup_path = path.data_directory.join("test.ibackup");
        pack(&backup_path, 0.5, &noop_progress).unwrap();

        // 从尾部截断 (parity_shards + 1) * shard_size 字节。
        let mut file = File::open(&backup_path).unwrap();
        let mut header_bytes = [0u8; HEADER_SIZE];
        file.read_exact(&mut header_bytes).unwrap();
        let header = Header::from_bytes(&header_bytes).unwrap();
        let truncated_len = file.metadata().unwrap().len()
            - (header.parity_shards as u64 + 1) * header.shard_size as u64;
        drop(file);
        let file = std::fs::OpenOptions::new()
            .write(true)
            .open(&backup_path)
            .unwrap();
        file.set_len(truncated_len).unwrap();
        drop(file);

        // probe 应报告不可恢复（丢失数 = parity + 1 > 上限）。
        let (recoverable, lost, limit) = probe(&backup_path).unwrap();
        assert!(!recoverable);
        assert_eq!(lost, header.parity_shards as usize + 1);
        assert_eq!(limit, header.parity_shards as usize);

        // unpack 应返回 BackupTooManyShardsLost。
        let result = unpack(&backup_path, &noop_progress);
        assert!(
            matches!(result, Err(ErrorCode::BackupTooManyShardsLost { .. })),
            "expected BackupTooManyShardsLost, got {:?}",
            result
        );

        // 数据目录里的见证文件未被破坏（decode 在 clear 之前失败）。
        let witness =
            std::fs::read(path.user_database_set_directory.join("witness.sqlite")).unwrap();
        assert_eq!(witness, b"witness-bytes");

        crate::test::cleanup(&path);
    }

    /// 覆盖截断严重到侵入校验和表的失败路径：
    /// 校验和表前置后，先于 shard 区受损意味着 shard 数据已全丢、无恢复价值，
    /// probe 应直接返回 `InvalidBackupFile` 错误。
    #[test]
    fn test_probe_rejects_truncation_into_checksum_table() {
        let _guard = crate::test::acquire_test_lock();
        let path = crate::test::create_test_path();
        state::set_path(path.clone());
        crate::business::preference::service::initialize().unwrap();
        crate::business::metadata::service::initialize().unwrap();

        std::fs::create_dir_all(&path.user_database_set_directory).unwrap();
        std::fs::write(
            path.user_database_set_directory.join("a.sqlite"),
            b"hello world",
        )
        .unwrap();

        let backup_path = path.data_directory.join("test.ibackup");
        pack(&backup_path, 0.5, &noop_progress).unwrap();

        // 截断到 Header 之后再留 4 字节，校验和表不完整。
        let file = std::fs::OpenOptions::new()
            .write(true)
            .open(&backup_path)
            .unwrap();
        file.set_len(HEADER_SIZE as u64 + 4).unwrap();
        drop(file);

        let result = probe(&backup_path);
        assert!(
            matches!(result, Err(ErrorCode::InvalidBackupFile { .. })),
            "expected InvalidBackupFile, got {:?}",
            result
        );

        crate::test::cleanup(&path);
    }
}