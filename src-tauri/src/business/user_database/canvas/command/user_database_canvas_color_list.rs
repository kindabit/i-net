use crate::business::user_database::canvas::response::CanvasColorEntry;
use crate::business::user_database::canvas::service;
use crate::error_code::ErrorCode;

/// 查询所有未删除且设置了颜色的画布的名称、父画布 id 与颜色。
///
/// # 返回值
/// 返回画布颜色条目列表；若发生错误则返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_canvas_color_list() -> Result<Vec<CanvasColorEntry>, ErrorCode> {
    preprocess()
}

/// `user_database_canvas_color_list` 的 preprocess 函数：无参数，直接接入 service 层的 color_list 函数。
pub fn preprocess() -> Result<Vec<CanvasColorEntry>, ErrorCode> {
    service::color_list()
}
