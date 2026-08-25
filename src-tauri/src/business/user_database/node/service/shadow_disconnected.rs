use std::collections::HashSet;

use rusqlite::Connection;

use crate::business::user_database::edge;
use crate::business::user_database::entity::Node;
use crate::business::user_database::node::dao;
use crate::business::user_database::node::service::shadow_direction;
use crate::business::user_database::node::vo::ShadowDirection;
use crate::error_code::ErrorCode;

/// 收集影子节点被物理删除时其所在画布以及下游各级画布内将失去连接的节点的展示标题（去重）。
///
/// 算法：
/// 1. 取该影子所在画布的全部边，按当前影子的方向过滤规则取邻居；
/// 2. 邻居不存在则跳过；邻居标题：邻居本身是影子时沿影子链向上取根原始节点的标题，
///    否则取邻居自身标题（影子行自身标题是空串）；
/// 3. 邻居是画布节点（canvas_ref_id 非空）时，shadow 被删除后其在引用画布内的嵌套影子
///    由外键级联删除，递归对下一层影子收集并合并结果；
/// 4. 标题去重后返回。
///
/// 影子方向因数据不一致推导不出时两个方向都收集，与
/// `collect_disconnected_nodes`（原 edge delete.rs 内的私有函数）保持一致。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `shadow`: 即将被物理删除的影子节点。
///
/// # 返回值
/// 返回受影响节点的标题列表（去重）；发生数据库错误时返回对应的 `ErrorCode`。
pub fn collect_shadow_disconnected(
    connection: &Connection,
    shadow: &Node,
) -> Result<Vec<String>, ErrorCode> {
    let mut affected: Vec<String> = Vec::new();
    collect_into(connection, shadow, &mut affected)?;
    Ok(affected)
}

/// 收集影子节点在当前画布内的受影响邻居标题，并向下递归收集嵌套影子带来的断连。
///
/// 邻居本身是影子时其 canvas_ref_id 在数据库行中恒为 None，不会触发本函数对影子邻居
/// 的递归分支；只有画布节点邻居才会引出下一层嵌套影子的递归扫描。
fn collect_into(
    connection: &Connection,
    shadow: &Node,
    affected: &mut Vec<String>,
) -> Result<(), ErrorCode> {
    let direction = shadow_direction(connection, shadow)?;
    let edges = edge::dao::select_by_canvas_id(connection, &shadow.canvas_id)?;
    let mut downstream_shadows: Vec<Node> = Vec::new();
    for edge_record in &edges {
        let neighbor_id = if edge_record.source_id == shadow.id
            && direction != Some(ShadowDirection::Outflow)
        {
            Some(edge_record.target_id.as_str())
        } else if edge_record.target_id == shadow.id
            && direction != Some(ShadowDirection::Inflow)
        {
            Some(edge_record.source_id.as_str())
        } else {
            None
        };
        let Some(neighbor_id) = neighbor_id else {
            continue;
        };
        let Some(neighbor) = dao::select_by_id(connection, neighbor_id)? else {
            continue;
        };
        // 影子行自身标题是空串；展示用其根原始节点的标题。
        let title = root_origin_title(connection, &neighbor)?;
        if !affected.contains(&title) {
            affected.push(title);
        }
        // 邻居是画布节点时，shadow 删除后该画布内 shadow 的嵌套影子会由外键级联删除，
        // 需要继续递归收集下一层画布中的断连。影子邻居的 canvas_ref_id 恒为 None，
        // 不会进入此分支，因此无需额外判断。
        if let Some(ref_canvas_id) = &neighbor.canvas_ref_id {
            if let Some(nested_shadow) = dao::select_by_shadow_id_and_canvas_id(
                connection,
                &shadow.id,
                ref_canvas_id,
            )? {
                downstream_shadows.push(nested_shadow);
            }
        }
    }
    for nested in downstream_shadows {
        collect_into(connection, &nested, affected)?;
    }
    Ok(())
}

/// 沿影子链向上查到根原始节点并返回其标题；邻居本身不是影子时直接返回其标题。
/// 用 visited 集合防御脏数据成环：成环时停止向上，返回当前节点（叶子）的标题。
fn root_origin_title(connection: &Connection, node: &Node) -> Result<String, ErrorCode> {
    let mut visited: HashSet<String> = HashSet::new();
    let mut cursor = node.clone();
    while let Some(next_id) = cursor.shadow_id.clone() {
        if !visited.insert(cursor.id.clone()) {
            break;
        }
        match dao::select_by_id(connection, &next_id)? {
            Some(next) => cursor = next,
            // 影子链中途悬空：返回当前已知节点的标题，与"宁可降级也不锁死"原则一致。
            None => break,
        }
    }
    Ok(cursor.title)
}