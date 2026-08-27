use crate::business::user_database::node::service;
use crate::error_code::ErrorCode;
use crate::util::preprocess_util;

/// 设置指定节点的颜色。
///
/// # 参数
/// - `id`: 节点 id。
/// - `color`: 前端序列化的自定义颜色，空串表示使用默认色。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_node_set_color(id: String, color: String) -> Result<(), ErrorCode> {
    preprocess(id, color)
}

/// `user_database_node_set_color` 的 preprocess 函数：校验参数后接入 service 层的 set_color 函数。
///
/// 颜色仅裁剪首尾空白字符，不做格式校验。
pub fn preprocess(id: String, color: String) -> Result<(), ErrorCode> {
    let id = preprocess_util::preprocess_node_id(id)?;
    service::set_color(&id, color.trim().to_string())
}
