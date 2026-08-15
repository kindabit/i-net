use crate::business::user_database::canvas::service;
use crate::business::user_database::entity::Canvas;
use crate::error_code::ErrorCode;
use crate::util::preprocess_util;

/// 在指定父画布下新建一个子画布。
///
/// # 参数
/// - `parent_id`: 父画布 id。
/// - `name`: 新画布的名称。
///
/// # 返回值
/// 返回新建的画布；若发生错误则返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_canvas_create(parent_id: String, name: String) -> Result<Canvas, ErrorCode> {
    preprocess(parent_id, name)
}

/// `user_database_canvas_create` 的 preprocess 函数：校验参数后接入 service 层的 create 函数。
pub fn preprocess(parent_id: String, name: String) -> Result<Canvas, ErrorCode> {
    let parent_id = preprocess_util::preprocess_canvas_id(parent_id)?;
    let name = preprocess_util::preprocess_canvas_name(name)?;
    service::create(&parent_id, name)
}
