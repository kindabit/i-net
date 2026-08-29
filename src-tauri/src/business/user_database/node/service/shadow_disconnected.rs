use rusqlite::Connection;

use crate::business::user_database::edge;
use crate::business::user_database::entity::Edge;
use crate::business::user_database::entity::Node;
use crate::business::user_database::node::dao;
use crate::business::user_database::node::service::shadow_direction;
use crate::business::user_database::node::vo::ShadowDirection;
use crate::error_code::ErrorCode;

/// 收集一条边被物理删除时，因其产生的影子节点级联删除而失去连接的节点标题（去重）。
///
/// 级联路径：边删除 → 其产生的影子经 node.shadow_id 外键级联删除 → 影子的相连边经
/// edge.source_id/target_id 外键级联删除 → 这些边若也是产生边，其影子递归级联删除。
/// 本函数在删除发生前沿同一路径预收集：找出边产生的影子，收集影子相连边另一端
/// （邻居，必然是非影子节点）的标题，并对每条相连边递归同一过程。
///
/// 防御性校验（均返回 DataCorruption* 错误，绝不静默）：
/// - 按连接规则应当产生影子的边查不到影子 → DataCorruptionMissingShadow
/// - 影子存在与其方向不匹配的边 → DataCorruptionShadowEdgeDirectionMismatch
/// - 相连边另一端节点缺失 → DataCorruptionEdgeEndpointMissing
/// - 邻居是影子（影子之间不允许相连）→ DataCorruptionShadowNeighborIsShadow
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `edge`: 即将被物理删除的边。
///
/// # 返回值
/// 返回受影响节点的标题列表（去重）；数据不一致时返回对应的 `DataCorruption*` 错误。
pub fn collect_edge_disconnected(
    connection: &Connection,
    edge: &Edge,
) -> Result<Vec<String>, ErrorCode> {
    let mut affected: Vec<String> = Vec::new();
    collect_edge_into(connection, edge, &mut affected)?;
    Ok(affected)
}

/// 收集单条边级联删除产生的断连：查该边产生的影子；查不到时按连接规则校验
/// 是否本应产生影子（应产生而缺失 → DataCorruptionMissingShadow）；查到则进入影子收集。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `edge`: 待检查的边。
/// - `affected`: 受影响节点标题的累积列表（去重写入）。
///
/// # 返回值
/// 成功时返回 `Ok(())`；数据不一致时返回对应的 `DataCorruption*` 错误；
/// 数据库错误返回对应的 `ErrorCode`。
fn collect_edge_into(
    connection: &Connection,
    edge: &Edge,
    affected: &mut Vec<String>,
) -> Result<(), ErrorCode> {
    let Some(shadow) = dao::select_by_producing_edge_id(connection, &edge.id)? else {
        let source = dao::select_by_id(connection, &edge.source_id)?.ok_or_else(|| {
            ErrorCode::DataCorruptionEdgeEndpointMissing {
                edge_id: edge.id.clone(),
                node_id: edge.source_id.clone(),
            }
        })?;
        let target = dao::select_by_id(connection, &edge.target_id)?.ok_or_else(|| {
            ErrorCode::DataCorruptionEdgeEndpointMissing {
                edge_id: edge.id.clone(),
                node_id: edge.target_id.clone(),
            }
        })?;
        if should_produce_shadow(connection, &source, &target)? {
            return Err(ErrorCode::DataCorruptionMissingShadow {
                edge_id: edge.id.clone(),
            });
        }
        return Ok(());
    };
    collect_shadow_into(connection, &shadow, affected)
}

/// 判断按连接规则一条边是否应当产生影子：源端是画布节点、目标端是画布节点
/// 或目标端是出向影子时应当产生，其余连接（普通→普通、入向影子→普通）不产生。
/// 目标端是影子时方向必然可推导，推导失败返回对应的 DataCorruption* 错误。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `source`: 边的源节点。
/// - `target`: 边的目标节点。
///
/// # 返回值
/// 返回该边是否应当产生影子；数据不一致时返回对应的 `DataCorruption*` 错误。
fn should_produce_shadow(
    connection: &Connection,
    source: &Node,
    target: &Node,
) -> Result<bool, ErrorCode> {
    if source.canvas_ref_id.is_some() || target.canvas_ref_id.is_some() {
        return Ok(true);
    }
    if target.shadow_id.is_some() {
        return Ok(shadow_direction(connection, target)? == ShadowDirection::Outflow);
    }
    Ok(false)
}

/// 收集影子被删除时其所在画布内失去连接的邻居标题，并对每条相连边递归 collect_edge_into。
/// 入向影子只收集其出边的目标（存在入边即方向不匹配）；出向影子只收集其入边的源
/// （存在出边即方向不匹配）。邻居必然不是影子；邻居标题去重后并入 affected。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `shadow`: 即将被级联删除的影子节点。
/// - `affected`: 受影响节点标题的累积列表（去重写入）。
///
/// # 返回值
/// 成功时返回 `Ok(())`；数据不一致时返回对应的 `DataCorruption*` 错误；
/// 数据库错误返回对应的 `ErrorCode`。
fn collect_shadow_into(
    connection: &Connection,
    shadow: &Node,
    affected: &mut Vec<String>,
) -> Result<(), ErrorCode> {
    let direction = shadow_direction(connection, shadow)?;
    let edges = edge::dao::select_by_canvas_id(connection, &shadow.canvas_id)?;
    for edge_record in &edges {
        let neighbor_id: &str = match direction {
            ShadowDirection::Inflow => {
                if edge_record.source_id == shadow.id {
                    &edge_record.target_id
                } else if edge_record.target_id == shadow.id {
                    return Err(ErrorCode::DataCorruptionShadowEdgeDirectionMismatch {
                        shadow_id: shadow.id.clone(),
                        edge_id: edge_record.id.clone(),
                    });
                } else {
                    continue;
                }
            }
            ShadowDirection::Outflow => {
                if edge_record.target_id == shadow.id {
                    &edge_record.source_id
                } else if edge_record.source_id == shadow.id {
                    return Err(ErrorCode::DataCorruptionShadowEdgeDirectionMismatch {
                        shadow_id: shadow.id.clone(),
                        edge_id: edge_record.id.clone(),
                    });
                } else {
                    continue;
                }
            }
        };
        let neighbor = dao::select_by_id(connection, neighbor_id)?.ok_or_else(|| {
            ErrorCode::DataCorruptionEdgeEndpointMissing {
                edge_id: edge_record.id.clone(),
                node_id: neighbor_id.to_string(),
            }
        })?;
        if neighbor.shadow_id.is_some() {
            return Err(ErrorCode::DataCorruptionShadowNeighborIsShadow {
                shadow_id: shadow.id.clone(),
                neighbor_id: neighbor.id.clone(),
            });
        }
        if !affected.contains(&neighbor.title) {
            affected.push(neighbor.title.clone());
        }
        collect_edge_into(connection, edge_record, affected)?;
    }
    Ok(())
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
            name: "shadow-disconnected-canvas".to_string(),
            x: 0.0,
            y: 0.0,
            deleted: false,
            color: String::new(),
        };
        canvas_dao::insert(connection, &canvas).unwrap();
        canvas.id
    }

    /// 在指定画布内插一条边记录；FK 关闭，端点可以指向尚不存在的节点以构造脏数据。
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

    /// 入向影子有两条出边连接到普通节点 M1、M2 时，断连收集返回两者标题并去重。
    #[test]
    fn test_collect_inflow_shadow_neighbors() {
        let connection = Connection::open_in_memory().unwrap();
        let canvas_id = setup_canvas(&connection);
        let source = make_node("source-plain", &canvas_id);
        node_dao::insert(&connection, &source).unwrap();
        let canvas_b = make_node("canvas-b", &canvas_id);
        node_dao::insert(&connection, &canvas_b).unwrap();
        insert_edge(&connection, &canvas_id, "edge-prod", &source.id, &canvas_b.id);
        let mut shadow = make_node("shadow-in", &canvas_id);
        shadow.title = "shadow-in".to_string();
        shadow.shadow_id = Some("edge-prod".to_string());
        node_dao::insert(&connection, &shadow).unwrap();

        // 影子所在画布内建两条出边，邻居分别设置独立 title 用于断言去重。
        let mut m1 = make_node("m1", &canvas_id);
        m1.title = "m1-title".to_string();
        node_dao::insert(&connection, &m1).unwrap();
        let mut m2 = make_node("m2", &canvas_id);
        m2.title = "m2-title".to_string();
        node_dao::insert(&connection, &m2).unwrap();
        insert_edge(&connection, &canvas_id, "edge-s-m1", &shadow.id, &m1.id);
        insert_edge(&connection, &canvas_id, "edge-s-m2", &shadow.id, &m2.id);

        let prod_edge = edge_dao::select_by_id(&connection, "edge-prod").unwrap().unwrap();
        let affected = collect_edge_disconnected(&connection, &prod_edge).unwrap();
        assert_eq!(affected.len(), 2);
        assert!(affected.contains(&"m1-title".to_string()));
        assert!(affected.contains(&"m2-title".to_string()));
    }

    /// 出向影子有一条入边来自普通节点 M 时，断连收集返回 M 的 title。
    /// 新规则下出向影子只能是画布节点的影子（产生边源端 = 画布节点），且任何"进入"出向影子的边
    /// （普通→出向影子 / 画布→出向影子）都会再触发嵌套影子创建，所以这里同步创建嵌套影子。
    #[test]
    fn test_collect_outflow_shadow_neighbors() {
        let connection = Connection::open_in_memory().unwrap();
        let canvas_id = setup_canvas(&connection);
        // 画布节点 source（canvas_ref_id = sub-canvas-1）→ 画布节点 target（canvas_ref_id = sub-canvas-2）。
        let mut canvas_source = make_node("canvas-source", &canvas_id);
        canvas_source.canvas_ref_id = Some("sub-canvas-1".to_string());
        node_dao::insert(&connection, &canvas_source).unwrap();
        let mut canvas_target = make_node("canvas-target", &canvas_id);
        canvas_target.canvas_ref_id = Some("sub-canvas-2".to_string());
        node_dao::insert(&connection, &canvas_target).unwrap();
        // edge-prod：source → target，按规则在 source 的子画布 sub-canvas-1 内产生 target 的出向影子。
        insert_edge(&connection, &canvas_id, "edge-prod", &canvas_source.id, &canvas_target.id);
        let mut shadow = make_node("shadow-out", "sub-canvas-1");
        shadow.shadow_id = Some("edge-prod".to_string());
        node_dao::insert(&connection, &shadow).unwrap();

        // sub-canvas-1 内建一条入边：m → shadow。该边按规则 3（普通→出向影子）应再触发
        // 嵌套影子创建于 target 根本体画布节点 canvas_target 引用的子画布 sub-canvas-2 内；
        // 测试中手动插入该嵌套影子以模拟真实流程。
        let mut m = make_node("m", "sub-canvas-1");
        m.title = "m-title".to_string();
        node_dao::insert(&connection, &m).unwrap();
        insert_edge(&connection, "sub-canvas-1", "edge-m-s", &m.id, &shadow.id);
        let mut nested_shadow = make_node("nested-shadow", "sub-canvas-2");
        nested_shadow.shadow_id = Some("edge-m-s".to_string());
        node_dao::insert(&connection, &nested_shadow).unwrap();

        let prod_edge = edge_dao::select_by_id(&connection, "edge-prod").unwrap().unwrap();
        let affected = collect_edge_disconnected(&connection, &prod_edge).unwrap();
        assert_eq!(affected, vec!["m-title".to_string()]);
    }

    /// 递归路径：影子的相连边若也产生嵌套影子（影子 → 画布节点），嵌套影子的邻居也被收集。
    /// 拓扑：edge-prod 产生入向影子 S1，S1 通过 edge-inner 连接画布节点 canvas_b2 →
    /// 在 canvas_b2 嵌套产生影子 S2，S2 拥有邻居 m2；断连 edge-prod 时 m2 也应被收集。
    #[test]
    fn test_collect_recursive_through_nested_shadow() {
        let connection = Connection::open_in_memory().unwrap();
        let canvas_id = setup_canvas(&connection);
        // 边 1：source plain → canvas_b（画布节点），在 canvas_b 产生入向影子 S1。
        let source = make_node("source-plain", &canvas_id);
        node_dao::insert(&connection, &source).unwrap();
        let mut canvas_b = make_node("canvas-b", &canvas_id);
        canvas_b.canvas_ref_id = Some("sub-canvas-1".to_string());
        node_dao::insert(&connection, &canvas_b).unwrap();
        insert_edge(&connection, &canvas_id, "edge-prod", &source.id, &canvas_b.id);
        let mut shadow = make_node("shadow-in", &canvas_id);
        shadow.shadow_id = Some("edge-prod".to_string());
        node_dao::insert(&connection, &shadow).unwrap();

        // 边 2：S1 → canvas_b2（画布节点），按规则应在 canvas_b2.canvas_ref_id 内产生嵌套影子 S2。
        // 此处为简化测试构造，直接把 S2 手工落到 canvas_id（与 S1 同画布）以避开跨画布建表：
        // 递归只关心"边产生影子 → 影子的邻居进入 affected"，对落点画布无要求。
        let mut canvas_b2 = make_node("canvas-b2", &canvas_id);
        canvas_b2.canvas_ref_id = Some("sub-canvas-2".to_string());
        node_dao::insert(&connection, &canvas_b2).unwrap();
        insert_edge(&connection, &canvas_id, "edge-inner", &shadow.id, &canvas_b2.id);
        let mut nested_shadow = make_node("nested-shadow", &canvas_id);
        nested_shadow.title = "nested-shadow-title".to_string();
        nested_shadow.shadow_id = Some("edge-inner".to_string());
        node_dao::insert(&connection, &nested_shadow).unwrap();
        // 邻居 m2 与 nested_shadow 相连。
        let mut m2 = make_node("m2", &canvas_id);
        m2.title = "m2-title".to_string();
        node_dao::insert(&connection, &m2).unwrap();
        insert_edge(&connection, &canvas_id, "edge-ns-m2", &nested_shadow.id, &m2.id);
        // 影子 S1 自己也有邻居 m1，应当一起被收集。
        let mut m1 = make_node("m1", &canvas_id);
        m1.title = "m1-title".to_string();
        node_dao::insert(&connection, &m1).unwrap();
        insert_edge(&connection, &canvas_id, "edge-s-m1", &shadow.id, &m1.id);

        let prod_edge = edge_dao::select_by_id(&connection, "edge-prod").unwrap().unwrap();
        let affected = collect_edge_disconnected(&connection, &prod_edge).unwrap();
        assert!(affected.contains(&"m1-title".to_string()));
        assert!(affected.contains(&"m2-title".to_string()));
    }

    /// 边不产生影子（普通→普通）时返回空列表。
    #[test]
    fn test_collect_plain_to_plain_returns_empty() {
        let connection = Connection::open_in_memory().unwrap();
        let canvas_id = setup_canvas(&connection);
        let a = make_node("a", &canvas_id);
        node_dao::insert(&connection, &a).unwrap();
        let b = make_node("b", &canvas_id);
        node_dao::insert(&connection, &b).unwrap();
        insert_edge(&connection, &canvas_id, "edge-ab", &a.id, &b.id);

        let edge = edge_dao::select_by_id(&connection, "edge-ab").unwrap().unwrap();
        let affected = collect_edge_disconnected(&connection, &edge).unwrap();
        assert!(affected.is_empty());
    }

    /// 应产生影子却缺失时返回 DataCorruptionMissingShadow。
    /// 构造：边 source 是画布节点 → target 是画布节点；FK OFF 跳过建影子。
    #[test]
    fn test_collect_missing_shadow_returns_data_corruption_missing_shadow() {
        let connection = Connection::open_in_memory().unwrap();
        let canvas_id = setup_canvas(&connection);
        let mut b1 = make_node("b1", &canvas_id);
        b1.canvas_ref_id = Some("sub-canvas-1".to_string());
        node_dao::insert(&connection, &b1).unwrap();
        let mut b2 = make_node("b2", &canvas_id);
        b2.canvas_ref_id = Some("sub-canvas-2".to_string());
        node_dao::insert(&connection, &b2).unwrap();
        // 不插影子，按规则画布→画布应产生影子，缺失即报 DataCorruptionMissingShadow。
        insert_edge(&connection, &canvas_id, "edge-b1b2", &b1.id, &b2.id);

        let edge = edge_dao::select_by_id(&connection, "edge-b1b2").unwrap().unwrap();
        let err = collect_edge_disconnected(&connection, &edge).unwrap_err();
        assert!(matches!(
            err,
            ErrorCode::DataCorruptionMissingShadow { ref edge_id } if edge_id == "edge-b1b2"
        ));
    }

    /// 入向影子存在入边时返回 DataCorruptionShadowEdgeDirectionMismatch。
    #[test]
    fn test_collect_inflow_with_in_edge_returns_direction_mismatch() {
        let connection = Connection::open_in_memory().unwrap();
        let canvas_id = setup_canvas(&connection);
        let source = make_node("source-plain", &canvas_id);
        node_dao::insert(&connection, &source).unwrap();
        let canvas_b = make_node("canvas-b", &canvas_id);
        node_dao::insert(&connection, &canvas_b).unwrap();
        insert_edge(&connection, &canvas_id, "edge-prod", &source.id, &canvas_b.id);
        let mut shadow = make_node("shadow-in", &canvas_id);
        shadow.title = "shadow-in".to_string();
        shadow.shadow_id = Some("edge-prod".to_string());
        node_dao::insert(&connection, &shadow).unwrap();

        // 构造一条入向影子的入边：m → shadow（应报方向不匹配）。
        let m = make_node("m", &canvas_id);
        node_dao::insert(&connection, &m).unwrap();
        insert_edge(&connection, &canvas_id, "edge-m-s", &m.id, &shadow.id);

        let prod_edge = edge_dao::select_by_id(&connection, "edge-prod").unwrap().unwrap();
        let err = collect_edge_disconnected(&connection, &prod_edge).unwrap_err();
        assert!(matches!(
            err,
            ErrorCode::DataCorruptionShadowEdgeDirectionMismatch {
                ref shadow_id,
                ref edge_id,
            } if shadow_id == "shadow-in" && edge_id == "edge-m-s"
        ));
    }

    /// 邻居是影子节点时返回 DataCorruptionShadowNeighborIsShadow。
    #[test]
    fn test_collect_neighbor_is_shadow_returns_neighbor_is_shadow() {
        let connection = Connection::open_in_memory().unwrap();
        let canvas_id = setup_canvas(&connection);
        // 边 source=plain → target=canvas_b，产生入向影子 S。
        let source = make_node("source-plain", &canvas_id);
        node_dao::insert(&connection, &source).unwrap();
        let canvas_b = make_node("canvas-b", &canvas_id);
        node_dao::insert(&connection, &canvas_b).unwrap();
        insert_edge(&connection, &canvas_id, "edge-prod", &source.id, &canvas_b.id);
        let mut shadow = make_node("shadow-in", &canvas_id);
        shadow.shadow_id = Some("edge-prod".to_string());
        node_dao::insert(&connection, &shadow).unwrap();
        // 邻居也是一个影子节点（脏数据构造；正常路径影子之间不允许相连）。
        let mut neighbor_shadow = make_node("neighbor-shadow", &canvas_id);
        neighbor_shadow.shadow_id = Some("some-other-edge-id".to_string());
        node_dao::insert(&connection, &neighbor_shadow).unwrap();
        insert_edge(&connection, &canvas_id, "edge-s-n", &shadow.id, &neighbor_shadow.id);

        let prod_edge = edge_dao::select_by_id(&connection, "edge-prod").unwrap().unwrap();
        let err = collect_edge_disconnected(&connection, &prod_edge).unwrap_err();
        assert!(matches!(
            err,
            ErrorCode::DataCorruptionShadowNeighborIsShadow {
                ref shadow_id,
                ref neighbor_id,
            } if shadow_id == "shadow-in" && neighbor_id == "neighbor-shadow"
        ));
    }

    /// 相连边端点缺失时返回 DataCorruptionEdgeEndpointMissing（影子端点缺失）。
    #[test]
    fn test_collect_shadow_edge_endpoint_missing() {
        let connection = Connection::open_in_memory().unwrap();
        let canvas_id = setup_canvas(&connection);
        let source = make_node("source-plain", &canvas_id);
        node_dao::insert(&connection, &source).unwrap();
        let canvas_b = make_node("canvas-b", &canvas_id);
        node_dao::insert(&connection, &canvas_b).unwrap();
        insert_edge(&connection, &canvas_id, "edge-prod", &source.id, &canvas_b.id);
        let mut shadow = make_node("shadow-in", &canvas_id);
        shadow.shadow_id = Some("edge-prod".to_string());
        node_dao::insert(&connection, &shadow).unwrap();
        // 边 target 端指向不存在的节点 id。
        insert_edge(&connection, &canvas_id, "edge-bad", &shadow.id, "missing-target-id");

        let prod_edge = edge_dao::select_by_id(&connection, "edge-prod").unwrap().unwrap();
        let err = collect_edge_disconnected(&connection, &prod_edge).unwrap_err();
        assert!(matches!(
            err,
            ErrorCode::DataCorruptionEdgeEndpointMissing {
                ref edge_id,
                ref node_id,
            } if edge_id == "edge-bad" && node_id == "missing-target-id"
        ));
    }
}
