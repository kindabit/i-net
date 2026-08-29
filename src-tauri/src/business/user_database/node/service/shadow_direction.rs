use rusqlite::Connection;

use crate::business::user_database::edge;
use crate::business::user_database::entity::Node;
use crate::business::user_database::node::dao;
use crate::business::user_database::node::vo::ShadowDirection;
use crate::error_code::ErrorCode;

/// 推导影子节点在其所在画布内的方向：方向由影子的产生边决定——
/// 产生边源端是画布节点时为 Outflow（出向影子），否则为 Inflow（入向影子）。
///
/// 前置条件：`shadow` 必须是影子节点（shadow_id 非空）。影子只可能由边联动创建并与
/// 产生边同生共死，因此数据一致时方向必然可推导；推导不出即数据损坏或程序缺陷，
/// 返回 DataCorruption* 错误，绝不静默放行。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `shadow`: 影子节点。
///
/// # 返回值
/// 返回推导出的方向；数据不一致时返回对应的 DataCorruption* 错误；数据库错误返回对应的 `ErrorCode`。
pub fn shadow_direction(
    connection: &Connection,
    shadow: &Node,
) -> Result<ShadowDirection, ErrorCode> {
    let Some(edge_id) = shadow.shadow_id.as_deref() else {
        return Err(ErrorCode::DataCorruptionNodeNotShadow {
            id: shadow.id.clone(),
        });
    };
    let producing_edge = edge::dao::select_by_id(connection, edge_id)?.ok_or_else(|| {
        ErrorCode::DataCorruptionDanglingShadow {
            shadow_id: shadow.id.clone(),
            missing_id: edge_id.to_string(),
        }
    })?;
    let source = dao::select_by_id(connection, &producing_edge.source_id)?.ok_or_else(|| {
        ErrorCode::DataCorruptionEdgeEndpointMissing {
            edge_id: producing_edge.id.clone(),
            node_id: producing_edge.source_id.clone(),
        }
    })?;
    if source.canvas_ref_id.is_some() {
        Ok(ShadowDirection::Outflow)
    } else {
        Ok(ShadowDirection::Inflow)
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

    /// 构造测试用 Node，仅设置 id 与 canvas_id，title / sub_title / color 等字段取默认值。
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

    /// 在 connection 上建齐 canvas / edge / node 三张表并插入一张画布，返回画布 id。
    /// 本测试聚焦本模块逻辑，关闭外键以便灵活构造引用；外键级联行为由 dao 测试覆盖。
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
            name: "shadow-dir-canvas".to_string(),
            x: 0.0,
            y: 0.0,
            deleted: false,
            color: String::new(),
        };
        canvas_dao::insert(connection, &canvas).unwrap();
        canvas.id
    }

    /// 非影子节点调用 shadow_direction 时返回 DataCorruptionNodeNotShadow。
    #[test]
    fn test_shadow_direction_non_shadow_returns_node_not_shadow() {
        let connection = Connection::open_in_memory().unwrap();
        let canvas_id = setup_canvas(&connection);
        let plain = make_node("plain-1", &canvas_id);
        node_dao::insert(&connection, &plain).unwrap();

        let err = shadow_direction(&connection, &plain).unwrap_err();
        assert!(matches!(
            err,
            ErrorCode::DataCorruptionNodeNotShadow { ref id } if id == "plain-1"
        ));
    }

    /// 影子的 shadow_id 指向不存在的边时返回 DataCorruptionDanglingShadow。
    #[test]
    fn test_shadow_direction_dangling_returns_dangling_shadow() {
        let connection = Connection::open_in_memory().unwrap();
        let canvas_id = setup_canvas(&connection);
        let mut shadow = make_node("shadow-1", &canvas_id);
        // shadow_id 指向不存在的边 id。
        shadow.shadow_id = Some("no-such-edge-id".to_string());
        node_dao::insert(&connection, &shadow).unwrap();

        let err = shadow_direction(&connection, &shadow).unwrap_err();
        assert!(matches!(
            err,
            ErrorCode::DataCorruptionDanglingShadow {
                ref shadow_id,
                ref missing_id,
            } if shadow_id == "shadow-1" && missing_id == "no-such-edge-id"
        ));
    }

    /// 影子产生边的源端节点缺失（脏数据构造）时返回 DataCorruptionEdgeEndpointMissing。
    #[test]
    fn test_shadow_direction_missing_source_returns_edge_endpoint_missing() {
        let connection = Connection::open_in_memory().unwrap();
        let canvas_id = setup_canvas(&connection);
        // 真实存在的 target 节点（仅为了让 edge 行能 insert）。
        let target = make_node("target-1", &canvas_id);
        node_dao::insert(&connection, &target).unwrap();
        // 边 source 指向不存在的节点 id。
        let bad_edge = Edge {
            id: "edge-1".to_string(),
            canvas_id: canvas_id.clone(),
            source_id: "missing-source-id".to_string(),
            source_port: "right".to_string(),
            target_id: target.id.clone(),
            target_port: "left".to_string(),
            title: String::new(),
            description: String::new(),
        };
        edge_dao::insert(&connection, &bad_edge).unwrap();
        // 影子指向这条边，shadow_direction 内部 select_by_id(source_id) 必然返回 None。
        let mut shadow = make_node("shadow-1", &canvas_id);
        shadow.shadow_id = Some(bad_edge.id.clone());
        node_dao::insert(&connection, &shadow).unwrap();

        let err = shadow_direction(&connection, &shadow).unwrap_err();
        assert!(matches!(
            err,
            ErrorCode::DataCorruptionEdgeEndpointMissing {
                ref edge_id,
                ref node_id,
            } if edge_id == "edge-1" && node_id == "missing-source-id"
        ));
    }

    /// 产生边源端是普通节点时推导为 Inflow（普通节点的影子）。
    #[test]
    fn test_shadow_direction_inflow_when_source_is_plain() {
        let connection = Connection::open_in_memory().unwrap();
        let canvas_id = setup_canvas(&connection);
        let source = make_node("source-plain", &canvas_id);
        node_dao::insert(&connection, &source).unwrap();
        let target = make_node("target-canvas", &canvas_id);
        node_dao::insert(&connection, &target).unwrap();
        let edge = Edge {
            id: "edge-inflow".to_string(),
            canvas_id: canvas_id.clone(),
            source_id: source.id.clone(),
            source_port: "right".to_string(),
            target_id: target.id.clone(),
            target_port: "left".to_string(),
            title: String::new(),
            description: String::new(),
        };
        edge_dao::insert(&connection, &edge).unwrap();
        let mut shadow = make_node("shadow-inflow", &canvas_id);
        shadow.shadow_id = Some(edge.id.clone());
        node_dao::insert(&connection, &shadow).unwrap();

        let direction = shadow_direction(&connection, &shadow).unwrap();
        assert_eq!(direction, ShadowDirection::Inflow);
    }

    /// 产生边源端是画布节点（canvas_ref_id 非空）时推导为 Outflow（画布节点的影子）。
    #[test]
    fn test_shadow_direction_outflow_when_source_is_canvas_node() {
        let connection = Connection::open_in_memory().unwrap();
        let canvas_id = setup_canvas(&connection);
        // 画布节点的 canvas_ref_id 指向另一个已存在的画布。
        let mut source = make_node("source-canvas", &canvas_id);
        source.canvas_ref_id = Some("sub-canvas-1".to_string());
        node_dao::insert(&connection, &source).unwrap();
        let target = make_node("target-plain", &canvas_id);
        node_dao::insert(&connection, &target).unwrap();
        let edge = Edge {
            id: "edge-outflow".to_string(),
            canvas_id: canvas_id.clone(),
            source_id: source.id.clone(),
            source_port: "right".to_string(),
            target_id: target.id.clone(),
            target_port: "left".to_string(),
            title: String::new(),
            description: String::new(),
        };
        edge_dao::insert(&connection, &edge).unwrap();
        let mut shadow = make_node("shadow-outflow", &canvas_id);
        shadow.shadow_id = Some(edge.id.clone());
        node_dao::insert(&connection, &shadow).unwrap();

        let direction = shadow_direction(&connection, &shadow).unwrap();
        assert_eq!(direction, ShadowDirection::Outflow);
    }
}
