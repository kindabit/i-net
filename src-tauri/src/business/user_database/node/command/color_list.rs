use crate::business::user_database::node::response::DataNodeColorEntry;
use crate::business::user_database::node::service;
use crate::error_code::ErrorCode;

/// 查询所有未删除且设置了颜色的节点的标题与颜色。
///
/// # 返回值
/// 返回数据节点颜色条目列表；若发生错误则返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_data_node_color_list() -> Result<Vec<DataNodeColorEntry>, ErrorCode> {
    preprocess()
}

/// `user_database_data_node_color_list` 的 preprocess 函数：无参数，直接接入 service 层的 color_list 函数。
pub fn preprocess() -> Result<Vec<DataNodeColorEntry>, ErrorCode> {
    service::color_list()
}
