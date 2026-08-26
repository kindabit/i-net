use rusqlite::Connection;

use crate::business::user_database::entity::Node;
use crate::business::user_database::node::vo::ShadowDirection;
use crate::business::user_database::{edge, node::dao};
use crate::error_code::ErrorCode;

/// 推导影子节点在其所在画布内的方向：找到引用影子所在画布的画布节点 B，再判断原始节点 X 与 B 之间边的方向；
/// X→B（X 是源）为入向影子 Inflow，B→X（B 是源）为出向影子 Outflow。
///
/// 前置条件：`shadow` 必须是影子节点（shadow_id 非空）。影子只可能由边联动创建于被引用的
/// 子画布内，且与"原始节点↔画布节点之间的边"同生共死，因此数据一致时方向必然可推导；
/// 推导不出即数据损坏或程序缺陷，返回 DataCorruption* 错误，绝不静默放行。
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
    let Some(origin_id) = shadow.shadow_id.as_deref() else {
        return Err(ErrorCode::DataCorruptionNodeNotShadow { id: shadow.id.clone() });
    };
    let Some(canvas_node) = dao::select_by_canvas_ref_id(connection, &shadow.canvas_id)? else {
        return Err(ErrorCode::DataCorruptionShadowCanvasUnreferenced {
            shadow_id: shadow.id.clone(),
            canvas_id: shadow.canvas_id.clone(),
        });
    };
    if edge::dao::exists_between(connection, origin_id, &canvas_node.id)? {
        return Ok(ShadowDirection::Inflow);
    }
    if edge::dao::exists_between(connection, &canvas_node.id, origin_id)? {
        return Ok(ShadowDirection::Outflow);
    }
    Err(ErrorCode::DataCorruptionShadowWithoutOriginEdge {
        shadow_id: shadow.id.clone(),
        origin_id: origin_id.to_string(),
        canvas_node_id: canvas_node.id.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::business::user_database::edge;
    use crate::business::user_database::entity::Edge;

    /// 构造测试用 Node，各字段可由调用方再修改。
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

    /// 构造测试用 Edge。
    fn make_edge(id: &str, canvas_id: &str, source_id: &str, target_id: &str) -> Edge {
        Edge {
            id: id.to_string(),
            canvas_id: canvas_id.to_string(),
            source_id: source_id.to_string(),
            source_port: "right".to_string(),
            target_id: target_id.to_string(),
            target_port: "left".to_string(),
            title: String::new(),
            description: String::new(),
        }
    }

    /// 失败路径：参数不是影子节点（shadow_id 为 None）时报 DataCorruptionNodeNotShadow。
    #[test]
    fn test_shadow_direction_not_shadow() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch("PRAGMA foreign_keys = OFF;")
            .unwrap();
        dao::create_table(&connection).unwrap();
        let not_shadow = make_node("n1", "canvas-1");
        assert!(matches!(
            shadow_direction(&connection, &not_shadow),
            Err(ErrorCode::DataCorruptionNodeNotShadow { id })
            if id == "n1"
        ));
    }

    /// 失败路径：影子所在画布没有被任何画布节点引用时报 DataCorruptionShadowCanvasUnreferenced。
    #[test]
    fn test_shadow_direction_canvas_unreferenced() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch("PRAGMA foreign_keys = OFF;")
            .unwrap();
        dao::create_table(&connection).unwrap();
        let mut shadow = make_node("s1", "orphan-canvas");
        shadow.shadow_id = Some("origin-1".to_string());
        assert!(matches!(
            shadow_direction(&connection, &shadow),
            Err(ErrorCode::DataCorruptionShadowCanvasUnreferenced {
                shadow_id,
                canvas_id,
            }) if shadow_id == "s1" && canvas_id == "orphan-canvas"
        ));
    }

    /// 失败路径：画布节点存在但与原始节点之间没有边时报 DataCorruptionShadowWithoutOriginEdge。
    #[test]
    fn test_shadow_direction_without_origin_edge() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch("PRAGMA foreign_keys = OFF;")
            .unwrap();
        dao::create_table(&connection).unwrap();
        edge::dao::create_table(&connection).unwrap();
        let mut cn = make_node("B", "parent-canvas");
        cn.canvas_ref_id = Some("sub-canvas".to_string());
        dao::insert(&connection, &cn).unwrap();
        let mut shadow = make_node("s1", "sub-canvas");
        shadow.shadow_id = Some("origin-x".to_string());
        assert!(matches!(
            shadow_direction(&connection, &shadow),
            Err(ErrorCode::DataCorruptionShadowWithoutOriginEdge {
                shadow_id,
                origin_id,
                canvas_node_id,
            }) if shadow_id == "s1" && origin_id == "origin-x" && canvas_node_id == "B"
        ));
    }

    /// 成功路径：原始节点→画布节点的边推导出 Inflow；画布节点→原始节点的边推导出 Outflow。
    /// 同一测试函数内顺序构造两种拓扑以减少代码量。
    #[test]
    fn test_shadow_direction_success() {
        // 子画布 1：X→B 边使对应影子为 Inflow。
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch("PRAGMA foreign_keys = OFF;")
            .unwrap();
        dao::create_table(&connection).unwrap();
        edge::dao::create_table(&connection).unwrap();
        let mut cn1 = make_node("B1", "parent-1");
        cn1.canvas_ref_id = Some("sub-1".to_string());
        dao::insert(&connection, &cn1).unwrap();
        dao::insert(&connection, &make_node("X1", "parent-1")).unwrap();
        edge::dao::insert(&connection, &make_edge("e1", "parent-1", "X1", "B1")).unwrap();
        let mut shadow_in = make_node("s-in", "sub-1");
        shadow_in.shadow_id = Some("X1".to_string());
        assert_eq!(
            shadow_direction(&connection, &shadow_in).unwrap(),
            ShadowDirection::Inflow
        );

        // 子画布 2：B→Z 边使对应影子为 Outflow。
        let mut cn2 = make_node("B2", "parent-2");
        cn2.canvas_ref_id = Some("sub-2".to_string());
        dao::insert(&connection, &cn2).unwrap();
        dao::insert(&connection, &make_node("Z2", "parent-2")).unwrap();
        edge::dao::insert(&connection, &make_edge("e2", "parent-2", "B2", "Z2")).unwrap();
        let mut shadow_out = make_node("s-out", "sub-2");
        shadow_out.shadow_id = Some("Z2".to_string());
        assert_eq!(
            shadow_direction(&connection, &shadow_out).unwrap(),
            ShadowDirection::Outflow
        );
    }
}