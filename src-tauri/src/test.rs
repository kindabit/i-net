use parking_lot::{ReentrantMutex, ReentrantMutexGuard};

use crate::state;

/// 全局测试锁，用于串行化所有依赖全局状态（路径状态、数据库连接状态）的测试，
/// 避免并行测试之间互相干扰。
static TEST_MUTEX: ReentrantMutex<()> = ReentrantMutex::new(());

/// 获取全局测试锁的守卫，守卫存活期间当前测试独占全局状态。
pub fn acquire_test_lock() -> ReentrantMutexGuard<'static, ()> {
    TEST_MUTEX.lock()
}

/// 构造测试专用的 [`state::Path`]，所有路径均位于项目 `target` 目录下，
/// 确保测试产生的文件不会离开项目文件夹。
pub fn create_test_path() -> state::Path {
    let data_directory = std::env::var("CARGO_MANIFEST_DIR")
        .map(std::path::PathBuf::from)
        .expect("CARGO_MANIFEST_DIR is not set")
        .join("target")
        .join("test-i-net-data")
        .join(uuid::Uuid::new_v4().to_string());
    crate::initialize_data_directory(Some(data_directory))
}

/// 清理测试数据目录。
pub fn cleanup(path: &state::Path) {
    let _ = std::fs::remove_dir_all(&path.data_directory);
}

/// 返回一个固定的 32 字节测试密钥。
pub fn test_key() -> [u8; 32] {
    [1u8; 32]
}
