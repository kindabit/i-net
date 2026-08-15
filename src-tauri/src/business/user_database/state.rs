use parking_lot::{ReentrantMutex, ReentrantMutexGuard};
use std::cell::{Ref, RefCell};
use std::ops::Deref;
use std::sync::{LazyLock, Mutex};

use rusqlite::Connection;

use crate::business::metadata::entity::Metadata;

/// 全局静态变量，存储内含用户数据库的 sqlite connection。
///
/// 用户数据库打开（initialize）时由 service 层写入，关闭（close）时清空。
/// 使用可重入锁，同一线程可多次获取而不会死锁；由于可重入锁守卫不提供可变引用，
/// 内部用 `RefCell` 支持写入。
static CONNECTION: LazyLock<ReentrantMutex<RefCell<Option<Connection>>>> =
    LazyLock::new(|| ReentrantMutex::new(RefCell::new(None)));

/// 全局静态变量，存储已打开的用户数据库的元信息。
///
/// 用户数据库打开（initialize）时由 service 层写入，关闭（close）时清空。
static METADATA: LazyLock<Mutex<Option<Metadata>>> = LazyLock::new(|| Mutex::new(None));

/// 全局静态变量，存储已打开的用户数据库的密码哈希得到的密钥。
///
/// 用户数据库打开（initialize）时由 service 层写入，关闭（close）时清空。
static KEY: LazyLock<Mutex<Option<[u8; 32]>>> = LazyLock::new(|| Mutex::new(None));

/// 将用户数据库连接写入全局状态。
///
/// # 参数
/// - `connection`: 用户数据库连接。
///
/// # 返回值
/// 无。
pub fn set_connection(connection: Connection) {
    *CONNECTION.lock().borrow_mut() = Some(connection);
}

/// 用户数据库连接的锁守卫，解引用后直接得到 `&Connection`。
///
/// 守卫只会在 [`set_connection`] 之后被获取，因此解引用时直接 expect。
/// 该锁为可重入锁，同一线程可多次获取而不会死锁，因此 service 函数调用其它 service 函数前无需先释放守卫。
/// 守卫同时持有可重入锁守卫和 `RefCell` 的共享借用 `Ref`：只要守卫存活，
/// 同线程再对连接执行写入（如 [`set_connection`]、[`clear`]）会触发借用冲突而 panic，
/// 避免读取方持有已被丢弃的连接引用。
pub struct ConnectionGuard {
    connection: Ref<'static, Option<Connection>>,
    _lock: ReentrantMutexGuard<'static, RefCell<Option<Connection>>>,
}

impl Deref for ConnectionGuard {
    type Target = Connection;

    fn deref(&self) -> &Connection {
        self.connection
            .as_ref()
            .expect("user database connection is not initialized")
    }
}

/// 获取用户数据库连接的锁守卫。
///
/// 该锁为可重入锁，同一线程重复获取不会死锁。
///
/// # 返回值
/// 返回连接的锁守卫。
pub fn lock_connection() -> ConnectionGuard {
    let lock = CONNECTION.lock();
    // SAFETY: `RefCell` 位于 `'static` 的 `LazyLock` 中，地址与生命周期固定；
    // 可重入锁守卫在存活期间保证其它线程无法访问 `RefCell`，同线程的再次访问由
    // `RefCell` 的借用检查约束。因此将 `RefCell` 的借用延长为 `'static` 是安全的。
    let connection = unsafe {
        let cell: &'static RefCell<Option<Connection>> =
            &*(&*lock as *const RefCell<Option<Connection>>);
        cell.borrow()
    };
    ConnectionGuard {
        connection,
        _lock: lock,
    }
}

/// 将已打开的用户数据库的元信息写入全局状态。
///
/// # 参数
/// - `metadata`: 用户数据库元信息。
///
/// # 返回值
/// 无。
pub fn set_metadata(metadata: Metadata) {
    *METADATA
        .lock()
        .expect("user database metadata lock is poisoned") = Some(metadata);
}

/// 获取已打开的用户数据库的元信息的克隆。
///
/// # 返回值
/// 返回用户数据库元信息；若尚未初始化则 panic。
pub fn metadata() -> Metadata {
    METADATA
        .lock()
        .expect("user database metadata lock is poisoned")
        .clone()
        .expect("user database metadata is not initialized")
}

/// 将已打开的用户数据库的密钥写入全局状态。
///
/// # 参数
/// - `key`: 32 字节的密钥。
///
/// # 返回值
/// 无。
pub fn set_key(key: [u8; 32]) {
    *KEY.lock().expect("user database key lock is poisoned") = Some(key);
}

/// 获取已打开的用户数据库的密钥。
///
/// # 返回值
/// 返回 32 字节的密钥；若尚未初始化则 panic。
pub fn key() -> [u8; 32] {
    KEY.lock()
        .expect("user database key lock is poisoned")
        .expect("user database key is not initialized")
}

/// 清空全局状态中的连接、元信息和密钥。
///
/// 用户数据库关闭（close）时由 service 层调用。
///
/// # 返回值
/// 无。
pub fn clear() {
    *CONNECTION.lock().borrow_mut() = None;
    *METADATA
        .lock()
        .expect("user database metadata lock is poisoned") = None;
    *KEY.lock().expect("user database key lock is poisoned") = None;
}

/// 判断当前是否有已打开的用户数据库。
///
/// # 返回值
/// 返回用户数据库连接是否存在的布尔值。
pub fn is_open() -> bool {
    CONNECTION.lock().borrow().is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 覆盖全局连接状态的锁可重入性与守卫 Deref 语义。
    ///
    /// 断言 1：同一线程连续两次调用 `lock_connection()` 不会死锁（若锁不可重入，
    /// 第二次调用会永久阻塞导致测试超时失败，因此无需额外构造失败断言）。
    /// 断言 2：两个守卫同时存活时，都能正常 Deref 读取到同一个连接。
    #[test]
    fn test_lock_connection_reentrant_and_shared_guard() {
        // 串行化依赖全局状态的测试，独占全局连接状态。
        let _guard = crate::test::acquire_test_lock();

        // 写入一个内存连接，保证守卫解引用时不会因未初始化而 panic。
        let connection = rusqlite::Connection::open_in_memory()
            .expect("fail to open in-memory sqlite connection for test");
        set_connection(connection);

        // 断言 1：同一线程连续两次获取守卫不会死锁。
        let guard_a = lock_connection();
        let guard_b = lock_connection();

        // 断言 2：两个守卫都存活时都能 Deref 到连接，且指向同一个连接。
        let _: &Connection = &*guard_a;
        let _: &Connection = &*guard_b;
        assert!(std::ptr::eq(&*guard_a, &*guard_b));

        // 先释放守卫，再复原全局状态，避免守卫存活期间写入触发借用冲突。
        drop(guard_a);
        drop(guard_b);
        clear();
    }
}
