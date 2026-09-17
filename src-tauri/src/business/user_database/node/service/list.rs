use rusqlite::Connection;

use crate::business::user_database::entity::Node;
use crate::business::user_database::node::dao;
use crate::business::user_database::node::vo::NodeVO;
use crate::business::user_database::shadow::service::{resolve_origin, shadow_direction};
use crate::business::user_database::shadow::vo::ShadowDirection;
use crate::business::user_database::state;
use crate::error_code::ErrorCode;

/// 返回指定画布内的正常节点或者已经逻辑删除的节点（以 NodeVO 形式）。不产生日志。
///
/// 影子节点的展示数据合并为本体节点的值，并附带本体节点 id、本体节点状态与影子方向。
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

/// 将 Node 转换为 NodeVO：影子节点沿产生边链解析到本体节点，合并本体的展示数据
/// （title / subtitle / color）；canvas_ref_id 恒为 None 不合并（出向影子本体引用的
/// 子画布 id 改由 shadow_origin_canvas_ref_id 单独携带，仅出向影子有值）。
///
/// 影子链由 resolve_origin 内部防环保证完整；悬空、端点缺失或成环即数据损坏，
/// 返回 DataCorruption* 错误；本体节点类型与影子方向矛盾时返回
/// `ErrorCode::DataCorruptionShadowOriginTypeMismatch`。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `node`: 待转换的节点。
///
/// # 返回值
/// 返回转换后的节点值对象；数据不一致时返回对应的 DataCorruption* 错误；
/// 数据库错误返回对应的 `ErrorCode`。
pub(crate) fn to_vo(connection: &Connection, node: Node) -> Result<NodeVO, ErrorCode> {
    if node.shadow_producing_edge_id.is_none() {
        return Ok(NodeVO {
            node,
            shadow_origin_id: None,
            shadow_origin_deleted: None,
            shadow_direction: None,
            shadow_origin_canvas_ref_id: None,
        });
    }
    let direction = shadow_direction(connection, &node)?;
    let origin = resolve_origin(connection, &node)?;
    let origin_is_canvas = origin.canvas_ref_id.is_some();
    if origin_is_canvas != (direction == ShadowDirection::Outflow) {
        return Err(ErrorCode::DataCorruptionShadowOriginTypeMismatch {
            shadow_id: node.id.clone(),
            origin_id: origin.id.clone(),
        });
    }
    let mut merged = node;
    merged.title = origin.title.clone();
    merged.subtitle = origin.subtitle.clone();
    merged.color = origin.color.clone();
    Ok(NodeVO {
        node: merged,
        shadow_origin_id: Some(origin.id.clone()),
        shadow_origin_deleted: Some(origin.deleted),
        shadow_direction: Some(direction),
        // 出向影子的本体必为画布数据节点（上方 DataCorruptionShadowOriginTypeMismatch 校验保证），
        // 其 canvas_ref_id 必然为 Some；入向影子本体是数据节点，无对应子画布。
        shadow_origin_canvas_ref_id: if origin_is_canvas { origin.canvas_ref_id.clone() } else { None },
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
            source_handle: "right".to_string(),
            target_id: target_id.to_string(),
            target_handle: "left".to_string(),
            title: String::new(),
            description: String::new(),
        };
        edge_dao::insert(connection, &edge).unwrap();
    }

    /// 非影子节点 to_vo 后三个 shadow_* 字段均为 None。
    #[test]
    fn test_to_vo_data_node_no_shadow_fields() {
        let connection = Connection::open_in_memory().unwrap();
        let canvas_id = setup_canvas(&connection);
        let data = make_node("data-1", &canvas_id);
        node_dao::insert(&connection, &data).unwrap();

        let vo = to_vo(&connection, data.clone()).unwrap();
        assert_eq!(vo.id, data.id);
        assert!(vo.shadow_origin_id.is_none());
        assert!(vo.shadow_origin_deleted.is_none());
        assert!(vo.shadow_direction.is_none());
        assert!(vo.shadow_origin_canvas_ref_id.is_none());
    }

    /// 单层入向影子：title/subtitle/color 沿产生边链合并自本体；
    /// shadow_origin_id / shadow_origin_deleted / shadow_direction 正确填入。
    #[test]
    fn test_to_vo_inflow_shadow_merges_origin() {
        let connection = Connection::open_in_memory().unwrap();
        let canvas_id = setup_canvas(&connection);
        let mut origin = make_node("origin-x", &canvas_id);
        origin.title = "origin-x-title".to_string();
        origin.subtitle = "origin-x-sub".to_string();
        origin.color = "{\"fill\":\"#ff0000\"}".to_string();
        node_dao::insert(&connection, &origin).unwrap();
        let target = make_node("canvas-b", &canvas_id);
        node_dao::insert(&connection, &target).unwrap();
        insert_edge(&connection, &canvas_id, "edge-xb", &origin.id, &target.id);
        let mut shadow = make_node("shadow-in", &canvas_id);
        shadow.shadow_producing_edge_id = Some("edge-xb".to_string());
        node_dao::insert(&connection, &shadow).unwrap();

        let vo = to_vo(&connection, shadow).unwrap();
        assert_eq!(vo.title, "origin-x-title");
        assert_eq!(vo.subtitle, "origin-x-sub");
        assert_eq!(vo.color, "{\"fill\":\"#ff0000\"}");
        assert_eq!(vo.shadow_origin_id.as_deref(), Some("origin-x"));
        assert_eq!(vo.shadow_origin_deleted, Some(false));
        assert_eq!(vo.shadow_direction, Some(ShadowDirection::Inflow));
        assert!(vo.shadow_origin_canvas_ref_id.is_none());
    }

    /// 嵌套入向影子沿产生边链合并到本体：S2.shadow_producing_edge_id=edge-inner（S1 → canvas_b1）→
    /// resolve_origin 沿 S1 的产生边 edge-xb 递归到 origin-x，展示数据合并自 origin-x。
    #[test]
    fn test_to_vo_nested_inflow_shadow_merges_to_origin() {
        let connection = Connection::open_in_memory().unwrap();
        let canvas_id = setup_canvas(&connection);
        let mut origin = make_node("origin-x", &canvas_id);
        origin.title = "origin-title".to_string();
        node_dao::insert(&connection, &origin).unwrap();
        let canvas_b = make_node("canvas-b", &canvas_id);
        node_dao::insert(&connection, &canvas_b).unwrap();
        insert_edge(&connection, &canvas_id, "edge-xb", &origin.id, &canvas_b.id);
        let mut s1 = make_node("shadow-s1", &canvas_id);
        s1.shadow_producing_edge_id = Some("edge-xb".to_string());
        node_dao::insert(&connection, &s1).unwrap();
        let canvas_b1 = make_node("canvas-b1", &canvas_id);
        node_dao::insert(&connection, &canvas_b1).unwrap();
        insert_edge(&connection, &canvas_id, "edge-inner", &s1.id, &canvas_b1.id);
        let mut s2 = make_node("shadow-s2", &canvas_id);
        s2.shadow_producing_edge_id = Some("edge-inner".to_string());
        node_dao::insert(&connection, &s2).unwrap();

        let vo = to_vo(&connection, s2).unwrap();
        assert_eq!(vo.title, "origin-title");
        assert_eq!(vo.shadow_origin_id.as_deref(), Some("origin-x"));
        assert_eq!(vo.shadow_direction, Some(ShadowDirection::Inflow));
        assert!(vo.shadow_origin_canvas_ref_id.is_none());
    }

    /// 出向影子：展示数据合并自本体画布数据节点；shadow_origin_canvas_ref_id 填入本体引用的
    /// 子画布 id；canvas_ref_id 保持不合并（恒为 None）。
    #[test]
    fn test_to_vo_outflow_shadow_carries_canvas_ref_id() {
        let connection = Connection::open_in_memory().unwrap();
        let canvas_id = setup_canvas(&connection);
        // 本体：画布数据节点 B2（引用子画布 sub-canvas-b2）。
        let mut origin = make_node("origin-b2", &canvas_id);
        origin.title = "origin-b2-title".to_string();
        origin.canvas_ref_id = Some("sub-canvas-b2".to_string());
        node_dao::insert(&connection, &origin).unwrap();
        // 产生边 source 端为画布数据节点 B1：shadow_direction 推导为 Outflow，
        // resolve_origin 沿 target 侧终止于 origin-b2。
        let mut source_canvas = make_node("source-b1", &canvas_id);
        source_canvas.canvas_ref_id = Some("sub-canvas-b1".to_string());
        node_dao::insert(&connection, &source_canvas).unwrap();
        insert_edge(&connection, &canvas_id, "edge-b1b2", &source_canvas.id, &origin.id);
        let mut shadow = make_node("shadow-out", &canvas_id);
        shadow.shadow_producing_edge_id = Some("edge-b1b2".to_string());
        node_dao::insert(&connection, &shadow).unwrap();

        let vo = to_vo(&connection, shadow).unwrap();
        assert_eq!(vo.title, "origin-b2-title");
        assert_eq!(vo.shadow_origin_id.as_deref(), Some("origin-b2"));
        assert_eq!(vo.shadow_direction, Some(ShadowDirection::Outflow));
        assert_eq!(vo.shadow_origin_canvas_ref_id.as_deref(), Some("sub-canvas-b2"));
        assert!(vo.canvas_ref_id.is_none());
    }

    /// 本体类型与影子方向矛盾：影子产生边源端是数据节点（Inflow），但沿 source 侧递归到的
    /// 根是画布数据节点——脏数据构造，期望 DataCorruptionShadowOriginTypeMismatch。
    #[test]
    fn test_to_vo_origin_type_mismatch_returns_data_corruption() {
        let connection = Connection::open_in_memory().unwrap();
        let canvas_id = setup_canvas(&connection);
        // 边 source=data_x（数据节点）→ target=canvas_root（画布数据节点）→ Inflow；
        // 但 FK OFF 直接把 canvas_root.canvas_ref_id 设空，模拟"沿 source 侧递归到的根是画布数据节点"的矛盾。
        let data_x = make_node("data-x", &canvas_id);
        node_dao::insert(&connection, &data_x).unwrap();
        let mut canvas_root = make_node("canvas-root", &canvas_id);
        canvas_root.canvas_ref_id = None; // 不是画布数据节点
        node_dao::insert(&connection, &canvas_root).unwrap();
        // 需要有另一个画布数据节点，使其 canvas_ref_id 非空，模拟矛盾——
        // 这里直接利用现有 data_x 的产生边链无法满足矛盾场景；改用另一种脏数据构造：
        // 影子产生边 source 端 canvas_ref_id 为 None（Inflow），但 target 沿产生边链会解析到一个画布数据节点。
        // 构造：影子指向产生边 edge-1，edge-1.source=data（Inflow 应走 source 侧），
        // 但 data 自身不是画布——为构造矛盾，让 source 节点本身被设为画布数据节点（canvas_ref_id 实际为 None）。
        // 简化路径：直接让 shadow 本身的方向被 shadow_direction 推断为 Inflow（source 数据节点），
        // 而 resolve_origin 沿 source 侧递归到尽头仍不是本体——这要求源节点后续有自己指向另一节点的产生边。
        // 这里用更直接的方式构造：源端是画布数据节点（Outflow），但沿其产生边链解析到的本体却是数据节点。
        // 重新规划拓扑：
        //   - 画布数据节点 B1（canvas_ref_id=Some）→ 数据节点 target；target 处产生 Outflow 影子 S。
        //   - 但使 target.canvas_ref_id 被改写为 None 后再用作影子自身——这与 resolve_origin 的递归逻辑无关，
        //     resolve_origin 只会沿 source/target 端点解析。
        // 最终方案：构造一个最简单的"方向与本体类型不一致"场景——
        //   - 影子 S 的产生边 source 是数据节点 data（无 canvas_ref_id）→ shadow_direction 推 Inflow；
        //   - 边 target 是一串影子链，最终落到一个画布数据节点 canvas_root 上 → resolve_origin 走 target 侧终止于画布数据节点。
        //   - 这要求 source 不是画布数据节点，但目标端是出向影子链；resolve_origin 在 source.canvas_ref_id 为 None 时
        //     走 source 侧，所以会终止于 data，与"本体是画布数据节点"矛盾。
        //
        // 重新对齐 resolve_origin 的递归条件：当 source.canvas_ref_id 为 None（Inflow）时走 source 侧，
        // 终止于 data，不会经过 target 侧。要构造矛盾，需要让 shadow_direction 推 Inflow 而 resolve_origin
        // 走 source 侧后命中一个画布数据节点。
        // 因此让 source 本身是画布数据节点（canvas_ref_id 非空）→ shadow_direction 推 Outflow，
        // resolve_origin 走 target 侧终止于数据节点——这才是与"Outflow 应对应画布数据节点本体"矛盾的脏数据。
        //
        // 最终采用：shadow_direction 推 Outflow（source 是画布数据节点），但 resolve_origin 走 target 侧
        // 终止于一个数据节点，从而 DataCorruptionShadowOriginTypeMismatch 触发。
        insert_edge(&connection, &canvas_id, "edge-1", &data_x.id, &canvas_root.id);
        // 把 data_x 改成画布数据节点（canvas_ref_id 非空），以让 shadow_direction 推 Outflow。
        let mut data_x_updated = data_x.clone();
        data_x_updated.canvas_ref_id = Some("sub-canvas-x".to_string());
        node_dao::update(&connection, &data_x_updated).unwrap();
        // canvas_root 保持 canvas_ref_id = None，是数据节点；它就是 resolve_origin 走 target 侧后
        // 终止的"本体"，与 Outflow 的"本体应是画布数据节点"矛盾。
        let mut shadow = make_node("shadow-bad", &canvas_id);
        shadow.shadow_producing_edge_id = Some("edge-1".to_string());
        node_dao::insert(&connection, &shadow).unwrap();

        let err = to_vo(&connection, shadow).unwrap_err();
        assert!(matches!(
            err,
            ErrorCode::DataCorruptionShadowOriginTypeMismatch { .. }
        ));
    }

    /// 影子产生边缺失时 to_vo 返回 DataCorruptionDanglingShadow（由 shadow_direction 触发）。
    #[test]
    fn test_to_vo_dangling_returns_data_corruption_dangling_shadow() {
        let connection = Connection::open_in_memory().unwrap();
        let canvas_id = setup_canvas(&connection);
        let mut shadow = make_node("shadow-1", &canvas_id);
        shadow.shadow_producing_edge_id = Some("no-such-edge-id".to_string());
        node_dao::insert(&connection, &shadow).unwrap();

        let err = to_vo(&connection, shadow).unwrap_err();
        assert!(matches!(
            err,
            ErrorCode::DataCorruptionDanglingShadow { .. }
        ));
    }

    /// 影子链成环时 to_vo 返回 DataCorruptionShadowChainCycle（由 resolve_origin 触发）。
    #[test]
    fn test_to_vo_cycle_returns_chain_cycle() {
        let connection = Connection::open_in_memory().unwrap();
        let canvas_id = setup_canvas(&connection);
        let mut s_a = make_node("s-a", &canvas_id);
        let mut s_b = make_node("s-b", &canvas_id);
        s_a.shadow_producing_edge_id = Some("edge-e1".to_string());
        s_b.shadow_producing_edge_id = Some("edge-e2".to_string());
        node_dao::insert(&connection, &s_a).unwrap();
        node_dao::insert(&connection, &s_b).unwrap();
        // 环：e1.source=s_b（数据节点侧），e2.source=s_a → resolve_origin(s_a) 命中 s_a 自身，触发环。
        insert_edge(&connection, &canvas_id, "edge-e1", &s_b.id, &s_a.id);
        insert_edge(&connection, &canvas_id, "edge-e2", &s_a.id, &s_b.id);

        let err = to_vo(&connection, s_a).unwrap_err();
        assert!(matches!(
            err,
            ErrorCode::DataCorruptionShadowChainCycle { .. }
        ));
    }
}
