use crate::business::user_database::node::dao;
use crate::business::user_database::node::response::DataNodeColorEntry;
use crate::business::user_database::state;
use crate::error_code::ErrorCode;

/// 查询所有未删除且设置了颜色的节点的标题与颜色。
///
/// # 返回值
/// 返回数据节点颜色条目列表；若发生错误则返回对应的 `ErrorCode`。
pub fn color_list() -> Result<Vec<DataNodeColorEntry>, ErrorCode> {
    let connection = state::lock_connection();
    dao::select_colored(&connection)
}
