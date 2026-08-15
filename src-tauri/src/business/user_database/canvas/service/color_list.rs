use crate::business::user_database::canvas::dao;
use crate::business::user_database::canvas::response::CanvasColorEntry;
use crate::business::user_database::state;
use crate::error_code::ErrorCode;

/// 查询所有未删除且设置了颜色的画布的名称、父画布 id 与颜色。
///
/// # 返回值
/// 返回画布颜色条目列表；若发生错误则返回对应的 `ErrorCode`。
pub fn color_list() -> Result<Vec<CanvasColorEntry>, ErrorCode> {
    let connection = state::lock_connection();
    dao::select_colored(&connection)
}
