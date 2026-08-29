use rusqlite::Connection;

use crate::business::user_database::entity::Node;
use crate::business::user_database::node::dao;
use crate::business::user_database::node::service::resolve_root;
use crate::business::user_database::node::service::shadow_direction;
use crate::business::user_database::node::vo::NodeVO;
use crate::business::user_database::node::vo::ShadowDirection;
use crate::business::user_database::state;
use crate::error_code::ErrorCode;

/// 返回指定画布内的正常节点或者已经逻辑删除的节点（以 NodeVO 形式）。不产生日志。
///
/// 影子节点的展示数据合并为根本体节点的值，并附带根本体节点 id、根本体节点状态与影子方向。
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

/// 将 Node 转换为 NodeVO：影子节点沿产生边链解析到根本体节点，合并根本体的展示数据
/// （title / sub_title / color）；canvas_ref_id 恒为 None 不合并（出向影子根本体引用的
/// 子画布 id 改由 shadow_origin_canvas_ref_id 单独携带，仅出向影子有值）。
///
/// 影子链由 resolve_root 内部防环保证完整；悬空、端点缺失或成环即数据损坏，
/// 返回 DataCorruption* 错误；根本体节点类型与影子方向矛盾时返回
/// `ErrorCode::DataCorruptionShadowRootTypeMismatch`。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `node`: 待转换的节点。
///
/// # 返回值
/// 返回转换后的节点值对象；数据不一致时返回对应的 DataCorruption* 错误；
/// 数据库错误返回对应的 `ErrorCode`。
pub(crate) fn to_vo(connection: &Connection, node: Node) -> Result<NodeVO, ErrorCode> {
    if node.shadow_id.is_none() {
        return Ok(NodeVO {
            node,
            shadow_origin_id: None,
            shadow_origin_deleted: None,
            shadow_direction: None,
            shadow_origin_canvas_ref_id: None,
        });
    }
    let direction = shadow_direction(connection, &node)?;
    let root = resolve_root(connection, &node)?;
    let root_is_canvas = root.canvas_ref_id.is_some();
    if root_is_canvas != (direction == ShadowDirection::Outflow) {
        return Err(ErrorCode::DataCorruptionShadowRootTypeMismatch {
            shadow_id: node.id.clone(),
            root_id: root.id.clone(),
        });
    }
    let mut merged = node;
    merged.title = root.title.clone();
    merged.sub_title = root.sub_title.clone();
    merged.color = root.color.clone();
    Ok(NodeVO {
        node: merged,
        shadow_origin_id: Some(root.id.clone()),
        shadow_origin_deleted: Some(root.deleted),
        shadow_direction: Some(direction),
        // 出向影子的根本体必为画布节点（上方 DataCorruptionShadowRootTypeMismatch 校验保证），
        // 其 canvas_ref_id 必然为 Some；入向影子根本体是普通节点，无对应子画布。
        shadow_origin_canvas_ref_id: if root_is_canvas { root.canvas_ref_id.clone() } else { None },
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

    /// 构造测试用 Node；调用方按需修改字段。title 默认为 id，便于断言展示数据。
    fn make_node(id: &str, canvas_id: &str) -> Node {
        Node {
            id: id.to_string(),
            canvas_id: canvas_id.to_string(),
            x: 0.0,
            y: 0.0,
            title: id.to_string(),
            sub_title: String::new(),
            canvas_ref_id: None,
            deleted: false,
            color: String::new(),
            shadow_id: None,
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
            name: "list-canvas".to_string(),
            x: 0.0,
            y: 0.0,
            deleted: false,
            color: String::new(),
        };
        canvas_dao::insert(connection, &canvas).unwrap();
        canvas.id
    }

    /// 在指定画布内插一条边记录；FK 关闭，端点可指向尚不存在的节点以构造脏数据。
    fn insert_edge(connection: &Connection, canvas_id: &str, id: &str, source_id: &str, target_id: &str) {
        let edge = Edge {
            id: id.to_string(),
            canvas_id: canvas_id.to_string(),
            source_id: source_id.to_string(),
            source_port: "right".to_string(),
            target_id: target_id.to_string(),
            target_port: "left".to_string(),
            title: String::new(),
            description: String::new(),
        };
        edge_dao::insert(connection, &edge).unwrap();
    }

    /// 非影子节点 to_vo 后三个 shadow_* 字段均为 None。
    #[test]
    fn test_to_vo_plain_node_no_shadow_fields() {
        let connection = Connection::open_in_memory().unwrap();
        let canvas_id = setup_canvas(&connection);
        let plain = make_node("plain-1", &canvas_id);
        node_dao::insert(&connection, &plain).unwrap();

        let vo = to_vo(&connection, plain.clone()).unwrap();
        assert_eq!(vo.id, plain.id);
        assert!(vo.shadow_origin_id.is_none());
        assert!(vo.shadow_origin_deleted.is_none());
        assert!(vo.shadow_direction.is_none());
        assert!(vo.shadow_origin_canvas_ref_id.is_none());
    }

    /// 单层入向影子：title/sub_title/color 沿产生边链合并自根本体；
    /// shadow_origin_id / shadow_origin_deleted / shadow_direction 正确填入。
    #[test]
    fn test_to_vo_inflow_shadow_merges_origin() {
        let connection = Connection::open_in_memory().unwrap();
        let canvas_id = setup_canvas(&connection);
        let mut origin = make_node("origin-x", &canvas_id);
        origin.title = "origin-x-title".to_string();
        origin.sub_title = "origin-x-sub".to_string();
        origin.color = "{\"fill\":\"#ff0000\"}".to_string();
        node_dao::insert(&connection, &origin).unwrap();
        let target = make_node("canvas-b", &canvas_id);
        node_dao::insert(&connection, &target).unwrap();
        insert_edge(&connection, &canvas_id, "edge-xb", &origin.id, &target.id);
        let mut shadow = make_node("shadow-in", &canvas_id);
        shadow.shadow_id = Some("edge-xb".to_string());
        node_dao::insert(&connection, &shadow).unwrap();

        let vo = to_vo(&connection, shadow).unwrap();
        assert_eq!(vo.title, "origin-x-title");
        assert_eq!(vo.sub_title, "origin-x-sub");
        assert_eq!(vo.color, "{\"fill\":\"#ff0000\"}");
        assert_eq!(vo.shadow_origin_id.as_deref(), Some("origin-x"));
        assert_eq!(vo.shadow_origin_deleted, Some(false));
        assert_eq!(vo.shadow_direction, Some(ShadowDirection::Inflow));
        assert!(vo.shadow_origin_canvas_ref_id.is_none());
    }

    /// 嵌套入向影子沿产生边链合并到根本体：S2.shadow_id=edge-inner（S1 → canvas_b1）→
    /// resolve_root 沿 S1 的产生边 edge-xb 递归到 origin-x，展示数据合并自 origin-x。
    #[test]
    fn test_to_vo_nested_inflow_shadow_merges_to_origin() {
        let connection = Connection::open_in_memory().unwrap();
        let canvas_id = setup_canvas(&connection);
        let mut origin = make_node("origin-x", &canvas_id);
        origin.title = "root-title".to_string();
        node_dao::insert(&connection, &origin).unwrap();
        let canvas_b = make_node("canvas-b", &canvas_id);
        node_dao::insert(&connection, &canvas_b).unwrap();
        insert_edge(&connection, &canvas_id, "edge-xb", &origin.id, &canvas_b.id);
        let mut s1 = make_node("shadow-s1", &canvas_id);
        s1.shadow_id = Some("edge-xb".to_string());
        node_dao::insert(&connection, &s1).unwrap();
        let canvas_b1 = make_node("canvas-b1", &canvas_id);
        node_dao::insert(&connection, &canvas_b1).unwrap();
        insert_edge(&connection, &canvas_id, "edge-inner", &s1.id, &canvas_b1.id);
        let mut s2 = make_node("shadow-s2", &canvas_id);
        s2.shadow_id = Some("edge-inner".to_string());
        node_dao::insert(&connection, &s2).unwrap();

        let vo = to_vo(&connection, s2).unwrap();
        assert_eq!(vo.title, "root-title");
        assert_eq!(vo.shadow_origin_id.as_deref(), Some("origin-x"));
        assert_eq!(vo.shadow_direction, Some(ShadowDirection::Inflow));
        assert!(vo.shadow_origin_canvas_ref_id.is_none());
    }

    /// 出向影子：展示数据合并自根本体画布节点；shadow_origin_canvas_ref_id 填入根本体引用的
    /// 子画布 id；canvas_ref_id 保持不合并（恒为 None）。
    #[test]
    fn test_to_vo_outflow_shadow_carries_canvas_ref_id() {
        let connection = Connection::open_in_memory().unwrap();
        let canvas_id = setup_canvas(&connection);
        // 根本体：画布节点 B2（引用子画布 sub-canvas-b2）。
        let mut origin = make_node("origin-b2", &canvas_id);
        origin.title = "origin-b2-title".to_string();
        origin.canvas_ref_id = Some("sub-canvas-b2".to_string());
        node_dao::insert(&connection, &origin).unwrap();
        // 产生边 source 端为画布节点 B1：shadow_direction 推导为 Outflow，
        // resolve_root 沿 target 侧终止于 origin-b2。
        let mut source_canvas = make_node("source-b1", &canvas_id);
        source_canvas.canvas_ref_id = Some("sub-canvas-b1".to_string());
        node_dao::insert(&connection, &source_canvas).unwrap();
        insert_edge(&connection, &canvas_id, "edge-b1b2", &source_canvas.id, &origin.id);
        let mut shadow = make_node("shadow-out", &canvas_id);
        shadow.shadow_id = Some("edge-b1b2".to_string());
        node_dao::insert(&connection, &shadow).unwrap();

        let vo = to_vo(&connection, shadow).unwrap();
        assert_eq!(vo.title, "origin-b2-title");
        assert_eq!(vo.shadow_origin_id.as_deref(), Some("origin-b2"));
        assert_eq!(vo.shadow_direction, Some(ShadowDirection::Outflow));
        assert_eq!(vo.shadow_origin_canvas_ref_id.as_deref(), Some("sub-canvas-b2"));
        assert!(vo.canvas_ref_id.is_none());
    }

    /// 根本体类型与影子方向矛盾：影子产生边源端是普通节点（Inflow），但沿 source 侧递归到的
    /// 根是画布节点——脏数据构造，期望 DataCorruptionShadowRootTypeMismatch。
    #[test]
    fn test_to_vo_root_type_mismatch_returns_data_corruption() {
        let connection = Connection::open_in_memory().unwrap();
        let canvas_id = setup_canvas(&connection);
        // 边 source=plain_x（普通节点）→ target=canvas_root（画布节点）→ Inflow；
        // 但 FK OFF 直接把 canvas_root.canvas_ref_id 设空，模拟"沿 source 侧递归到的根是画布节点"的矛盾。
        let plain_x = make_node("plain-x", &canvas_id);
        node_dao::insert(&connection, &plain_x).unwrap();
        let mut canvas_root = make_node("canvas-root", &canvas_id);
        canvas_root.canvas_ref_id = None; // 不是画布节点
        node_dao::insert(&connection, &canvas_root).unwrap();
        // 需要有另一个画布节点，使其 canvas_ref_id 非空，模拟矛盾——
        // 这里直接利用现有 plain_x 的来源链无法满足矛盾场景；改用另一种脏数据构造：
        // 影子产生边 source 端 canvas_ref_id 为 None（Inflow），但 target 沿本体链会解析到一个画布节点。
        // 构造：shadow_id=edge-1，edge-1.source=plain（Inflow 应走 source 侧），
        // 但 plain 自身不是画布——为构造矛盾，让 source 节点本身被设为画布节点（canvas_ref_id 实际为 None）。
        // 简化路径：直接让 shadow 本身的方向被 shadow_direction 推断为 Inflow（source 普通），
        // 而 resolve_root 沿 source 侧递归到尽头仍不是根本体——这要求源节点后续有自己指向另一节点的产生边。
        // 这里用更直接的方式构造：源端是画布节点（Outflow），但其本体链解析到的根本体却是普通节点。
        // 重新规划拓扑：
        //   - 画布节点 B1（canvas_ref_id=Some）→ 普通节点 target；target 处产生 Outflow 影子 S。
        //   - 但使 target.canvas_ref_id 被改写为 None 后再用作影子本体——这与 resolve_root 的递归逻辑无关，
        //     resolve_root 只会沿 source/target 端点解析。
        // 最终方案：构造一个最简单的"方向与根本体类型不一致"场景——
        //   - 影子 S 的产生边 source 是普通节点 plain（无 canvas_ref_id）→ shadow_direction 推 Inflow；
        //   - 边 target 是一串影子链，最终落到一个画布节点 canvas_root 上 → resolve_root 走 target 侧终止于画布节点。
        //   - 这要求 source 不是画布节点，但目标端是出向影子链；resolve_root 在 source.canvas_ref_id 为 None 时
        //     走 source 侧，所以会终止于 plain，与"根本体是画布节点"矛盾。
        //
        // 重新对齐 resolve_root 的递归条件：当 source.canvas_ref_id 为 None（Inflow）时走 source 侧，
        // 终止于 plain，不会经过 target 侧。要构造矛盾，需要让 shadow_direction 推 Inflow 而 resolve_root
        // 走 source 侧后命中一个画布节点。
        // 因此让 source 本身是画布节点（canvas_ref_id 非空）→ shadow_direction 推 Outflow，
        // resolve_root 走 target 侧终止于普通节点——这才是与"Outflow 应对应画布节点根本体"矛盾的脏数据。
        //
        // 最终采用：shadow_direction 推 Outflow（source 是画布节点），但 resolve_root 走 target 侧
        // 终止于一个普通节点，从而 DataCorruptionShadowRootTypeMismatch 触发。
        insert_edge(&connection, &canvas_id, "edge-1", &plain_x.id, &canvas_root.id);
        // 把 plain_x 改成画布节点（canvas_ref_id 非空），以让 shadow_direction 推 Outflow。
        let mut plain_x_updated = plain_x.clone();
        plain_x_updated.canvas_ref_id = Some("sub-canvas-x".to_string());
        node_dao::update(&connection, &plain_x_updated).unwrap();
        // canvas_root 保持 canvas_ref_id = None，是普通节点；它就是 resolve_root 走 target 侧后
        // 终止的"根本体"，与 Outflow 的"根本体应是画布节点"矛盾。
        let mut shadow = make_node("shadow-bad", &canvas_id);
        shadow.shadow_id = Some("edge-1".to_string());
        node_dao::insert(&connection, &shadow).unwrap();

        let err = to_vo(&connection, shadow).unwrap_err();
        assert!(matches!(
            err,
            ErrorCode::DataCorruptionShadowRootTypeMismatch { .. }
        ));
    }

    /// 影子产生边缺失时 to_vo 返回 DataCorruptionDanglingShadow（由 shadow_direction 触发）。
    #[test]
    fn test_to_vo_dangling_returns_data_corruption_dangling_shadow() {
        let connection = Connection::open_in_memory().unwrap();
        let canvas_id = setup_canvas(&connection);
        let mut shadow = make_node("shadow-1", &canvas_id);
        shadow.shadow_id = Some("no-such-edge-id".to_string());
        node_dao::insert(&connection, &shadow).unwrap();

        let err = to_vo(&connection, shadow).unwrap_err();
        assert!(matches!(
            err,
            ErrorCode::DataCorruptionDanglingShadow { .. }
        ));
    }

    /// 影子链成环时 to_vo 返回 DataCorruptionShadowChainCycle（由 resolve_root 触发）。
    #[test]
    fn test_to_vo_cycle_returns_chain_cycle() {
        let connection = Connection::open_in_memory().unwrap();
        let canvas_id = setup_canvas(&connection);
        let mut s_a = make_node("s-a", &canvas_id);
        let mut s_b = make_node("s-b", &canvas_id);
        s_a.shadow_id = Some("edge-e1".to_string());
        s_b.shadow_id = Some("edge-e2".to_string());
        node_dao::insert(&connection, &s_a).unwrap();
        node_dao::insert(&connection, &s_b).unwrap();
        // 环：e1.source=s_b（普通节点侧），e2.source=s_a → resolve_root(s_a) 命中 s_a 自身，触发环。
        insert_edge(&connection, &canvas_id, "edge-e1", &s_b.id, &s_a.id);
        insert_edge(&connection, &canvas_id, "edge-e2", &s_a.id, &s_b.id);

        let err = to_vo(&connection, s_a).unwrap_err();
        assert!(matches!(
            err,
            ErrorCode::DataCorruptionShadowChainCycle { .. }
        ));
    }
}
