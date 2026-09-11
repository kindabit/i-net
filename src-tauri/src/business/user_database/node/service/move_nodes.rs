use crate::business::user_database::entity::{Action, Node};
use crate::business::user_database::node::dao;
use crate::business::user_database::node::vo::MoveNodeVO;
use crate::business::user_database::shadow::service::display_title;
use crate::business::user_database::{log, state};
use crate::error_code::ErrorCode;

/// 批量移动节点坐标。若所有条目均未发生位移，则不更新数据库、不产生日志。
///
/// 实际位移的节点数量为 1 时产生一条 NodeMove 日志，载荷为节点标题、旧坐标和新坐标；
/// 大于 1 时产生一条 AutoLayoutDataNodes 日志，node_count 为实际位移的节点数量。
///
/// # 参数
/// - `items`: 要移动的节点列表。
///
/// # 返回值
/// 成功时返回 `Ok(())`；任一节点不存在时返回 `ErrorCode::NoNodeWithSuchId`，
/// 影子链解析失败时返回对应的 `DataCorruption*` 错误，
/// 发生其他错误时返回对应的 `ErrorCode`。
pub fn move_nodes(items: &[MoveNodeVO]) -> Result<(), ErrorCode> {
    let connection = state::lock_connection();
    // 逐个校验存在性，同时记录节点实体（含旧坐标；单条位移分支产日志时还需展示标题）。
    let mut nodes: Vec<Node> = Vec::new();
    for item in items {
        let node = dao::select_by_id(&connection, &item.id)?
            .ok_or_else(|| ErrorCode::NoNodeWithSuchId {
                id: item.id.clone(),
            })?;
        nodes.push(node);
    }
    // 筛出实际发生位移的条目（连同其节点实体）。
    let moved: Vec<(&MoveNodeVO, &Node)> = items
        .iter()
        .zip(nodes.iter())
        .filter(|(item, node)| item.x != node.x || item.y != node.y)
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
        let (item, node) = moved[0];
        // 日志载荷的标题取展示标题：影子节点的标题落库为空串，须沿产生边链解析根本体标题。
        // 移动不改写标题与影子引用，用更新前的实体解析即可。
        let title = display_title(&connection, node)?;
        log::service::create(Action::NodeMove {
            title,
            old_x: node.x,
            old_y: node.y,
            new_x: item.x,
            new_y: item.y,
        })?;
    } else {
        let node_count = moved.len() as i64;
        log::service::create(Action::AutoLayoutDataNodes { node_count })?;
    }
    Ok(())
}
