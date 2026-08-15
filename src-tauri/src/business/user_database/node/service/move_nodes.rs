use crate::business::user_database::entity::Action;
use crate::business::user_database::node::dao;
use crate::business::user_database::node::vo::MoveNodeVO;
use crate::business::user_database::{log, state};
use crate::error_code::ErrorCode;

/// 批量移动节点坐标。若所有条目均未发生位移，则不更新数据库、不产生日志。
///
/// 产生一条 AutoLayoutDataNodes 日志，object_id 为第一个实际位移节点所属的画布 id，
/// node_count 为实际位移的节点数量。
///
/// # 参数
/// - `items`: 要移动的节点列表。
///
/// # 返回值
/// 成功时返回 `Ok(())`；任一节点不存在时返回 `ErrorCode::NoNodeWithSuchId`，
/// 发生其他错误时返回对应的 `ErrorCode`。
pub fn move_nodes(items: &[MoveNodeVO]) -> Result<(), ErrorCode> {
    let connection = state::lock_connection();
    // 逐个校验存在性，同时记录旧坐标。
    let mut old_coords: Vec<(String, f64, f64, String)> = Vec::new();
    for item in items {
        let node = dao::select_by_id(&connection, &item.id)?
            .ok_or_else(|| ErrorCode::NoNodeWithSuchId { id: item.id.clone() })?;
        old_coords.push((item.id.clone(), node.x, node.y, node.canvas_id.clone()));
    }
    // 筛出实际发生位移的条目。
    let moved: Vec<(String, f64, f64)> = items
        .iter()
        .zip(old_coords.iter())
        .filter(|(item, (_, old_x, old_y, _))| item.x != *old_x || item.y != *old_y)
        .map(|(item, _)| (item.id.clone(), item.x, item.y))
        .collect();
    // 若筛后为空：不更新数据库、不产日志，返回 Ok。
    if moved.is_empty() {
        return Ok(());
    }
    // 一次性更新。
    dao::batch_move(&connection, &moved)?;
    // 产生一条日志：object_id 取第一个实际位移节点的画布 id。
    let first_moved_id = &moved[0].0;
    let canvas_id = old_coords
        .iter()
        .find(|(id, _, _, _)| id == first_moved_id)
        .map(|(_, _, _, canvas_id)| canvas_id.clone())
        .unwrap_or_default();
    let node_count = moved.len() as i64;
    log::service::create(
        &canvas_id,
        Action::AutoLayoutDataNodes { node_count },
    )?;
    Ok(())
}
