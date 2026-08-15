use super::collect_ancestor_ids;
use crate::business::user_database::canvas::dao;
use crate::business::user_database::entity::Action;
use crate::business::user_database::{log, node, state};
use crate::error_code::ErrorCode;

/// 恢复被逻辑删除的画布：恢复该画布以及它祖先链上所有被逻辑删除的画布，
/// 并将该画布的坐标修改为新坐标；同时计算该画布新旧坐标之差，
/// 使用该差值计算其它被恢复的祖先画布的新坐标（其它一起被恢复的画布的位置会跟着这个画布走）；
/// 同时恢复所有引用这些画布的画布节点。
///
/// 每个被恢复的画布产生一条 CanvasRestore 日志，载荷内记录画布名称、旧坐标和新坐标。
/// 每个被恢复的画布节点产生一条 NodeRestore 日志（坐标保持节点库存坐标不变）。
///
/// # 参数
/// - `id`: 画布 id。
/// - `x`: 新 x 坐标。
/// - `y`: 新 y 坐标。
///
/// # 返回值
/// 成功时返回 `Ok(())`；画布不存在时返回 `ErrorCode::NoCanvasWithSuchId`，
/// 发生其他错误时返回对应的 `ErrorCode`。
pub fn restore(id: &str, x: f64, y: f64) -> Result<(), ErrorCode> {
    let connection = state::lock_connection();
    let all = dao::select_all(&connection)?;
    let target = all
        .iter()
        .find(|canvas| canvas.id == id)
        .ok_or_else(|| ErrorCode::NoCanvasWithSuchId { id: id.to_string() })?;
    // 目标画布未被逻辑删除时无需恢复，直接视为成功。
    if !target.deleted {
        return Ok(());
    }
    let dx = x - target.x;
    let dy = y - target.y;
    // 需要恢复的对象：目标画布自身（移动到新坐标），
    // 以及祖先链上所有被逻辑删除的画布（跟随目标画布的位移移动）。
    let mut to_restore = vec![(target.clone(), x, y)];
    for ancestor_id in collect_ancestor_ids(&all, id) {
        if let Some(ancestor) = all.iter().find(|canvas| canvas.id == ancestor_id) {
            if ancestor.deleted {
                to_restore.push((ancestor.clone(), ancestor.x + dx, ancestor.y + dy));
            }
        }
    }
    let mut restored = Vec::new();
    let mut node_restored = Vec::new();
    for (canvas, new_x, new_y) in to_restore {
        let old_x = canvas.x;
        let old_y = canvas.y;
        let canvas_id = canvas.id.clone();
        let mut canvas = canvas;
        canvas.deleted = false;
        canvas.x = new_x;
        canvas.y = new_y;
        dao::update(&connection, &canvas)?;
        restored.push((
            canvas.id,
            Action::CanvasRestore {
                name: canvas.name,
                old_x,
                old_y,
                new_x,
                new_y,
            },
        ));
        // 恢复引用该画布的画布节点（若存在且已逻辑删除）
        if let Some(mut ref_node) = node::dao::select_by_canvas_ref_id(&connection, &canvas_id)? {
            if ref_node.deleted {
                let old_x = ref_node.x;
                let old_y = ref_node.y;
                ref_node.deleted = false;
                node::dao::update(&connection, &ref_node)?;
                node_restored.push((
                    ref_node.id,
                    Action::NodeRestore {
                        title: ref_node.title,
                        old_x,
                        old_y,
                        new_x: old_x,
                        new_y: old_y,
                    },
                ));
            }
        }
    }
    for (id, action) in restored {
        log::service::create(&id, action)?;
    }
    for (id, action) in node_restored {
        log::service::create(&id, action)?;
    }
    Ok(())
}
