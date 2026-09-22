use crate::business::user_database::node_tag::dao;
use crate::business::user_database::node_tag::vo::NodeTagVO;
use crate::business::user_database::state;
use crate::error_code::ErrorCode;

/// 列出全部标签及其关联的未删除数据节点数量，按数量降序、标签名称升序。不产生日志。
///
/// # 返回值
/// 返回标签值对象列表；若发生错误则返回对应的 `ErrorCode`。
pub fn list() -> Result<Vec<NodeTagVO>, ErrorCode> {
    let connection = state::lock_connection();
    let rows = dao::select_all_with_count(&connection)?;
    Ok(rows
        .into_iter()
        .map(|(name, node_count)| NodeTagVO { name, node_count })
        .collect())
}
