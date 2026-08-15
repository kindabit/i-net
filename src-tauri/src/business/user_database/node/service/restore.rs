use crate::business::user_database::entity::Action;
use crate::business::user_database::node::dao;
use crate::business::user_database::{canvas, log, state};
use crate::error_code::ErrorCode;

/// 恢复被逻辑删除的节点：清空该节点的逻辑删除状态，并将节点移动至新坐标。
/// 如果是画布节点，使用该节点引用的画布在库内的坐标恢复该画布。
///
/// 产生 NodeRestore 日志，载荷为节点的标题、旧坐标和新坐标。
///
/// # 参数
/// - `id`: 节点 id。
/// - `x`: 新 x 坐标。
/// - `y`: 新 y 坐标。
///
/// # 返回值
/// 成功时返回 `Ok(())`；节点不存在时返回 `ErrorCode::NoNodeWithSuchId`，
/// 发生其他错误时返回对应的 `ErrorCode`。
pub fn restore(id: &str, x: f64, y: f64) -> Result<(), ErrorCode> {
    let connection = state::lock_connection();
    let mut node = dao::select_by_id(&connection, id)?
        .ok_or_else(|| ErrorCode::NoNodeWithSuchId { id: id.to_string() })?;
    let old_x = node.x;
    let old_y = node.y;
    node.deleted = false;
    node.x = x;
    node.y = y;
    dao::update(&connection, &node)?;
    let canvas_ref_id = node.canvas_ref_id.clone();
    log::service::create(
        id,
        Action::NodeRestore {
            title: node.title,
            old_x,
            old_y,
            new_x: x,
            new_y: y,
        },
    )?;
    // 级联恢复引用的子画布（使用库内坐标）
    if let Some(ref_id) = canvas_ref_id {
        let canvas = canvas::dao::select_by_id(&connection, &ref_id)?;
        let (cx, cy) = if let Some(c) = canvas {
            (c.x, c.y)
        } else {
            (0.0, 0.0)
        };
        canvas::service::restore(&ref_id, cx, cy)?;
    }
    Ok(())
}
