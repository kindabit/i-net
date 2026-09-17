use std::collections::HashSet;

use rusqlite::Connection;

use crate::business::user_database::edge;
use crate::business::user_database::entity::Node;
use crate::business::user_database::node::dao;
use crate::error_code::ErrorCode;

/// 沿产生边链解析节点的本体节点：非影子节点的本体是其自身；
/// 影子节点读其产生边——产生边源端是画布数据节点（出向影子）时本体在目标端，
/// 否则（入向影子）本体在源端，沿该侧端点递归直到非影子节点。
///
/// 一条边只可能产生一个影子节点，当出现连接“源画布数据节点 -> 目标画布数据节点”的边时，
/// 只在源画布中建立目标画布数据节点的影子节点，而不是在源画布中建立目标画布数据节点的影子节点的同时再在
/// 目标画布中建立源画布数据节点的影子节点，这是本函数功能成立的前提条件之一，即每一层解析都只有一种可能性。
///
/// 影子链由外键级联保证完整；悬空、端点缺失或成环即数据损坏，返回 DataCorruption* 错误。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `node`: 待解析的节点。
///
/// # 返回值
/// 返回本体节点；影子产生边缺失时返回 `ErrorCode::DataCorruptionDanglingShadow`，
/// 产生边端点缺失时返回 `ErrorCode::DataCorruptionEdgeEndpointMissing`，
/// 影子链成环时返回 `ErrorCode::DataCorruptionShadowChainCycle`，
/// 数据库错误返回对应的 `ErrorCode`。
pub fn resolve_origin(connection: &Connection, node: &Node) -> Result<Node, ErrorCode> {
    let mut visited: HashSet<String> = HashSet::new();
    resolve_origin_into(connection, node, &mut visited)
}

/// resolve_origin 的递归实现：visited 记录链上已访问的影子节点 id 用于成环检测。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `node`: 当前待解析的节点。
/// - `visited`: 已访问的影子节点 id 集合，调用方提供初始空集。
///
/// # 返回值
/// 含义同 `resolve_origin`。
fn resolve_origin_into(
    connection: &Connection,
    node: &Node,
    visited: &mut HashSet<String>,
) -> Result<Node, ErrorCode> {
    let Some(edge_id) = node.shadow_producing_edge_id.clone() else {
        return Ok(node.clone());
    };
    if !visited.insert(node.id.clone()) {
        return Err(ErrorCode::DataCorruptionShadowChainCycle {
            id: node.id.clone(),
        });
    }
    let producing_edge = edge::dao::select_by_id(connection, &edge_id)?.ok_or_else(|| {
        ErrorCode::DataCorruptionDanglingShadow {
            shadow_id: node.id.clone(),
            missing_id: edge_id.clone(),
        }
    })?;
    let source = dao::select_by_id(connection, &producing_edge.source_id)?.ok_or_else(|| {
        ErrorCode::DataCorruptionEdgeEndpointMissing {
            edge_id: producing_edge.id.clone(),
            node_id: producing_edge.source_id.clone(),
        }
    })?;
    if source.canvas_ref_id.is_some() {
        let target = dao::select_by_id(connection, &producing_edge.target_id)?.ok_or_else(|| {
            ErrorCode::DataCorruptionEdgeEndpointMissing {
                edge_id: producing_edge.id.clone(),
                node_id: producing_edge.target_id.clone(),
            }
        })?;
        resolve_origin_into(connection, &target, visited)
    } else {
        resolve_origin_into(connection, &source, visited)
    }
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use super::*;
    use crate::business::user_database::canvas::dao as canvas_dao;
    use crate::business::user_database::edge::dao as edge_dao;
    use crate::business::user_database::entity::{Canvas, Edge};
    use crate::business::user_database::node::dao as node_dao;

    /// 构造测试用 Node，仅设置 id 与 canvas_id，其它字段取默认值。
    fn make_node(id: &str, canvas_id: &str) -> Node {
        Node {
            id: id.to_string(),
            canvas_id: canvas_id.to_string(),
            x: 0.0,
            y: 0.0,
            title: String::new(),
            subtitle: String::new(),
            canvas_ref_id: None,
            deleted: false,
            color: String::new(),
            shadow_producing_edge_id: None,
        }
    }

    /// 在 connection 上建齐 canvas / edge / node 三张表并插入一张画布，返回画布 id。
    fn setup_canvas(connection: &Connection) -> String {
        connection
            .execute_batch("PRAGMA foreign_keys = OFF;")
            .unwrap();
        canvas_dao::create_table(connection).unwrap();
        edge_dao::create_table(connection).unwrap();
        node_dao::create_table(connection).unwrap();
        let canvas = Canvas {
            id: "canvas-1".to_string(),
            parent_id: None,
            name: "resolve-origin-canvas".to_string(),
            x: 0.0,
            y: 0.0,
            deleted: false,
            color: String::new(),
        };
        canvas_dao::insert(connection, &canvas).unwrap();
        canvas.id
    }

    /// 在指定画布内插一条边记录（两端节点需先存在）；FK 关闭，源 / 目标可指向不存在的节点以构造脏数据。
    fn insert_edge(connection: &Connection, canvas_id: &str, id: &str, source_id: &str, target_id: &str) {
        let edge = Edge {
            id: id.to_string(),
            canvas_id: canvas_id.to_string(),
            source_id: source_id.to_string(),
            source_handle: "right".to_string(),
            target_id: target_id.to_string(),
            target_handle: "left".to_string(),
            title: String::new(),
            description: String::new(),
        };
        edge_dao::insert(connection, &edge).unwrap();
    }

    /// 非影子节点的本体是其自身。
    #[test]
    fn test_resolve_origin_non_shadow_returns_self() {
        let connection = Connection::open_in_memory().unwrap();
        let canvas_id = setup_canvas(&connection);
        let data = make_node("data-1", &canvas_id);
        node_dao::insert(&connection, &data).unwrap();

        let origin = resolve_origin(&connection, &data).unwrap();
        assert_eq!(origin.id, "data-1");
    }

    /// 单层入向影子：影子 S 的产生边源端是数据节点 X（无 canvas_ref_id），
    /// resolve_origin 沿源侧递归到 X 后终止。
    #[test]
    fn test_resolve_origin_single_inflow() {
        let connection = Connection::open_in_memory().unwrap();
        let canvas_id = setup_canvas(&connection);
        let origin_x = make_node("origin-x", &canvas_id);
        node_dao::insert(&connection, &origin_x).unwrap();
        let target_b = make_node("target-b", &canvas_id);
        node_dao::insert(&connection, &target_b).unwrap();
        insert_edge(&connection, &canvas_id, "edge-xb", &origin_x.id, &target_b.id);
        let mut shadow_s = make_node("shadow-s", &canvas_id);
        shadow_s.shadow_producing_edge_id = Some("edge-xb".to_string());
        node_dao::insert(&connection, &shadow_s).unwrap();

        let origin = resolve_origin(&connection, &shadow_s).unwrap();
        assert_eq!(origin.id, "origin-x");
    }

    /// 单层出向影子：影子 S 的产生边源端是画布数据节点 B1（canvas_ref_id 非空），
    /// resolve_origin 沿目标侧递归到本体画布数据节点 B2 后终止。
    #[test]
    fn test_resolve_origin_single_outflow() {
        let connection = Connection::open_in_memory().unwrap();
        let canvas_id = setup_canvas(&connection);
        // 画布数据节点 B1 / B2：canvas_ref_id 都指向已存在的画布（构造画布数据节点）。
        let mut b1 = make_node("b1", &canvas_id);
        b1.canvas_ref_id = Some("sub-canvas-1".to_string());
        node_dao::insert(&connection, &b1).unwrap();
        let mut b2 = make_node("b2", &canvas_id);
        b2.canvas_ref_id = Some("sub-canvas-2".to_string());
        node_dao::insert(&connection, &b2).unwrap();
        insert_edge(&connection, &canvas_id, "edge-b1b2", &b1.id, &b2.id);
        let mut shadow_s = make_node("shadow-s", &canvas_id);
        shadow_s.shadow_producing_edge_id = Some("edge-b1b2".to_string());
        node_dao::insert(&connection, &shadow_s).unwrap();

        let origin = resolve_origin(&connection, &shadow_s).unwrap();
        assert_eq!(origin.id, "b2");
    }

    /// 嵌套入向影子：S2.shadow_producing_edge_id=e2，e2.source=S1；S1.shadow_producing_edge_id=e1，e1.source=X（数据节点）→
    /// resolve_origin(S2) 沿 X 终止于本体 X。
    #[test]
    fn test_resolve_origin_nested_inflow() {
        let connection = Connection::open_in_memory().unwrap();
        let canvas_id = setup_canvas(&connection);
        let origin_x = make_node("origin-x", &canvas_id);
        node_dao::insert(&connection, &origin_x).unwrap();
        let canvas_b = make_node("canvas-b", &canvas_id);
        node_dao::insert(&connection, &canvas_b).unwrap();
        // e1：X → B（X 数据节点），B 处产生入向影子 S1。
        insert_edge(&connection, &canvas_id, "edge-e1", &origin_x.id, &canvas_b.id);
        let mut s1 = make_node("shadow-s1", &canvas_id);
        s1.shadow_producing_edge_id = Some("edge-e1".to_string());
        node_dao::insert(&connection, &s1).unwrap();
        let canvas_b1 = make_node("canvas-b1", &canvas_id);
        node_dao::insert(&connection, &canvas_b1).unwrap();
        // e2：S1 → B1（S1 是入向影子），B1 处产生嵌套入向影子 S2。
        insert_edge(&connection, &canvas_id, "edge-e2", &s1.id, &canvas_b1.id);
        let mut s2 = make_node("shadow-s2", &canvas_id);
        s2.shadow_producing_edge_id = Some("edge-e2".to_string());
        node_dao::insert(&connection, &s2).unwrap();

        let origin = resolve_origin(&connection, &s2).unwrap();
        assert_eq!(origin.id, "origin-x");
    }

    /// 嵌套出向影子：S2.shadow_producing_edge_id=e2，e2.source=B1（画布数据节点）→ 走目标侧；e2.target=S1；
    /// S1.shadow_producing_edge_id=e1，e1.source=B0（画布数据节点）→ 走目标侧；e1.target=B2（画布数据节点）→
    /// resolve_origin(S2) 沿产生边链（经目标端）终止于本体 B2。
    #[test]
    fn test_resolve_origin_nested_outflow() {
        let connection = Connection::open_in_memory().unwrap();
        let canvas_id = setup_canvas(&connection);
        // 三个画布数据节点 B0 / B1 / B2：canvas_ref_id 都指向已存在的画布以构成画布数据节点。
        let mut b0 = make_node("b0", &canvas_id);
        b0.canvas_ref_id = Some("sub-canvas-0".to_string());
        node_dao::insert(&connection, &b0).unwrap();
        let mut b1 = make_node("b1", &canvas_id);
        b1.canvas_ref_id = Some("sub-canvas-1".to_string());
        node_dao::insert(&connection, &b1).unwrap();
        let mut b2 = make_node("b2", &canvas_id);
        b2.canvas_ref_id = Some("sub-canvas-2".to_string());
        node_dao::insert(&connection, &b2).unwrap();
        // e1：B0 → B2，产生出向影子 S1。
        insert_edge(&connection, &canvas_id, "edge-e1", &b0.id, &b2.id);
        let mut s1 = make_node("shadow-s1", &canvas_id);
        s1.shadow_producing_edge_id = Some("edge-e1".to_string());
        node_dao::insert(&connection, &s1).unwrap();
        // e2：B1 → S1，产生嵌套出向影子 S2。
        insert_edge(&connection, &canvas_id, "edge-e2", &b1.id, &s1.id);
        let mut s2 = make_node("shadow-s2", &canvas_id);
        s2.shadow_producing_edge_id = Some("edge-e2".to_string());
        node_dao::insert(&connection, &s2).unwrap();

        let origin = resolve_origin(&connection, &s2).unwrap();
        assert_eq!(origin.id, "b2");
    }

    /// 影子产生边缺失时返回 DataCorruptionDanglingShadow。
    #[test]
    fn test_resolve_origin_dangling_returns_dangling_shadow() {
        let connection = Connection::open_in_memory().unwrap();
        let canvas_id = setup_canvas(&connection);
        let mut shadow = make_node("shadow-1", &canvas_id);
        shadow.shadow_producing_edge_id = Some("no-such-edge-id".to_string());
        node_dao::insert(&connection, &shadow).unwrap();

        let err = resolve_origin(&connection, &shadow).unwrap_err();
        assert!(matches!(
            err,
            ErrorCode::DataCorruptionDanglingShadow {
                ref shadow_id,
                ref missing_id,
            } if shadow_id == "shadow-1" && missing_id == "no-such-edge-id"
        ));
    }

    /// 影子链成环时返回 DataCorruptionShadowChainCycle：构造 s_a.shadow_producing_edge_id=e1，e1.source=s_b
    /// （数据节点侧递归走 source）；s_b.shadow_producing_edge_id=e2，e2.source=s_a——resolve_origin(s_a) →
    /// e1 → s_b → e2 → s_a 形成环。
    #[test]
    fn test_resolve_origin_cycle_returns_chain_cycle() {
        let connection = Connection::open_in_memory().unwrap();
        let canvas_id = setup_canvas(&connection);
        let mut s_a = make_node("s-a", &canvas_id);
        let mut s_b = make_node("s-b", &canvas_id);
        // FK 关闭，两条边的另一端可以指向尚不存在的节点；先插影子再插边以避开 FK。
        s_a.shadow_producing_edge_id = Some("edge-e1".to_string());
        s_b.shadow_producing_edge_id = Some("edge-e2".to_string());
        node_dao::insert(&connection, &s_a).unwrap();
        node_dao::insert(&connection, &s_b).unwrap();
        // e1.source=s_b（数据节点侧，递归走 source 端命中 s_b）。
        insert_edge(&connection, &canvas_id, "edge-e1", &s_b.id, &s_a.id);
        // e2.source=s_a（递归走 source 端命中 s_a，形成环）。
        insert_edge(&connection, &canvas_id, "edge-e2", &s_a.id, &s_b.id);

        let err = resolve_origin(&connection, &s_a).unwrap_err();
        assert!(matches!(
            err,
            ErrorCode::DataCorruptionShadowChainCycle { ref id } if id == "s-a" || id == "s-b"
        ));
    }
}
