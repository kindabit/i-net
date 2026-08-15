use crate::business::user_database::canvas::dao;
use crate::business::user_database::canvas::vo::MoveNodeVO;
use crate::business::user_database::entity::Action;
use crate::business::user_database::{log, state};
use crate::error_code::ErrorCode;

/// 批量移动画布坐标。若所有条目均未发生位移，则不更新数据库、不产生日志。
///
/// 产生一条 AutoLayoutCanvasNodes 日志，object_id 为根画布 id，
/// canvas_count 为实际位移的画布数量。
///
/// # 参数
/// - `items`: 要移动的画布列表。
///
/// # 返回值
/// 成功时返回 `Ok(())`；任一画布不存在时返回 `ErrorCode::NoCanvasWithSuchId`，
/// 根画布不存在时返回 `ErrorCode::DatabaseError`，
/// 发生其他错误时返回对应的 `ErrorCode`。
pub fn move_canvases(items: &[MoveNodeVO]) -> Result<(), ErrorCode> {
    let connection = state::lock_connection();
    // 逐个校验存在性，同时记录旧坐标。
    let mut old_coords: Vec<(String, f64, f64)> = Vec::new();
    for item in items {
        let canvas = dao::select_by_id(&connection, &item.id)?
            .ok_or_else(|| ErrorCode::NoCanvasWithSuchId { id: item.id.clone() })?;
        old_coords.push((item.id.clone(), canvas.x, canvas.y));
    }
    // 筛出实际发生位移的条目。
    let moved: Vec<(String, f64, f64)> = items
        .iter()
        .zip(old_coords.iter())
        .filter(|(item, (_, old_x, old_y))| item.x != *old_x || item.y != *old_y)
        .map(|(item, _)| (item.id.clone(), item.x, item.y))
        .collect();
    // 若筛后为空：不更新数据库、不产日志，返回 Ok。
    if moved.is_empty() {
        return Ok(());
    }
    // 一次性更新。
    dao::batch_move(&connection, &moved)?;
    // 产生一条日志：object_id 取根画布 id。
    let root = dao::select_root(&connection)?
        .ok_or_else(|| ErrorCode::DatabaseError {
            detail: "root canvas not found".to_string(),
        })?;
    let canvas_count = moved.len() as i64;
    log::service::create(
        &root.id,
        Action::AutoLayoutCanvasNodes { canvas_count },
    )?;
    Ok(())
}
