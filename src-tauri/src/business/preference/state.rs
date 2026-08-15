use parking_lot::{ReentrantMutex, ReentrantMutexGuard};
use std::cell::{Ref, RefCell};
use std::ops::Deref;
use std::sync::LazyLock;

use rusqlite::Connection;

/// 全局静态变量，存储内含 preference 数据库的 sqlite connection。
///
/// 程序启动时由 service 层 initialize 函数写入。
/// 使用可重入锁，同一线程可多次获取而不会死锁；由于可重入锁守卫不提供可变引用，
/// 内部用 `RefCell` 支持写入。
static CONNECTION: LazyLock<ReentrantMutex<RefCell<Option<Connection>>>> =
    LazyLock::new(|| ReentrantMutex::new(RefCell::new(None)));

/// 将 preference 数据库连接写入全局状态。
///
/// # 参数
/// - `connection`: preference 数据库连接。
///
/// # 返回值
/// 无。
pub fn set_connection(connection: Connection) {
    *CONNECTION.lock().borrow_mut() = Some(connection);
}

/// preference 数据库连接的锁守卫，解引用后直接得到 `&Connection`。
///
/// 守卫只会在 [`set_connection`] 之后被获取，因此解引用时直接 expect。
/// 该锁为可重入锁，同一线程可多次获取而不会死锁，因此 service 函数调用其它 service 函数前无需先释放守卫。
/// 守卫同时持有可重入锁守卫和 `RefCell` 的共享借用 `Ref`：只要守卫存活，
/// 同线程再对连接执行写入（如 [`set_connection`]）会触发借用冲突而 panic，
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
            .expect("preference database connection is not initialized")
    }
}

/// 获取 preference 数据库连接的锁守卫。
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
