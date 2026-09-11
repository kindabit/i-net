use crate::business::user_database::canvas::dao;
use crate::business::user_database::canvas::vo::MoveNodeVO;
use crate::business::user_database::entity::{Action, Canvas};
use crate::business::user_database::{log, state};
use crate::error_code::ErrorCode;

/// 批量移动画布坐标。若所有条目均未发生位移，则不更新数据库、不产生日志。
///
/// 实际位移的画布数量为 1 时产生一条 CanvasMove 日志，载荷为画布名称、旧坐标和新坐标；
/// 大于 1 时产生一条 AutoLayoutCanvasNodes 日志，canvas_count 为实际位移的画布数量。
///
/// # 参数
/// - `items`: 要移动的画布列表。
///
/// # 返回值
/// 成功时返回 `Ok(())`；任一画布不存在时返回 `ErrorCode::NoCanvasWithSuchId`，
/// 发生其他错误时返回对应的 `ErrorCode`。
pub fn move_canvases(items: &[MoveNodeVO]) -> Result<(), ErrorCode> {
    let connection = state::lock_connection();
    // 逐个校验存在性，同时记录画布实体（含旧坐标；单条位移分支产日志时还需画布名称）。
    let mut canvases: Vec<Canvas> = Vec::new();
    for item in items {
        let canvas = dao::select_by_id(&connection, &item.id)?
            .ok_or_else(|| ErrorCode::NoCanvasWithSuchId {
                id: item.id.clone(),
            })?;
        canvases.push(canvas);
    }
    // 筛出实际发生位移的条目（连同其画布实体）。
    let moved: Vec<(&MoveNodeVO, &Canvas)> = items
        .iter()
        .zip(canvases.iter())
        .filter(|(item, canvas)| item.x != canvas.x || item.y != canvas.y)
        .collect();
    // 若筛后为空：不更新数据库、不产日志，返回 Ok。
    if moved.is_empty() {
        return Ok(());
    }
    // 一次性更新。
    let moved_coords: Vec<(String, f64, f64)> = moved
        .iter()
        .map(|(item, _)| (item.id.clone(), item.x, item.y))
        .collect();
    dao::batch_move(&connection, &moved_coords)?;
    // 按实际位移数量产生一条日志。
    if moved.len() == 1 {
        let (item, canvas) = moved[0];
        log::service::create(Action::CanvasMove {
            name: canvas.name.clone(),
            old_x: canvas.x,
            old_y: canvas.y,
            new_x: item.x,
            new_y: item.y,
        })?;
    } else {
        let canvas_count = moved.len() as i64;
        log::service::create(Action::AutoLayoutCanvasNodes { canvas_count })?;
    }
    Ok(())
}
