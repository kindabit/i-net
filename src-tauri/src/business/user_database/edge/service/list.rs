use crate::business::user_database::edge::dao;
use crate::business::user_database::entity::Edge;
use crate::business::user_database::state;
use crate::error_code::ErrorCode;

/// 返回指定画布内的所有边。不产生日志。
///
/// # 参数
/// - `canvas_id`: 画布 id。
///
/// # 返回值
/// 返回该画布内的边列表；若发生错误则返回对应的 `ErrorCode`。
pub fn list(canvas_id: &str) -> Result<Vec<Edge>, ErrorCode> {
    let connection = state::lock_connection();
    dao::select_by_canvas_id(&connection, canvas_id)
}
