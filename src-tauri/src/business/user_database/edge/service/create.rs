use std::collections::{HashMap, HashSet};

use rusqlite::Connection;

use crate::business::user_database::edge::dao;
use crate::business::user_database::entity::{Action, Edge, Node};
use crate::business::user_database::shadow::vo::ShadowDirection;
use crate::business::user_database::{log, node, shadow, state};
use crate::error_code::ErrorCode;

/// 在指定画布内新建一条边：两端节点都必须存在且属于该画布，
/// 且新建这条边不会在画布内成环（不考虑连接桩；两端已有边时排除旧边后检查）。
///
/// 新连接规则矩阵：
/// - 普通节点 → 普通节点：仅插边，无影子。
/// - 普通节点 / 入向影子 → 画布节点：在 target 引用的子画布内创建源端根本体的入向影子。
/// - 普通节点 → 出向影子：沿目标影子本体链解析到根本体画布节点，在其引用的子画布内创建源端根本体的入向影子。
/// - 画布节点 → 画布节点 / 出向影子：在 source 引用的子画布内创建目标端根本体的出向影子。
/// - 入向影子 → 普通节点：仅插边，无影子。
///
/// 非法连接：
/// - 画布节点 → 普通节点：`ErrorCode::CanvasToPlainNodeEdge`（避免依赖项散落各画布）。
/// - 入向影子作为目标 / 出向影子作为源：`ErrorCode::InvalidShadowEdge`。
/// - 入向影子 → 出向影子：`ErrorCode::ShadowToShadowEdge`（应在父画布直接连线）。
///
/// 影子生命周期由边控制：影子的 shadow_id 指向产生它的边，删除边时其产生的影子
/// 经 node.shadow_id 外键级联删除，影子的相连边经 edge.source_id/target_id 外键级联删除，
/// 下游嵌套影子沿外键链递归坍塌，应用层不再手动删除影子。
///
/// 重建语义分两段：
/// - 同向已有边（同 (source_id, target_id) 命中旧边）：仅更新旧边的连接桩后直接返回，
///   不删边、不动影子、不做成环检查、不做断连确认、不记任何日志；连接桩完全相同的重复拖线幂等成功。
/// - 反向已有边（同 (target_id, source_id) 命中旧边）：执行"删旧建新"——从边集中排除该旧边后
///   做成环检查；通过后收集旧边关联影子的断连影响，未确认时返回
///   `ErrorCode::EdgeDeleteDisconnectsNodes`，由前端确认后以 `confirmed = true` 重调；
///   确认后删除旧边（其产生的影子经外键级联删除），插入新边（继承旧边的 title 和 description），
///   再按新建流程建影子联动；产生 `Action::EdgeReplace` 日志。
/// - 无旧边：走全新建边流程，产生 `Action::EdgeCreate` 日志。
///
/// # 参数
/// - `canvas_id`: 画布 id。
/// - `source_id`: 源节点 id。
/// - `source_port`: 源节点连接桩。
/// - `target_id`: 目标节点 id。
/// - `target_port`: 目标节点连接桩。
/// - `confirmed`: 调用方已确认反向替换路径中影子删除带来的连接断开影响；同向重建与无旧边流程忽略。
///
/// # 返回值
/// 返回新建或更新后的边；任一节点不存在时返回 `ErrorCode::NoNodeWithSuchId`，
/// 两端连接桩相同时返回 `ErrorCode::EdgeSameNodePort`，
/// 新建该边会成环时返回 `ErrorCode::EdgeWouldFormCycle`，
/// 画布节点作为源连接普通节点时返回 `ErrorCode::CanvasToPlainNodeEdge`，
/// 影子节点互相连接时返回 `ErrorCode::ShadowToShadowEdge`，
/// 影子节点连线方向不合法时返回 `ErrorCode::InvalidShadowEdge`，
/// 反向替换路径中删除旧边会使子画布内的节点失去连接且未确认时返回
/// `ErrorCode::EdgeDeleteDisconnectsNodes`，
/// 发生其他错误时返回对应的 `ErrorCode`。
pub fn create(
    canvas_id: &str,
    source_id: &str,
    source_port: String,
    target_id: &str,
    target_port: String,
    confirmed: bool,
) -> Result<Edge, ErrorCode> {
    let connection = state::lock_connection();
    let source = node::dao::select_by_id(&connection, source_id)?
        .filter(|node| node.canvas_id == canvas_id)
        .ok_or_else(|| ErrorCode::NoNodeWithSuchId {
            id: source_id.to_string(),
        })?;
    let target = node::dao::select_by_id(&connection, target_id)?
        .filter(|node| node.canvas_id == canvas_id)
        .ok_or_else(|| ErrorCode::NoNodeWithSuchId {
            id: target_id.to_string(),
        })?;
    validate_shadow_endpoints(&connection, &source, &target)?;
    if source_port == target_port {
        return Err(ErrorCode::EdgeSameNodePort);
    }
    // 查同向旧边：命中则直接更新连接桩并返回（同向重建只是调整连接位置，
    // 不删边、不动影子、无需成环检查与断连确认、不记日志）。
    if let Some(mut old) = dao::select_between(&connection, source_id, target_id)? {
        dao::update_ports(&connection, &old.id, &source_port, &target_port)?;
        old.source_port = source_port;
        old.target_port = target_port;
        return Ok(old);
    }
    // 查反向旧边（换向替换路径）：UNIQUE(source_id, target_id) 保证至多一行。
    let old_edge = dao::select_between(&connection, target_id, source_id)?;
    // 成环检查：边集排除旧边。
    let edges = dao::select_by_canvas_id(&connection, canvas_id)?;
    let edges: Vec<Edge> = match &old_edge {
        Some(old) => edges.into_iter().filter(|e| e.id != old.id).collect(),
        None => edges,
    };
    if would_form_cycle(&edges, source_id, target_id) {
        return Err(ErrorCode::EdgeWouldFormCycle);
    }
    // 日志载荷的端点标题取展示标题：影子端点的标题落库为空串，须沿产生边链解析根本体标题；
    // 解析须在删除旧边之前完成（影子链解析依赖产生边链完好）。
    let source_title = shadow::service::display_title(&connection, &source)?;
    let target_title = shadow::service::display_title(&connection, &target)?;
    // 替换路径：断连收集与确认拦截 → 删旧边（影子由外键级联）。
    let replaced = old_edge.is_some();
    let mut inherited: Option<(String, String)> = None;
    if let Some(old) = &old_edge {
        // 断连收集须在删除旧边之前完成（收集依赖产生边链完好）。
        let affected = shadow::service::collect_edge_disconnected(&connection, old)?;
        if !affected.is_empty() && !confirmed {
            return Err(ErrorCode::EdgeDeleteDisconnectsNodes { nodes: affected });
        }
        // 删除旧边：其产生的影子经 node.shadow_id 外键级联删除，下游嵌套影子沿
        // edge.source_id/target_id 与 node.shadow_id 外键链递归级联，应用层禁止手写递归删除。
        dao::delete_by_id(&connection, &old.id)?;
        inherited = Some((old.title.clone(), old.description.clone()));
    }
    let (title, description) = inherited.unwrap_or_default();
    let edge = Edge {
        id: uuid::Uuid::new_v4().to_string(),
        canvas_id: canvas_id.to_string(),
        source_id: source_id.to_string(),
        source_port,
        target_id: target_id.to_string(),
        target_port,
        title,
        description,
    };
    dao::insert(&connection, &edge)?;
    shadow::service::create_shadow_for_edge(&connection, &edge, &source, &target)?;
    if replaced {
        // 旧边即新边的反向：旧边源端是新边目标节点，旧边目标端是新边源节点。
        log::service::create(Action::EdgeReplace {
            old_source_title: target_title.clone(),
            old_target_title: source_title.clone(),
            source_title,
            target_title,
        })?;
    } else {
        log::service::create(Action::EdgeCreate {
            source_title,
            target_title,
        })?;
    }
    Ok(edge)
}

/// 校验影子节点参与连线时的约束：影子节点之间不能互相连接；
/// 入向影子只能作为源（只有出度），出向影子只能作为目标（只有入度）；
/// 画布节点不能直接作为源连接普通节点。
///
/// 影子方向必然可推导，推导不出时 shadow_direction 返回 DataCorruption* 错误。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `source`: 源节点。
/// - `target`: 目标节点。
///
/// # 返回值
/// 连线合法时返回 `Ok(())`；画布节点连接普通节点时返回 `ErrorCode::CanvasToPlainNodeEdge`，
/// 影子节点连线方向不合法时返回 `ErrorCode::InvalidShadowEdge`，
/// 两端皆影子节点时返回 `ErrorCode::ShadowToShadowEdge`，
/// 影子方向推导失败时返回对应的 `DataCorruption*` 错误，
/// 发生数据库错误时返回对应的 `ErrorCode`。
fn validate_shadow_endpoints(
    connection: &Connection,
    source: &Node,
    target: &Node,
) -> Result<(), ErrorCode> {
    // 画布节点不能直接作为源连接普通节点（避免目标节点的依赖项散落在各画布中，
    // 应先经子画布中转）。
    if source.canvas_ref_id.is_some() && target.canvas_ref_id.is_none() && target.shadow_id.is_none()
    {
        return Err(ErrorCode::CanvasToPlainNodeEdge);
    }
    // 入向影子（普通节点的影子）只能作为源，不能作为目标。
    if target.shadow_id.is_some()
        && shadow::service::shadow_direction(connection, target)? == ShadowDirection::Inflow
    {
        return Err(ErrorCode::InvalidShadowEdge);
    }
    // 出向影子（画布节点的影子）只能作为目标，不能作为源。
    if source.shadow_id.is_some()
        && shadow::service::shadow_direction(connection, source)? == ShadowDirection::Outflow
    {
        return Err(ErrorCode::InvalidShadowEdge);
    }
    // 能走到这里的双影子组合只剩 入向影子→出向影子：两者的本体（或上级影子）都直接或间接
    // 存在于父画布中，若需要连接应直接在父画布中进行，而不是先接入子画布再连接。
    if source.shadow_id.is_some() && target.shadow_id.is_some() {
        return Err(ErrorCode::ShadowToShadowEdge);
    }
    Ok(())
}

/// 判断在现有边的基础上新建一条从源节点到目标节点的有向边是否会成环：
/// 从目标节点出发沿有向边能够到达源节点（含源节点与目标节点相同的自环情况）即成环。
///
/// # 参数
/// - `edges`: 画布内现有的全部边。
/// - `source_id`: 新边的源节点 id。
/// - `target_id`: 新边的目标节点 id。
///
/// # 返回值
/// 返回新建该边是否会成环的布尔值。
fn would_form_cycle(edges: &[Edge], source_id: &str, target_id: &str) -> bool {
    let mut adjacency: HashMap<&str, Vec<&str>> = HashMap::new();
    for edge in edges {
        adjacency
            .entry(edge.source_id.as_str())
            .or_default()
            .push(edge.target_id.as_str());
    }
    let mut stack = vec![target_id];
    let mut visited = HashSet::new();
    while let Some(current) = stack.pop() {
        if current == source_id {
            return true;
        }
        if !visited.insert(current) {
            continue;
        }
        if let Some(next) = adjacency.get(current) {
            stack.extend(next);
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::business::metadata;
    use crate::business::user_database::canvas;
    use crate::business::user_database::edge;
    use crate::business::user_database::entity;
    use crate::business::user_database::lifecycle;
    use crate::business::user_database::log::service::LogFilter;
    use crate::test;

    /// 边新建的重建语义：同向重建直接更新旧边连接桩（id 不变、title/description 保留、不记日志）、
    /// 换向重建继承旧边 title/description、换向仍成环时拦截、影子断连双阶段确认、
    /// 影子方向翻转、影子端点校验与同 port 校验在替换路径下仍然先于边存在性判断生效。
    #[test]
    fn test_edge_replace() {
        let _guard = test::acquire_test_lock();

        // 初始化测试数据目录、metadata 数据库并打开一个全新的用户数据库。
        let path = test::create_test_path();
        crate::state::set_path(path.clone());
        metadata::service::initialize().unwrap();
        let registered = metadata::service::register("edge-replace-test-db".to_string()).unwrap();
        lifecycle::service::initialize(&registered.id, test::test_key()).unwrap();
        let canvases = canvas::service::list(false).unwrap();
        let root = canvases[0].clone();

        // 影子行查询辅助：按产生边 id 从 connection 上取影子节点本体；
        // 新机制下画布内对某原始节点的影子是当前生效产生边的影子，需传入最新边 id 才能查到当前影子。
        let shadow_by_edge = |edge_id: &str| {
            let connection = state::lock_connection();
            shadow::dao::select_by_producing_edge_id(&connection, edge_id).unwrap()
        };

        // ===== 第 1 阶段：同向重建（无影子），仅更新连接桩，id 不变且 title/description 保留，不记日志 =====
        // 准备根画布内两个普通节点 A、B，建边 A→B。
        let node_a = node::service::create(&root.id, "title-a".to_string(), "sub-a".to_string(), 0.0, 0.0, None, false).unwrap();
        let node_b = node::service::create(&root.id, "title-b".to_string(), "sub-b".to_string(), 200.0, 0.0, None, false).unwrap();
        let edge_ab = edge::service::create(&root.id, &node_a.id, "right".to_string(), &node_b.id, "left".to_string(), false).unwrap();
        // 写入标题与详情，便于断言保留。
        edge::service::update(&edge_ab.id, "inherited title".to_string(), "inherited desc".to_string()).unwrap();
        // 同向用不同连接桩重建 → 触发端口更新路径。
        let log_total_before_port = log::service::list(0, 1, LogFilter::default()).unwrap().total;
        let edge_ab_updated = edge::service::create(&root.id, &node_a.id, "top".to_string(), &node_b.id, "bottom".to_string(), false).unwrap();
        // 边 id 不变，端口为新值，title/description 保留。
        assert_eq!(edge_ab_updated.id, edge_ab.id);
        assert_eq!(edge_ab_updated.source_port, "top");
        assert_eq!(edge_ab_updated.target_port, "bottom");
        assert_eq!(edge_ab_updated.title, "inherited title");
        assert_eq!(edge_ab_updated.description, "inherited desc");
        let persisted = edge::service::list(&root.id).unwrap();
        assert_eq!(persisted.len(), 1);
        assert_eq!(persisted[0].id, edge_ab.id);
        // 日志总数不变（同向重建不记任何日志）。
        assert_eq!(log::service::list(0, 1, LogFilter::default()).unwrap().total, log_total_before_port);
        // 幂等用例：连接桩完全相同的重复拖线也成功，且日志总数仍不变。
        let edge_ab_idempotent = edge::service::create(&root.id, &node_a.id, "top".to_string(), &node_b.id, "bottom".to_string(), false).unwrap();
        assert_eq!(edge_ab_idempotent.id, edge_ab.id);
        assert_eq!(edge_ab_idempotent.source_port, "top");
        assert_eq!(edge_ab_idempotent.target_port, "bottom");
        assert_eq!(log::service::list(0, 1, LogFilter::default()).unwrap().total, log_total_before_port);
        let edge_ab_new = edge_ab_updated;

        // ===== 第 2 阶段：换向替换（无影子），反向建边在排除旧边后不成环 =====
        // 当前画布只有 edge_ab_new（A→B）；再建 B→A（换向）应替换原 A→B。
        assert!(edge::service::list(&root.id).unwrap().iter().any(|e| e.id == edge_ab_new.id));
        let edge_reversed = edge::service::create(&root.id, &node_b.id, "right".to_string(), &node_a.id, "left".to_string(), false).unwrap();
        // 原 A→B 消失，B→A 存在。
        let after_reverse = edge::service::list(&root.id).unwrap();
        assert!(!after_reverse.iter().any(|e| e.id == edge_ab_new.id));
        assert_eq!(after_reverse.len(), 1);
        assert_eq!(after_reverse[0].id, edge_reversed.id);
        assert_eq!(after_reverse[0].source_id, node_b.id);
        assert_eq!(after_reverse[0].target_id, node_a.id);
        // 日志：EdgeReplace，载荷含新方向（B→A）与旧方向（A→B）的标题；title/description 继承自旧边。
        let replace_log = log::service::list(0, 1000, LogFilter::default()).unwrap()
            .items
            .into_iter()
            .find(|e| matches!(e.action, entity::Action::EdgeReplace { .. }))
            .expect("EdgeReplace log should exist for the reversed edge");
        assert!(matches!(
            &replace_log.action,
            entity::Action::EdgeReplace {
                source_title,
                target_title,
                old_source_title,
                old_target_title,
            } if source_title == "title-b"
                && target_title == "title-a"
                && *old_source_title == "title-a"
                && *old_target_title == "title-b"
        ));

        // ===== 第 3 阶段：换向后仍成环的失败路径 =====
        // 在画布内构造 A→B、B→C、A→C；尝试 C→A 替换 A→C 仍因 A→B→C 形成环而失败。
        // 上一阶段画布只有 B→A；先删除 B→A 以保持画布干净。
        edge::service::delete(&edge_reversed.id, false).unwrap();
        assert!(edge::service::list(&root.id).unwrap().is_empty());
        let node_c = node::service::create(&root.id, "title-c".to_string(), "sub-c".to_string(), 400.0, 0.0, None, false).unwrap();
        edge::service::create(&root.id, &node_a.id, "right".to_string(), &node_b.id, "left".to_string(), false).unwrap();
        edge::service::create(&root.id, &node_b.id, "right".to_string(), &node_c.id, "left".to_string(), false).unwrap();
        let edge_ac = edge::service::create(&root.id, &node_a.id, "right".to_string(), &node_c.id, "left".to_string(), false).unwrap();
        // 尝试换向建 C→A：旧边 A→C 存在，但排除后 A→B→C 仍使 C→A 成环。
        let edges_before = edge::service::list(&root.id).unwrap().len();
        assert!(matches!(
            edge::service::create(&root.id, &node_c.id, "right".to_string(), &node_a.id, "left".to_string(), false),
            Err(ErrorCode::EdgeWouldFormCycle)
        ));
        // 旧边 A→C 保留，画布边数不变。
        assert!(edge::service::list(&root.id).unwrap().iter().any(|e| e.id == edge_ac.id));
        assert_eq!(edge::service::list(&root.id).unwrap().len(), edges_before);

        // ===== 第 4 阶段：同向 port 更新不影响影子及其连接 =====
        // 准备：根画布内画布节点 A、B（分别引用画布 a、画布 b），建边 A→B 在 a 内产生 B 的出向影子。
        // 新规则下画布→普通被禁止，故两端均用画布节点以保留影子+连线场景。
        let node_a = node::service::create(&root.id, "canvas-a".to_string(), String::new(), 0.0, 0.0, None, true).unwrap();
        let canvas_a = node_a.canvas_ref_id.clone().unwrap();
        let node_b2 = node::service::create(&root.id, "canvas-b".to_string(), String::new(), 200.0, 0.0, None, true).unwrap();
        let canvas_b = node_b2.canvas_ref_id.clone().unwrap();
        let edge_ab = edge::service::create(&root.id, &node_a.id, "right".to_string(), &node_b2.id, "left".to_string(), false).unwrap();
        let shadow_b = shadow_by_edge(&edge_ab.id).unwrap();
        // 画布 a 内建普通节点 M1 并建边 shadow_b→M1（出向影子不能作 source，但入向可以；
        // 此处改用画布 a 内普通节点 M1，shadow_b 是出向影子，故边方向 M1 → shadow_b）。
        let node_m1 = node::service::create(&canvas_a, "internal-m1".to_string(), String::new(), 400.0, 0.0, None, false).unwrap();
        let edge_m_s = edge::service::create(&canvas_a, &node_m1.id, "right".to_string(), &shadow_b.id, "left".to_string(), false).unwrap();
        // 同向重建 A→B（port 变换）→ 直接成功（同向已有边走端口更新路径），
        // 影子仍由同一条边产生，shadow_id 与 id 都不变，画布 a 内 M1→shadow_b 边保留。
        let edge_ab_updated = edge::service::create(
            &root.id,
            &node_a.id,
            "top".to_string(),
            &node_b2.id,
            "bottom".to_string(),
            false,
        )
        .unwrap();
        // 边 id 不变，port 为新值。
        assert_eq!(edge_ab_updated.id, edge_ab.id);
        assert_eq!(edge_ab_updated.source_port, "top");
        assert_eq!(edge_ab_updated.target_port, "bottom");
        // 影子仍由同一产生边产生：id 与 shadow_id 都保持不变，画布 a 内 M1→shadow_b 边保留。
        let shadow_b_unchanged = shadow_by_edge(&edge_ab.id).unwrap();
        assert_eq!(shadow_b_unchanged.id, shadow_b.id);
        assert_eq!(shadow_b_unchanged.shadow_id.as_deref(), Some(edge_ab.id.as_str()));
        assert!(edge::service::list(&canvas_a).unwrap().iter().any(|e| e.id == edge_m_s.id));
        // 影子方向仍为 Outflow（产生边源端是画布节点 A）。
        let shadow_b_vo = node::service::list(&canvas_a, false)
            .unwrap()
            .into_iter()
            .find(|n| n.id == shadow_b.id)
            .unwrap();
        assert_eq!(shadow_b_vo.shadow_direction, Some(shadow::vo::ShadowDirection::Outflow));

        // ===== 第 5 阶段：换向替换 + 影子断连双阶段确认 =====
        // 未确认换向建 B→A → 报 EdgeDeleteDisconnectsNodes，nodes 恰为 ["internal-m1"]。
        let Err(ErrorCode::EdgeDeleteDisconnectsNodes { nodes: affected }) = edge::service::create(
            &root.id,
            &node_b2.id,
            "right".to_string(),
            &node_a.id,
            "left".to_string(),
            false,
        )
        else {
            panic!("expected EdgeDeleteDisconnectsNodes");
        };
        assert_eq!(affected, vec!["internal-m1".to_string()]);
        // 旧边 A→B 与影子保留。
        assert!(edge::service::list(&root.id).unwrap().iter().any(|e| e.id == edge_ab.id));
        assert_eq!(shadow_by_edge(&edge_ab.id).unwrap().id, shadow_b.id);
        // confirmed=true 重调 → 成功，根画布边为 B→A，旧边 A→B 被删除，旧影子经 shadow_id 外键级联消失；
        // 新边 B→A 在 source.canvas_ref_id 画布 b 内产生新出向影子（目标根本体 = A）。
        let edge_ba = edge::service::create(
            &root.id,
            &node_b2.id,
            "right".to_string(),
            &node_a.id,
            "left".to_string(),
            true,
        )
        .unwrap();
        let root_edges = edge::service::list(&root.id).unwrap();
        assert!(root_edges.iter().any(|e| e.id == edge_ba.id && e.source_id == node_b2.id && e.target_id == node_a.id));
        assert!(!root_edges.iter().any(|e| e.id == edge_ab.id));
        // 旧影子由旧产生边产生 → 旧产生边已被替换删除 → 旧影子经外键级联消失；
        // 新影子由新产生边 edge_ba 产生，落点在 canvas_b（与旧影子落点 canvas_a 不同）。
        assert!(shadow_by_edge(&edge_ab.id).is_none());
        let shadow_a_in_b = shadow_by_edge(&edge_ba.id).unwrap();
        assert_ne!(shadow_a_in_b.id, shadow_b.id);
        assert_eq!(shadow_a_in_b.canvas_id, canvas_b);
        let shadow_a_in_b_vo = node::service::list(&canvas_b, false)
            .unwrap()
            .into_iter()
            .find(|n| n.id == shadow_a_in_b.id)
            .unwrap();
        assert_eq!(shadow_a_in_b_vo.shadow_direction, Some(shadow::vo::ShadowDirection::Outflow));

        // ===== 第 6 阶段：替换路径影子端点校验仍生效 =====
        // 画布 b 内建普通节点 M2；尝试在 b 内建 shadow_a_in_b（出向影子）→ M2（出向影子作 source）应被拦截。
        let node_m2 = node::service::create(&canvas_b, "internal-m2".to_string(), String::new(), 400.0, 0.0, None, false).unwrap();
        assert!(matches!(
            edge::service::create(&canvas_b, &shadow_a_in_b.id, "right".to_string(), &node_m2.id, "left".to_string(), false),
            Err(ErrorCode::InvalidShadowEdge)
        ));
        // 旧状态不变：M2 仍存在，shadow_a_in_b 仍为出向影子。
        assert!(node::service::list(&canvas_b, false).unwrap().iter().any(|n| n.id == node_m2.id));
        let still_outflow = node::service::list(&canvas_b, false)
            .unwrap()
            .into_iter()
            .find(|n| n.id == shadow_a_in_b.id)
            .unwrap();
        assert_eq!(still_outflow.shadow_direction, Some(shadow::vo::ShadowDirection::Outflow));

        // ===== 第 7 阶段：替换路径同 port 检查仍先生效 =====
        // 准备：在第 5 步状态下 B→A 已存在；尝试用相同 port 重建 B→A 应被 EdgeSameNodePort 拦截。
        assert!(matches!(
            edge::service::create(&root.id, &node_b2.id, "right".to_string(), &node_a.id, "right".to_string(), false),
            Err(ErrorCode::EdgeSameNodePort)
        ));
        // 旧边保留。
        assert!(edge::service::list(&root.id).unwrap().iter().any(|e| e.id == edge_ba.id));

        // 保存并关闭数据库，清理测试数据目录。
        lifecycle::service::save().unwrap();
        lifecycle::service::close().unwrap();
        test::cleanup(&path);
    }
}
