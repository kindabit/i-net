use crate::business::user_database::edge::dao;
use crate::business::user_database::entity::Edge;
use crate::business::user_database::state;
use crate::error_code::ErrorCode;

/// 返回指定边。不产生日志。
///
/// # 参数
/// - `id`: 边 id。
///
/// # 返回值
/// 返回该边；边不存在时返回 `ErrorCode::NoEdgeWithSuchId`；若发生错误则返回对应的 `ErrorCode`。
pub fn get(id: &str) -> Result<Edge, ErrorCode> {
    let connection = state::lock_connection();
    dao::select_by_id(&connection, id)?.ok_or_else(|| ErrorCode::NoEdgeWithSuchId { id: id.to_string() })
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use super::*;
    use crate::business::user_database::canvas::dao as canvas_dao;
    use crate::business::user_database::edge::dao as edge_dao;
    use crate::business::user_database::entity::{Canvas, Node};
    use crate::business::user_database::node::dao as node_dao;

    /// 在 connection 上建齐 canvas / edge / node 三张表，插入一张画布与两个端点节点，返回画布 id。
    /// 本测试聚焦本模块逻辑，关闭外键以便灵活构造引用；外键级联行为由 dao 测试覆盖。
    fn setup(connection: &Connection) -> String {
        connection
            .execute_batch("PRAGMA foreign_keys = OFF;")
            .unwrap();
        canvas_dao::create_table(connection).unwrap();
        edge_dao::create_table(connection).unwrap();
        node_dao::create_table(connection).unwrap();
        let canvas = Canvas {
            id: "canvas-1".to_string(),
            parent_id: None,
            name: "edge-get-canvas".to_string(),
            x: 0.0,
            y: 0.0,
            deleted: false,
            color: String::new(),
        };
        canvas_dao::insert(connection, &canvas).unwrap();
        for node_id in ["node-source", "node-target"] {
            let node = Node {
                id: node_id.to_string(),
                canvas_id: canvas.id.clone(),
                x: 0.0,
                y: 0.0,
                title: node_id.to_string(),
                sub_title: String::new(),
                canvas_ref_id: None,
                deleted: false,
                color: String::new(),
                shadow_id: None,
            };
            node_dao::insert(connection, &node).unwrap();
        }
        canvas.id
    }

    /// 构造测试用边；调用方按需修改字段。
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

    /// 成功路径：按 id 取回的边各字段与入库时一致。
    #[test]
    fn test_get_returns_edge() {
        // 串行化依赖全局状态的测试，独占全局连接状态。
        let _guard = crate::test::acquire_test_lock();
        let connection = Connection::open_in_memory().unwrap();
        let canvas_id = setup(&connection);
        let edge = make_edge("edge-1", &canvas_id, "node-source", "node-target");
        edge_dao::insert(&connection, &edge).unwrap();
        state::set_connection(connection);

        let found = get("edge-1").unwrap();
        assert_eq!(found.id, "edge-1");
        assert_eq!(found.canvas_id, canvas_id);
        assert_eq!(found.source_id, "node-source");
        assert_eq!(found.target_id, "node-target");

        state::clear();
    }

    /// 失败路径：边不存在时返回 NoEdgeWithSuchId，携带请求的 id。
    #[test]
    fn test_get_missing_returns_no_edge_with_such_id() {
        // 串行化依赖全局状态的测试，独占全局连接状态。
        let _guard = crate::test::acquire_test_lock();
        let connection = Connection::open_in_memory().unwrap();
        setup(&connection);
        state::set_connection(connection);

        let err = get("no-such-edge-id").unwrap_err();
        assert!(matches!(
            err,
            ErrorCode::NoEdgeWithSuchId { ref id } if id == "no-such-edge-id"
        ));

        state::clear();
    }
}
