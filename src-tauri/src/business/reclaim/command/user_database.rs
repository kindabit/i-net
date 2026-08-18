//! `reclaim_user_database` 命令：重新同步 user_database 状态。
//!
//! 正常 Home.vue 路径下不会有 user_database 处于打开状态；
//! 若开着则强制关闭，避免陈旧 in-memory 在后续 save 时覆盖还原结果。

use crate::business::user_database;
use crate::error_code::ErrorCode;

/// 重新同步 user_database 状态。
/// 正常 Home.vue 路径下不会有 user_database 处于打开状态；
/// 若开着则强制关闭，避免陈旧 in-memory 在后续 save 时覆盖还原结果。
#[tauri::command]
pub fn reclaim_user_database() -> Result<(), ErrorCode> {
    tracing::info!("[RECLAIM] user_database: re-syncing state");
    if user_database::state::is_open() {
        tracing::info!("[RECLAIM] user_database: closing open database (defensive)");
        user_database::lifecycle::service::close()?;
    }
    Ok(())
}
