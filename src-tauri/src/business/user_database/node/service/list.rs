use std::collections::HashSet;

use rusqlite::Connection;

use crate::business::user_database::entity::Node;
use crate::business::user_database::node::dao;
use crate::business::user_database::node::service::shadow_direction;
use crate::business::user_database::node::vo::NodeVO;
use crate::business::user_database::state;
use crate::error_code::ErrorCode;

/// 返回指定画布内的正常节点或者已经逻辑删除的节点（以 NodeVO 形式）。不产生日志。
///
/// 影子节点的展示数据合并为原始节点的值，并附带原始节点状态与影子方向。
///
/// # 参数
/// - `canvas_id`: 画布 id。
/// - `deleted`: 逻辑删除标志，false 返回正常节点，true 返回已逻辑删除的节点。
///
/// # 返回值
/// 返回节点值对象列表；若发生错误则返回对应的 `ErrorCode`。
pub fn list(canvas_id: &str, deleted: bool) -> Result<Vec<NodeVO>, ErrorCode> {
    let connection = state::lock_connection();
    let nodes = dao::select_by_canvas_id_and_deleted(&connection, canvas_id, deleted)?;
    nodes.into_iter().map(|node| to_vo(&connection, node)).collect()
}

/// 将 Node 转换为 NodeVO，影子节点合并原始节点的展示数据。
///
/// 影子链支持嵌套：影子节点的 shadow_id 指向其直接来源节点；展示数据沿影子链
/// 级联向上查到根原始节点（shadow_id 为 None 的节点），合并 title / sub_title /
/// color；原始节点状态以根为准。影子方向只对当前节点的直接来源推导。
/// canvas_ref_id 在新规则下恒为 None：画布节点之间不允许建边，影子的原始节点
/// 只能是普通节点，因此不需要合并 canvas_ref_id；遍历根节点时其 canvas_ref_id
/// 也恒为 None，合并是无操作，故此处不再写入。
///
/// 影子链由外键级联保证完整；悬空或成环即数据损坏，返回 DataCorruption* 错误。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `node`: 待转换的节点。
///
/// # 返回值
/// 返回转换后的节点值对象；影子链上任一节点的原始节点缺失时返回
/// `ErrorCode::DataCorruptionDanglingShadow`；影子链成环时返回
/// `ErrorCode::DataCorruptionShadowChainCycle`；影子方向推导失败时返回对应的
/// `DataCorruption*` 错误；数据库错误返回对应的 `ErrorCode`。
fn to_vo(connection: &Connection, node: Node) -> Result<NodeVO, ErrorCode> {
    let Some(first_origin_id) = node.shadow_id.clone() else {
        return Ok(NodeVO { node, shadow_origin_deleted: None, shadow_direction: None });
    };
    let direction = shadow_direction(connection, &node)?;
    // 沿影子链向上查到根原始节点：展示数据与"原始节点已删除"状态以根为准。
    let mut visited: HashSet<String> = HashSet::new();
    let mut cursor = match dao::select_by_id(connection, &first_origin_id)? {
        Some(origin) => origin,
        None => {
            return Err(ErrorCode::DataCorruptionDanglingShadow {
                shadow_id: node.id.clone(),
                missing_id: first_origin_id,
            });
        }
    };
    while let Some(next_id) = cursor.shadow_id.clone() {
        if !visited.insert(cursor.id.clone()) {
            return Err(ErrorCode::DataCorruptionShadowChainCycle {
                id: cursor.id.clone(),
            });
        }
        match dao::select_by_id(connection, &next_id)? {
            Some(next) => cursor = next,
            None => {
                return Err(ErrorCode::DataCorruptionDanglingShadow {
                    shadow_id: cursor.id.clone(),
                    missing_id: next_id,
                });
            }
        }
    }
    let mut merged = node;
    merged.title = cursor.title;
    merged.sub_title = cursor.sub_title;
    merged.color = cursor.color;
    Ok(NodeVO { node: merged, shadow_origin_deleted: Some(cursor.deleted), shadow_direction: Some(direction) })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// to_vo 失败路径：影子节点的 shadow_id 悬空（仅可能由数据污染或外键被关闭造成）时报 DataCorruptionDanglingShadow。
    /// 构造策略：先配齐 shadow_direction 所需的画布节点与"原始节点→画布节点"边，让方向可推导，
    /// 再删除原始节点模拟悬空（外键关闭使删除成为可能）。
    #[test]
    fn test_to_vo_shadow_missing_origin() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch("PRAGMA foreign_keys = OFF;")
            .unwrap();
        dao::create_table(&connection).unwrap();
        crate::business::user_database::edge::dao::create_table(&connection).unwrap();
        // 画布节点 cn（canvas_ref_id = "canvas-1"）。
        let mut cn = make_node("cn", "parent-canvas");
        cn.canvas_ref_id = Some("canvas-1".to_string());
        dao::insert(&connection, &cn).unwrap();
        // 原始节点 origin-x 与边 origin-x → cn。
        let origin = make_node("origin-x", "parent-canvas");
        dao::insert(&connection, &origin).unwrap();
        insert_edge(&connection, "e1", "parent-canvas", "origin-x", "cn");
        // 影子节点 shadow-1（shadow_id 指向 origin-x）。
        let shadow = Node {
            id: "shadow-1".to_string(),
            canvas_id: "canvas-1".to_string(),
            x: 0.0,
            y: 0.0,
            title: String::new(),
            sub_title: String::new(),
            canvas_ref_id: None,
            deleted: false,
            color: String::new(),
            shadow_id: Some("origin-x".to_string()),
        };
        dao::insert(&connection, &shadow).unwrap();
        // 删除原始节点模拟悬空（外键关闭，边的行仍保留，shadow_direction 因此仍可推导）。
        dao::delete_by_id(&connection, "origin-x").unwrap();
        assert!(matches!(
            to_vo(&connection, shadow),
            Err(ErrorCode::DataCorruptionDanglingShadow {
                shadow_id,
                missing_id,
            }) if shadow_id == "shadow-1" && missing_id == "origin-x"
        ));
    }

    /// to_vo 失败路径：影子链成环时返回 DataCorruptionShadowChainCycle。
    ///
    /// 构造两个影子节点互相把对方作为 shadow_id：to_vo 收到 s_a 后首跳查到 s_b，随后沿
    /// 互指链 s_b→s_a→s_b 前进，第三圈时 s_b 已在 visited 中，触发成环。
    /// 影子方向推导需要原始节点↔画布节点的边，本测试给两个影子分别配好各自方向的边。
    #[test]
    fn test_to_vo_shadow_chain_cycle() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch("PRAGMA foreign_keys = OFF;")
            .unwrap();
        dao::create_table(&connection).unwrap();
        crate::business::user_database::edge::dao::create_table(&connection).unwrap();

        // 两个影子互相指向：s-a.shadow_id == s-b.id，s-b.shadow_id == s-a.id。
        // s-a 所在画布 sub-a 由画布节点 Ba 引用；s-a 的方向需要"s-a 指向的来源 s-b" 与 Ba 之间存在边。
        // 反之 s-b 的方向需要"s-b 指向的来源 s-a" 与 Bb 之间存在边。
        let mut ba = make_node("Ba", "parent-a");
        ba.canvas_ref_id = Some("sub-a".to_string());
        dao::insert(&connection, &ba).unwrap();
        let mut bb = make_node("Bb", "parent-b");
        bb.canvas_ref_id = Some("sub-b".to_string());
        dao::insert(&connection, &bb).unwrap();
        let mut s_a = make_node("s-a", "sub-a");
        s_a.shadow_id = Some("s-b".to_string());
        dao::insert(&connection, &s_a).unwrap();
        let mut s_b = make_node("s-b", "sub-b");
        s_b.shadow_id = Some("s-a".to_string());
        dao::insert(&connection, &s_b).unwrap();

        // 边 s-b → Ba 让 s-a 的方向可推导为 Inflow。
        insert_edge(&connection, "e-b-a", "parent-a", "s-b", "Ba");
        // 边 Ba → s-a 让 s-b 的方向可推导为 Outflow。
        insert_edge(&connection, "e-a-b", "parent-b", "Bb", "s-a");

        // 直接调用 to_vo 处理 s_a：首跳查到 s_b，沿互指链前进至第三圈时 s_b 重复，触发成环。
        assert!(matches!(
            to_vo(&connection, s_a),
            Err(ErrorCode::DataCorruptionShadowChainCycle { id }) if id == "s-b"
        ));
    }

    /// 构造测试用 Node。
    fn make_node(id: &str, canvas_id: &str) -> Node {
        Node {
            id: id.to_string(),
            canvas_id: canvas_id.to_string(),
            x: 0.0,
            y: 0.0,
            title: String::new(),
            sub_title: String::new(),
            canvas_ref_id: None,
            deleted: false,
            color: String::new(),
            shadow_id: None,
        }
    }

    /// 构造并插入测试用 Edge。
    fn insert_edge(
        connection: &Connection,
        id: &str,
        canvas_id: &str,
        source_id: &str,
        target_id: &str,
    ) {
        let edge = crate::business::user_database::entity::Edge {
            id: id.to_string(),
            canvas_id: canvas_id.to_string(),
            source_id: source_id.to_string(),
            source_port: "right".to_string(),
            target_id: target_id.to_string(),
            target_port: "left".to_string(),
            title: String::new(),
            description: String::new(),
        };
        crate::business::user_database::edge::dao::insert(connection, &edge).unwrap();
    }
}