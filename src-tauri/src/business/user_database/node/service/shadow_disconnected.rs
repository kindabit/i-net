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
/// 1. 推导当前影子的方向（影子方向必然可推导，推导不出或发现其它不一致时返回 DataCorruption* 错误）；
/// 2. 取该影子所在画布的全部边，仅按影子方向精确匹配的边收集邻居：
///    入向影子只收集其出边的目标；出向影子只收集其入边的源；
/// 3. 邻居必然是普通节点或画布节点（不会是影子，否则数据损坏），邻居标题直接取自身 title；
/// 4. 邻居是画布节点（canvas_ref_id 非空）时，shadow 被删除后其在引用画布内的嵌套影子
///    由外键级联删除，递归对下一层影子收集并合并结果；嵌套影子缺失属于数据损坏；
/// 5. 标题去重后返回。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `shadow`: 即将被物理删除的影子节点。
///
/// # 返回值
/// 返回受影响节点的标题列表（去重）；数据不一致或损坏时返回对应的 `DataCorruption*` 错误；
/// 发生数据库错误时返回对应的 `ErrorCode`。
pub fn collect_shadow_disconnected(
    connection: &Connection,
    shadow: &Node,
) -> Result<Vec<String>, ErrorCode> {
    let mut affected: Vec<String> = Vec::new();
    collect_into(connection, shadow, &mut affected)?;
    Ok(affected)
}

/// 在当前画布内按影子方向精确收集受影响邻居标题，并向下递归收集嵌套影子带来的断连。
///
/// 邻居必然不是影子（影子节点之间不能互连，画布节点之间也不能互连），因此可以直接使用
/// 邻居自身的 title 作为展示标题；只有画布节点邻居才会引出下一层嵌套影子的递归扫描。
fn collect_into(
    connection: &Connection,
    shadow: &Node,
    affected: &mut Vec<String>,
) -> Result<(), ErrorCode> {
    let direction = shadow_direction(connection, shadow)?;
    let edges = edge::dao::select_by_canvas_id(connection, &shadow.canvas_id)?;
    let mut downstream_shadows: Vec<Node> = Vec::new();
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
        let title = neighbor.title.clone();
        if !affected.contains(&title) {
            affected.push(title);
        }
        // 邻居是画布节点时，shadow 删除后该画布内 shadow 的嵌套影子会由外键级联删除，
        // 需要继续递归收集下一层画布中的断连。影子邻居已被前置校验排除，不会进入此分支。
        if let Some(ref_canvas_id) = &neighbor.canvas_ref_id {
            let nested_shadow = dao::select_by_shadow_id_and_canvas_id(
                connection,
                &shadow.id,
                ref_canvas_id,
            )?
            .ok_or_else(|| ErrorCode::DataCorruptionMissingShadow {
                origin_id: shadow.id.clone(),
                canvas_id: ref_canvas_id.clone(),
            })?;
            downstream_shadows.push(nested_shadow);
        }
    }
    for nested in downstream_shadows {
        collect_into(connection, &nested, affected)?;
    }
    Ok(())
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
            title: id.to_string(),
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

    /// 建立可推导方向的最小拓扑：父画布内的画布节点 B（引用 sub）加边 X→B，使
    /// sub 内指向 X 的影子被推导为 Inflow；同时返回该影子节点。
    /// 失败路径测试需要在该拓扑上叠加脏数据。
    fn build_inflow_topology(connection: &Connection) -> Node {
        dao::create_table(connection).unwrap();
        edge::dao::create_table(connection).unwrap();
        let mut b = make_node("B", "parent");
        b.canvas_ref_id = Some("sub".to_string());
        dao::insert(connection, &b).unwrap();
        dao::insert(connection, &make_node("X", "parent")).unwrap();
        edge::dao::insert(connection, &make_edge("e-xb", "parent", "X", "B")).unwrap();
        let mut shadow = make_node("s-x", "sub");
        shadow.title = String::new();
        shadow.sub_title = String::new();
        shadow.color = String::new();
        shadow.shadow_id = Some("X".to_string());
        dao::insert(connection, &shadow).unwrap();
        shadow
    }

    /// 成功路径：入向影子有出边时收集出边目标标题；出向影子有入边时收集入边源标题；
    /// 标题去重。同函数内复用拓扑建立两种方向以减少代码量。
    #[test]
    fn test_collect_into_success() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch("PRAGMA foreign_keys = OFF;")
            .unwrap();
        // 入向影子 sub-s 的拓扑：parent 内 X→B；sub 内插入 s-x 与 s-z，
        // 额外建立 B→Z 让 s-z 推导为 Outflow。
        let shadow_in = build_inflow_topology(&connection);
        // sub 内创建普通节点 M1、M2，建边 s-x→M1、s-x→M2（入向影子有出边）。
        dao::insert(&connection, &make_node("M1", "sub")).unwrap();
        dao::insert(&connection, &make_node("M2", "sub")).unwrap();
        edge::dao::insert(&connection, &make_edge("e1", "sub", "s-x", "M1")).unwrap();
        edge::dao::insert(&connection, &make_edge("e2", "sub", "s-x", "M2")).unwrap();
        let titles = collect_shadow_disconnected(&connection, &shadow_in).unwrap();
        assert_eq!(titles.len(), 2);
        assert!(titles.contains(&"M1".to_string()));
        assert!(titles.contains(&"M2".to_string()));

        // 出向影子：在 sub 内创建 s-z（shadow_id=Z），建立 N→s-z（出向影子有入边）。
        dao::insert(&connection, &make_node("N", "sub")).unwrap();
        let mut z = make_node("Z", "parent");
        z.title = "Z".to_string();
        dao::insert(&connection, &z).unwrap();
        edge::dao::insert(&connection, &make_edge("e-bz", "parent", "B", "Z")).unwrap();
        let mut shadow_out = make_node("s-z", "sub");
        shadow_out.title = String::new();
        shadow_out.sub_title = String::new();
        shadow_out.color = String::new();
        shadow_out.shadow_id = Some("Z".to_string());
        dao::insert(&connection, &shadow_out).unwrap();
        edge::dao::insert(&connection, &make_edge("e-nz", "sub", "N", "s-z")).unwrap();
        let titles = collect_shadow_disconnected(&connection, &shadow_out).unwrap();
        assert_eq!(titles, vec!["N".to_string()]);
    }

    /// 失败路径：入向影子存在入边（方向不匹配）时报 DataCorruptionShadowEdgeDirectionMismatch。
    #[test]
    fn test_collect_into_direction_mismatch() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch("PRAGMA foreign_keys = OFF;")
            .unwrap();
        let shadow = build_inflow_topology(&connection);
        // sub 内建普通节点 K，让某条边从 K 指向入向影子 s-x（入边，方向不匹配）。
        dao::insert(&connection, &make_node("K", "sub")).unwrap();
        edge::dao::insert(&connection, &make_edge("e-bad", "sub", "K", "s-x")).unwrap();
        assert!(matches!(
            collect_shadow_disconnected(&connection, &shadow),
            Err(ErrorCode::DataCorruptionShadowEdgeDirectionMismatch {
                shadow_id,
                edge_id,
            }) if shadow_id == "s-x" && edge_id == "e-bad"
        ));
    }

    /// 失败路径：边的另一端节点不存在时报 DataCorruptionEdgeEndpointMissing。
    #[test]
    fn test_collect_into_endpoint_missing() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch("PRAGMA foreign_keys = OFF;")
            .unwrap();
        let shadow = build_inflow_topology(&connection);
        // 关闭外键直接插入一条 source 为 s-x、target 为不存在节点的边（方向上 s-x→target）。
        edge::dao::insert(&connection, &make_edge("e-orphan", "sub", "s-x", "missing")).unwrap();
        assert!(matches!(
            collect_shadow_disconnected(&connection, &shadow),
            Err(ErrorCode::DataCorruptionEdgeEndpointMissing {
                edge_id,
                node_id,
            }) if edge_id == "e-orphan" && node_id == "missing"
        ));
    }

    /// 失败路径：邻居是影子时报 DataCorruptionShadowNeighborIsShadow。
    #[test]
    fn test_collect_into_neighbor_is_shadow() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch("PRAGMA foreign_keys = OFF;")
            .unwrap();
        let shadow = build_inflow_topology(&connection);
        // 在 sub 内插入另一个影子节点 s-other（指向不存在的原始节点，使后续 shadow_direction
        // 在收集阶段不会触发），然后建立 s-x→s-other 让 s-other 成为 s-x 的"邻居"。
        let mut other = make_node("s-other", "sub");
        other.title = String::new();
        other.sub_title = String::new();
        other.color = String::new();
        other.shadow_id = Some("some-origin".to_string());
        dao::insert(&connection, &other).unwrap();
        edge::dao::insert(&connection, &make_edge("e-bad", "sub", "s-x", "s-other")).unwrap();
        assert!(matches!(
            collect_shadow_disconnected(&connection, &shadow),
            Err(ErrorCode::DataCorruptionShadowNeighborIsShadow {
                shadow_id,
                neighbor_id,
            }) if shadow_id == "s-x" && neighbor_id == "s-other"
        ));
    }
}