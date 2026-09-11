use rusqlite::{Connection, OptionalExtension};

use crate::business::user_database::entity::Node;
use crate::business::user_database::node::dao::map_row;
use crate::business::user_database::node::dao::NodeIden;
use crate::error_code::ErrorCode;
use crate::util::sea_query_util::values_to_params;
use sea_query::{Expr, ExprTrait, Query, SqliteQueryBuilder};

/// 按产生边 id 查询其产生的影子节点（一条边至多产生一个影子）。
///
/// 影子节点存于 node 表（shadow_id 列指向产生边），行映射复用 node::dao::map_row。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `edge_id`: 产生边的 id（即影子节点 shadow_id 列的值）。
///
/// # 返回值
/// 返回查询到的影子节点，不存在时返回 `None`；若发生错误则返回对应的 `ErrorCode`。
pub fn select_by_producing_edge_id(
    connection: &Connection,
    edge_id: &str,
) -> Result<Option<Node>, ErrorCode> {
    let query = Query::select()
        .columns([
            NodeIden::Id,
            NodeIden::CanvasId,
            NodeIden::X,
            NodeIden::Y,
            NodeIden::Title,
            NodeIden::SubTitle,
            NodeIden::CanvasRefId,
            NodeIden::Deleted,
            NodeIden::Color,
            NodeIden::ShadowId,
        ])
        .from(NodeIden::Table)
        .and_where(Expr::col(NodeIden::ShadowId).eq(edge_id))
        .take();
    let (sql, values) = query.build(SqliteQueryBuilder);
    connection
        .query_row(
            &sql,
            rusqlite::params_from_iter(values_to_params(values)),
            map_row,
        )
        .optional()
        .map_err(|e| ErrorCode::DatabaseError {
            detail: e.to_string(),
        })
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use super::*;
    use crate::business::user_database::canvas::dao as canvas_dao;
    use crate::business::user_database::edge::dao as edge_dao;
    use crate::business::user_database::entity::{Canvas, Edge};
    use crate::business::user_database::node::dao as node_dao;

    /// 构造测试用 Node，各字段可由调用方再修改。
    fn node(id: &str, canvas_id: &str) -> Node {
        Node {
            id: id.to_string(),
            canvas_id: canvas_id.to_string(),
            x: 0.0,
            y: 0.0,
            title: format!("title-{id}"),
            sub_title: format!("sub-title-{id}"),
            canvas_ref_id: None,
            deleted: false,
            color: String::new(),
            shadow_id: None,
        }
    }

    /// 在 connection 上建齐 canvas / edge / node 三张表（FK 关闭，可无序插入数据）。
    fn setup_tables(connection: &Connection) {
        connection
            .execute_batch("PRAGMA foreign_keys = OFF;")
            .unwrap();
        canvas_dao::create_table(connection).unwrap();
        edge_dao::create_table(connection).unwrap();
        node_dao::create_table(connection).unwrap();
    }

    /// select_by_producing_edge_id 成功路径：按产生边 id 查到 shadow_id 等于该边 id 的影子节点；
    /// 未命中路径：传入不存在的产生边 id 时返回 None。
    #[test]
    fn test_select_by_producing_edge_id() {
        let connection = Connection::open_in_memory().unwrap();
        setup_tables(&connection);

        let mut shadow = node("shadow-node-1", "canvas-1");
        shadow.shadow_id = Some("edge-1".to_string());
        node_dao::insert(&connection, &shadow).unwrap();

        // 命中：按产生边 id 查到的影子节点等于 shadow-node-1，且 shadow_id 列的值与传入的
        // 产生边 id 一致（影子由产生边联动创建后即固化该引用）。
        let found = select_by_producing_edge_id(&connection, "edge-1")
            .unwrap()
            .unwrap();
        assert_eq!(found.id, "shadow-node-1");
        assert_eq!(found.shadow_id.as_deref(), Some("edge-1"));

        // 未命中：传入不存在的产生边 id 时返回 None。
        assert!(select_by_producing_edge_id(&connection, "no-such-edge-id")
            .unwrap()
            .is_none());
    }

    /// 独立测试：影子节点沿产生边外键级联链在物理删除边 / 节点时被一并清理。
    ///
    /// 新机制下 `node.shadow_id REFERENCES edge(id) ON DELETE CASCADE`、`edge.source_id/target_id
    /// REFERENCES node(id) ON DELETE CASCADE`：本测试开启外键约束并建齐 canvas / edge / node
    /// 三张表，沿两条级联链路验证：(a) 删除产生边 → 影子经 shadow_id 外键级联消失；(b) 删除
    /// 节点 → 相连边经 source_id / target_id 外键级联消失 → 这些边若也是产生边则其影子随之
    /// 级联消失。最后再验证一条三层嵌套影子（影子连接画布节点再产生影子）随最上游边删除时
    /// 整套递归坍塌。
    #[test]
    fn test_shadow_cascade_delete() {
        let connection = Connection::open_in_memory().unwrap();

        // 先关闭外键约束以便无序建表与插入数据，所有数据备齐后再开启外键以验证级联删除行为。
        connection
            .execute_batch("PRAGMA foreign_keys = OFF;")
            .unwrap();

        // 建表：node 表与 edge 表互为外键依赖，建表顺序由 SQLite 推迟到 FK 启用后再检查。
        canvas_dao::create_table(&connection).unwrap();
        edge_dao::create_table(&connection).unwrap();
        node_dao::create_table(&connection).unwrap();

        // 插入画布行。
        let canvas = Canvas {
            id: "cascade-canvas-1".to_string(),
            parent_id: None,
            name: "Cascade Canvas".to_string(),
            x: 0.0,
            y: 0.0,
            deleted: false,
            color: String::new(),
        };
        canvas_dao::insert(&connection, &canvas).unwrap();

        // ===== 第 1 阶段：删除产生边 → 影子经 node.shadow_id 外键级联消失 =====
        // 准备边 e1 与影子 s1（s1.shadow_id = e1.id）。
        let source_1 = node("cascade-source-1", "cascade-canvas-1");
        node_dao::insert(&connection, &source_1).unwrap();
        let target_1 = node("cascade-target-1", "cascade-canvas-1");
        node_dao::insert(&connection, &target_1).unwrap();
        let e1 = Edge {
            id: "cascade-edge-1".to_string(),
            canvas_id: "cascade-canvas-1".to_string(),
            source_id: source_1.id.clone(),
            source_port: "right".to_string(),
            target_id: target_1.id.clone(),
            target_port: "left".to_string(),
            title: String::new(),
            description: String::new(),
        };
        edge_dao::insert(&connection, &e1).unwrap();
        let mut s1 = node("cascade-shadow-1", "cascade-canvas-1");
        s1.shadow_id = Some(e1.id.clone());
        node_dao::insert(&connection, &s1).unwrap();

        // 确认数据落库后再开启外键以验证级联删除行为。
        connection
            .execute_batch("PRAGMA foreign_keys = ON;")
            .unwrap();
        assert!(node_dao::select_by_id(&connection, "cascade-shadow-1").unwrap().is_some());

        // 删除产生边 → 影子经 node.shadow_id 外键级联消失。
        edge_dao::delete_by_id(&connection, &e1.id).unwrap();
        assert!(node_dao::select_by_id(&connection, "cascade-shadow-1").unwrap().is_none());

        // ===== 第 2 阶段：删除节点 → 相连边级联 → 影子级联（递归） =====
        // 关闭外键以便继续插入数据。
        connection
            .execute_batch("PRAGMA foreign_keys = OFF;")
            .unwrap();

        // 准备边 e2 与影子 s2（s2.shadow_id = e2.id）。
        let source_2 = node("cascade-source-2", "cascade-canvas-1");
        node_dao::insert(&connection, &source_2).unwrap();
        let target_2 = node("cascade-target-2", "cascade-canvas-1");
        node_dao::insert(&connection, &target_2).unwrap();
        let e2 = Edge {
            id: "cascade-edge-2".to_string(),
            canvas_id: "cascade-canvas-1".to_string(),
            source_id: source_2.id.clone(),
            source_port: "right".to_string(),
            target_id: target_2.id.clone(),
            target_port: "left".to_string(),
            title: String::new(),
            description: String::new(),
        };
        edge_dao::insert(&connection, &e2).unwrap();
        let mut s2 = node("cascade-shadow-2", "cascade-canvas-1");
        s2.shadow_id = Some(e2.id.clone());
        node_dao::insert(&connection, &s2).unwrap();

        // 开启外键后删除 source_2：相连边 e2 经 source_id 外键级联 → 影子 s2 经 shadow_id 外键级联。
        connection
            .execute_batch("PRAGMA foreign_keys = ON;")
            .unwrap();
        node_dao::delete_by_id(&connection, &source_2.id).unwrap();
        assert!(node_dao::select_by_id(&connection, &source_2.id).unwrap().is_none());
        assert!(edge_dao::select_by_id(&connection, &e2.id)
            .unwrap()
            .is_none());
        assert!(node_dao::select_by_id(&connection, "cascade-shadow-2").unwrap().is_none());

        // ===== 第 3 阶段：嵌套影子随最上游边删除时整套递归坍塌 =====
        // 拓扑：e_top（产生顶级影子 s_top）；s_top 通过边 e_inner 连接画布节点 canvas_n；e_inner
        // 产生嵌套影子 s_inner。删除 e_top 后整套：s_top 经 e_top 级联 → e_inner 经 s_top
        // （target_id）级联 → s_inner 经 e_inner 级联。
        connection
            .execute_batch("PRAGMA foreign_keys = OFF;")
            .unwrap();

        // 画布节点 canvas_n 引用子画布 canvas_inner。
        let canvas_inner = Canvas {
            id: "cascade-canvas-inner".to_string(),
            parent_id: None,
            name: "Cascade Canvas Inner".to_string(),
            x: 0.0,
            y: 0.0,
            deleted: false,
            color: String::new(),
        };
        canvas_dao::insert(&connection, &canvas_inner).unwrap();
        let mut canvas_n = node("cascade-canvas-n", "cascade-canvas-1");
        canvas_n.canvas_ref_id = Some(canvas_inner.id.clone());
        node_dao::insert(&connection, &canvas_n).unwrap();
        // 普通节点 target_top 作为 e_top 的目标。
        let target_top = node("cascade-target-top", "cascade-canvas-1");
        node_dao::insert(&connection, &target_top).unwrap();
        // e_top：source_top 普通节点 → canvas_n（画布节点），按新规则在 canvas_inner 内产生顶级影子 s_top。
        let source_top = node("cascade-source-top", "cascade-canvas-1");
        node_dao::insert(&connection, &source_top).unwrap();
        let e_top = Edge {
            id: "cascade-edge-top".to_string(),
            canvas_id: "cascade-canvas-1".to_string(),
            source_id: source_top.id.clone(),
            source_port: "right".to_string(),
            target_id: canvas_n.id.clone(),
            target_port: "left".to_string(),
            title: String::new(),
            description: String::new(),
        };
        edge_dao::insert(&connection, &e_top).unwrap();
        // 顶级影子 s_top（位于 canvas_inner，shadow_id = e_top.id）。
        let mut s_top = node("cascade-shadow-top", "cascade-canvas-inner");
        s_top.shadow_id = Some(e_top.id.clone());
        node_dao::insert(&connection, &s_top).unwrap();
        // canvas_inner 内普通节点 inner_target。
        let inner_target = node("cascade-inner-target", "cascade-canvas-inner");
        node_dao::insert(&connection, &inner_target).unwrap();
        // e_inner：s_top → inner_target（普通节点），按新规则不产生影子——为构造嵌套影子，改用
        // s_top → canvas_n2（画布节点）使其在 canvas_inner 嵌套产生影子 s_inner。
        let mut canvas_n2 = node("cascade-canvas-n2", "cascade-canvas-inner");
        canvas_n2.canvas_ref_id = Some("cascade-canvas-inner2".to_string());
        node_dao::insert(&connection, &canvas_n2).unwrap();
        let canvas_inner2 = Canvas {
            id: "cascade-canvas-inner2".to_string(),
            parent_id: None,
            name: "Cascade Canvas Inner 2".to_string(),
            x: 0.0,
            y: 0.0,
            deleted: false,
            color: String::new(),
        };
        canvas_dao::insert(&connection, &canvas_inner2).unwrap();
        let e_inner = Edge {
            id: "cascade-edge-inner".to_string(),
            canvas_id: "cascade-canvas-inner".to_string(),
            source_id: s_top.id.clone(),
            source_port: "top".to_string(),
            target_id: canvas_n2.id.clone(),
            target_port: "bottom".to_string(),
            title: String::new(),
            description: String::new(),
        };
        edge_dao::insert(&connection, &e_inner).unwrap();
        let mut s_inner = node("cascade-shadow-inner", "cascade-canvas-inner2");
        s_inner.shadow_id = Some(e_inner.id.clone());
        node_dao::insert(&connection, &s_inner).unwrap();

        // 开启外键后删除最上游的 e_top。
        connection
            .execute_batch("PRAGMA foreign_keys = ON;")
            .unwrap();
        edge_dao::delete_by_id(&connection, &e_top.id).unwrap();
        // 顶级影子 s_top 消失；相连边 e_inner 因 s_top 是其 source 端而被 source_id 外键级联；
        // 嵌套影子 s_inner 随之消失。
        assert!(node_dao::select_by_id(&connection, &s_top.id).unwrap().is_none());
        assert!(edge_dao::select_by_id(&connection, &e_inner.id)
            .unwrap()
            .is_none());
        assert!(node_dao::select_by_id(&connection, &s_inner.id).unwrap().is_none());
        // 普通节点 inner_target 与画布节点 canvas_n2 不受影响（它们没有依赖任何被删除的边）。
        assert!(node_dao::select_by_id(&connection, &inner_target.id).unwrap().is_some());
        assert!(node_dao::select_by_id(&connection, &canvas_n2.id).unwrap().is_some());
    }
}