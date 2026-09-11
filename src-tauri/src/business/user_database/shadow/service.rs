mod create;
mod display_title;
mod resolve_root;
mod shadow_direction;
mod shadow_disconnected;

pub use create::create_shadow_for_edge;
pub use display_title::display_title;
pub use resolve_root::resolve_root;
pub use shadow_direction::shadow_direction;
pub use shadow_disconnected::collect_edge_disconnected;

#[cfg(test)]
mod tests {
    use crate::business::metadata;
    use crate::business::user_database::attachment;
    use crate::business::user_database::canvas;
    use crate::business::user_database::edge;
    use crate::business::user_database::entity;
    use crate::business::user_database::export;
    use crate::business::user_database::lifecycle;
    use crate::business::user_database::log;
    use crate::business::user_database::log::service::LogFilter;
    use crate::business::user_database::node;
    use crate::business::user_database::node_field;
    use crate::business::user_database::shadow;
    use crate::business::user_database::state;
    use crate::error_code::ErrorCode;
    use crate::test;
    use crate::util::file_system_util;

    /// 影子节点 service 行为：list 合并展示数据与方向推导、原始节点逻辑删除状态透传、
    /// 各 service 的影子守卫（失败路径）、影子可移动与参与边（成功路径）、导出过滤影子节点。
    #[test]
    fn test_shadow_node_service() {
        let _guard = test::acquire_test_lock();

        // 初始化测试数据目录、metadata 数据库并打开一个全新的用户数据库。
        let path = test::create_test_path();
        crate::state::set_path(path.clone());
        metadata::service::initialize().unwrap();
        let registered = metadata::service::register("shadow-test-db".to_string()).unwrap();
        lifecycle::service::initialize(&registered.id, test::test_key()).unwrap();
        let canvases = canvas::service::list(false).unwrap();
        let root = canvases[0].clone();

        // 准备父画布（根画布）内的节点：普通节点 X、画布节点 B（引用画布 b）、画布节点 Z（引用画布 z）。
        // 新建边规则下画布节点→普通节点被禁止（CanvasToPlainNodeEdge），产生出向影子须经画布→画布路径。
        let node_x = node::service::create(&root.id, "origin-x".to_string(), String::new(), 0.0, 0.0, None, false).unwrap();
        let node_b = node::service::create(&root.id, "canvas-b".to_string(), String::new(), 200.0, 0.0, None, true).unwrap();
        let node_z = node::service::create(&root.id, "canvas-z".to_string(), String::new(), 400.0, 0.0, None, true).unwrap();
        let canvas_b = node_b.canvas_ref_id.clone().unwrap();

        // 通过 service 层建边：由 service 层自动按规则联动创建影子节点。
        // X→B（X 普通节点）：B.canvas_ref_id 画布 b 内产生 X 的入向影子。
        // B→Z（B 画布节点 → Z 画布节点）：B.canvas_ref_id 画布 b 内产生 Z 的出向影子。
        let edge_xb = edge::service::create(&root.id, &node_x.id, "right".to_string(), &node_b.id, "left".to_string(), false).unwrap();
        let edge_bz = edge::service::create(&root.id, &node_b.id, "right".to_string(), &node_z.id, "left".to_string(), false).unwrap();

        // 通过 select_by_producing_edge_id 取出两个影子节点本体（shadow_id 指向产生边）。
        let connection = state::lock_connection();
        let shadow_x = shadow::dao::select_by_producing_edge_id(&connection, &edge_xb.id).unwrap().unwrap();
        let shadow_z = shadow::dao::select_by_producing_edge_id(&connection, &edge_bz.id).unwrap().unwrap();
        drop(connection);

        // list 合并成功路径：影子的 title 合并自原始节点，shadow_id 指向产生边；
        // shadow_direction 由产生边源端节点类型决定：X 普通节点 → Inflow，B 画布节点 → Outflow。
        let nodes_b = node::service::list(&canvas_b, false).unwrap();
        let vo_x = nodes_b.iter().find(|n| n.id == shadow_x.id).unwrap();
        let vo_z = nodes_b.iter().find(|n| n.id == shadow_z.id).unwrap();
        assert_eq!(vo_x.title, "origin-x");
        assert_eq!(vo_x.shadow_id.as_deref(), Some(edge_xb.id.as_str()));
        assert_eq!(vo_x.shadow_direction, Some(shadow::vo::ShadowDirection::Inflow));
        assert!(vo_x.canvas_ref_id.is_none());
        assert_eq!(vo_z.title, "canvas-z");
        assert_eq!(vo_z.shadow_id.as_deref(), Some(edge_bz.id.as_str()));
        assert_eq!(vo_z.shadow_direction, Some(shadow::vo::ShadowDirection::Outflow));

        // 原始节点逻辑删除状态透传：逻辑删除 X 后影子保留且 shadow_origin_deleted 变为 true，恢复后回到 false。
        node::service::logical_delete(&node_x.id).unwrap();
        let merged_x = node::service::list(&canvas_b, false)
            .unwrap()
            .into_iter()
            .find(|n| n.id == shadow_x.id)
            .unwrap();
        assert_eq!(merged_x.shadow_origin_deleted, Some(true));
        node::service::restore(&node_x.id, 0.0, 0.0).unwrap();
        let merged_x = node::service::list(&canvas_b, false)
            .unwrap()
            .into_iter()
            .find(|n| n.id == shadow_x.id)
            .unwrap();
        assert_eq!(merged_x.shadow_origin_deleted, Some(false));

        // 影子守卫失败路径：修改、逻辑删除、恢复、设置颜色、物理删除、写字段、导入附件、复制
        // 作用于影子节点时均报 NodeIsShadow。
        assert!(matches!(
            node::service::modify(&shadow_x.id, "t".to_string(), "s".to_string()),
            Err(ErrorCode::NodeIsShadow)
        ));
        assert!(matches!(
            node::service::copy(&shadow_x.id, 0.0, 0.0),
            Err(ErrorCode::NodeIsShadow)
        ));
        assert!(matches!(
            node::service::logical_delete(&shadow_x.id),
            Err(ErrorCode::NodeIsShadow)
        ));
        assert!(matches!(
            node::service::restore(&shadow_x.id, 0.0, 0.0),
            Err(ErrorCode::NodeIsShadow)
        ));
        assert!(matches!(
            node::service::set_color(&shadow_x.id, "color".to_string()),
            Err(ErrorCode::NodeIsShadow)
        ));
        assert!(matches!(
            node::service::physical_delete(&shadow_x.id, false),
            Err(ErrorCode::NodeIsShadow)
        ));
        assert!(matches!(
            node_field::service::set(&shadow_x.id, &[]),
            Err(ErrorCode::NodeIsShadow)
        ));
        assert!(matches!(
            attachment::service::import(&shadow_x.id, "no-such-file"),
            Err(ErrorCode::NodeIsShadow)
        ));

        // 移动成功路径：位置是影子的自有数据，允许移动。
        node::service::move_nodes(&[node::vo::MoveNodeVO {
            id: shadow_x.id.clone(),
            x: 10.0,
            y: 20.0,
        }])
        .unwrap();
        let moved = node::service::list(&canvas_b, false)
            .unwrap()
            .into_iter()
            .find(|n| n.id == shadow_x.id)
            .unwrap();
        assert_eq!((moved.x, moved.y), (10.0, 20.0));
        // 日志载荷成功路径：影子的移动日志标题沿产生边链解析为根本体标题（影子本体标题落库为空串）。
        let logs = log::service::list(0, 1000, LogFilter::default()).unwrap();
        assert!(logs.items.iter().any(|entry| matches!(
            &entry.action,
            entity::Action::NodeMove { title, .. } if title == "origin-x"
        )));

        // 导出过滤准备：在画布 b 内创建普通节点 N，并建边 shadow_x→N（入向影子有出边）。
        let node_n = node::service::create(&canvas_b, "internal-n".to_string(), String::new(), 500.0, 0.0, None, false).unwrap();
        edge::service::create(&canvas_b, &shadow_x.id, "right".to_string(), &node_n.id, "left".to_string(), false).unwrap();

        // 导出过滤成功路径：影子不作为独立节点导出，与影子相连的边也不出现在关系小节。
        let export_dir = path.data_directory.parent().unwrap().join("shadow-export-test");
        file_system_util::create_dir_all(&export_dir).unwrap();
        let export_path = export_dir.join("shadow.md");
        export::service::export(
            export::service::ExportMode::ExcludeFields,
            "zh-CN",
            &export_path.to_string_lossy(),
        )
        .unwrap();
        let content = String::from_utf8(file_system_util::read(&export_path).unwrap()).unwrap();
        // 全部非影子节点共 4 个（根画布 X/B/Z + 画布 b 内的 N）；若影子未被过滤，会多出空标题的节点小节。
        assert_eq!(content.matches("### 节点：").count(), 4);
        // 关系行共 2 条（根画布内 X→B、B→Z）；画布 b 内唯一的边 shadow_x→N 因影子被过滤而不出现。
        assert_eq!(content.matches("--[]-->").count(), 2);
        // 画布 b 小节正常导出其中的真实节点。
        assert!(content.contains("## 画布：canvas-b"));
        assert!(content.contains("### 节点：internal-n"));

        // 清理导出产物、保存并关闭数据库，最后清理测试数据目录。
        let _ = std::fs::remove_dir_all(&export_dir);
        lifecycle::service::save().unwrap();
        lifecycle::service::close().unwrap();
        test::cleanup(&path);
    }

    /// 边创建的影子节点联动：双向创建影子、影子初始位置车道算法、影子连线的方向守卫、
    /// 影子与画布节点连线（嵌套影子）、画布节点之间允许互相连接（产生出向影子）以及普通建边行为不回归。
    #[test]
    fn test_shadow_node_edge_create() {
        let _guard = test::acquire_test_lock();

        // 初始化测试数据目录、metadata 数据库并打开一个全新的用户数据库。
        let path = test::create_test_path();
        crate::state::set_path(path.clone());
        metadata::service::initialize().unwrap();
        let registered = metadata::service::register("shadow-edge-test-db".to_string()).unwrap();
        lifecycle::service::initialize(&registered.id, test::test_key()).unwrap();
        let canvases = canvas::service::list(false).unwrap();
        let root = canvases[0].clone();

        // 影子行查询辅助：按产生边 id 从 connection 上取影子节点本体。
        let shadow_by_edge = |edge_id: &str| {
            let connection = state::lock_connection();
            shadow::dao::select_by_producing_edge_id(&connection, edge_id).unwrap()
        };

        // 准备父画布（根画布）内的普通节点 X 与画布节点 B（引用画布 b）。
        let node_x = node::service::create(&root.id, "origin-x".to_string(), String::new(), 0.0, 0.0, None, false).unwrap();
        let node_b = node::service::create(&root.id, "canvas-b".to_string(), String::new(), 200.0, 0.0, None, true).unwrap();
        let canvas_b = node_b.canvas_ref_id.clone().unwrap();

        // 入向影子创建成功路径：建边 X→B 后在画布 b 内创建 X 的入向影子。
        // 画布 b 内还没有非影子节点，入向车道取默认 x=0，首个影子 y=0。
        let edge_xb = edge::service::create(&root.id, &node_x.id, "right".to_string(), &node_b.id, "left".to_string(), false).unwrap();
        let shadow_x = shadow_by_edge(&edge_xb.id).unwrap();
        // 影子行本体只有位置与 shadow_id 有意义：title/sub_title/color 为空串，deleted 为 false。
        assert_eq!(shadow_x.canvas_id, canvas_b);
        assert_eq!(shadow_x.shadow_id.as_deref(), Some(edge_xb.id.as_str()));
        assert!(shadow_x.title.is_empty() && shadow_x.sub_title.is_empty() && shadow_x.color.is_empty());
        assert!(!shadow_x.deleted);
        assert!(shadow_x.canvas_ref_id.is_none());
        assert_eq!((shadow_x.x, shadow_x.y), (0.0, 0.0));

        // 出向影子创建成功路径：建边 B→Z 后在画布 b 内创建 Z 的出向影子。
        // 新规则下出向影子只能是画布节点的影子，因此 Z 必须是画布节点（B→Z 为画布→画布）。
        // 无非影子节点时出向车道取默认 x=400，首个出向影子 y=0。
        let node_z = node::service::create(&root.id, "canvas-z".to_string(), String::new(), 400.0, 0.0, None, true).unwrap();
        let edge_bz = edge::service::create(&root.id, &node_b.id, "right".to_string(), &node_z.id, "left".to_string(), false).unwrap();
        let shadow_z = shadow_by_edge(&edge_bz.id).unwrap();
        assert_eq!((shadow_z.x, shadow_z.y), (400.0, 0.0));

        // 同向影子垂直堆叠：第二个入向影子（X2→B）落在第一个入向影子下方 y+120。
        let node_x2 = node::service::create(&root.id, "origin-x2".to_string(), String::new(), 0.0, 200.0, None, false).unwrap();
        let edge_x2b = edge::service::create(&root.id, &node_x2.id, "right".to_string(), &node_b.id, "left".to_string(), false).unwrap();
        let shadow_x2 = shadow_by_edge(&edge_x2b.id).unwrap();
        assert_eq!((shadow_x2.x, shadow_x2.y), (0.0, 120.0));

        // 车道参考非影子内容：画布 b 内新建普通节点 N(1000, 500) 后，
        // 入向影子（X3→B）车道 x = 1000-400 = 600，堆叠 y = 240；出向影子（B→Z2）车道 x = 1000+400 = 1400，堆叠 y = 120。
        let node_n = node::service::create(&canvas_b, "internal-n".to_string(), String::new(), 1000.0, 500.0, None, false).unwrap();
        let node_x3 = node::service::create(&root.id, "origin-x3".to_string(), String::new(), 0.0, 400.0, None, false).unwrap();
        let edge_x3b = edge::service::create(&root.id, &node_x3.id, "right".to_string(), &node_b.id, "left".to_string(), false).unwrap();
        let shadow_x3 = shadow_by_edge(&edge_x3b.id).unwrap();
        assert_eq!((shadow_x3.x, shadow_x3.y), (600.0, 240.0));
        let node_z2 = node::service::create(&root.id, "canvas-z2".to_string(), String::new(), 800.0, 0.0, None, true).unwrap();
        let edge_bz2 = edge::service::create(&root.id, &node_b.id, "right".to_string(), &node_z2.id, "left".to_string(), false).unwrap();
        let shadow_z2 = shadow_by_edge(&edge_bz2.id).unwrap();
        assert_eq!((shadow_z2.x, shadow_z2.y), (1400.0, 120.0));

        // 画布节点→画布节点成功路径（建边规则 4，已放开）：Y 与 B 都是画布节点，建边 Y→B
        // 在 Y.canvas_ref_id 画布内产生 B 的出向影子。注意旧版此情形报 CanvasToCanvasEdge，
        // 新版允许并联动创建影子。
        let node_y = node::service::create(&root.id, "canvas-y".to_string(), String::new(), 600.0, 0.0, None, true).unwrap();
        let canvas_y = node_y.canvas_ref_id.clone().unwrap();
        let edge_yb = edge::service::create(&root.id, &node_y.id, "right".to_string(), &node_b.id, "left".to_string(), false).unwrap();
        let shadow_b_in_y = shadow_by_edge(&edge_yb.id).unwrap();
        assert_eq!(shadow_b_in_y.canvas_id, canvas_y);
        // B 本身是画布节点，所以这是 Outflow 影子。
        assert_eq!(
            node::service::list(&canvas_y, false).unwrap().iter().find(|n| n.id == shadow_b_in_y.id).unwrap().shadow_direction,
            Some(shadow::vo::ShadowDirection::Outflow)
        );

        // list 视图断言：影子展示数据合并自根本体节点且方向正确。
        let nodes_b = node::service::list(&canvas_b, false).unwrap();
        let vo_z = nodes_b.iter().find(|n| n.id == shadow_z.id).unwrap();
        assert_eq!(vo_z.shadow_direction, Some(shadow::vo::ShadowDirection::Outflow));

        // 方向守卫失败路径：出向影子不允许作为源（建边规则反向约束）。
        assert!(matches!(
            edge::service::create(&canvas_b, &shadow_z.id, "right".to_string(), &node_n.id, "left".to_string(), false),
            Err(ErrorCode::InvalidShadowEdge)
        ));
        // 方向守卫失败路径：入向影子不允许作为目标。
        assert!(matches!(
            edge::service::create(&canvas_b, &node_n.id, "right".to_string(), &shadow_x.id, "left".to_string(), false),
            Err(ErrorCode::InvalidShadowEdge)
        ));
        // 影子与画布节点连线成功路径（建边规则 7）：入向影子 shadow_x 作为源连接画布节点 C2，
        // 在画布 c2 内创建 shadow_x 的入向影子（嵌套影子，shadow_id 指向直接来源 shadow_x 的产生边）；
        // 画布节点 C2 作为源连接出向影子 shadow_z，在画布 c2 内创建 shadow_z 的出向影子。
        let node_c2 = node::service::create(&canvas_b, "canvas-c2".to_string(), String::new(), 1200.0, 600.0, None, true).unwrap();
        let canvas_c2 = node_c2.canvas_ref_id.clone().unwrap();
        let edge_sxc2 = edge::service::create(&canvas_b, &shadow_x.id, "top".to_string(), &node_c2.id, "bottom".to_string(), false).unwrap();
        let nested_shadow_x = shadow_by_edge(&edge_sxc2.id).unwrap();
        assert_eq!(nested_shadow_x.canvas_id, canvas_c2);
        assert_eq!(nested_shadow_x.shadow_id.as_deref(), Some(edge_sxc2.id.as_str()));
        let edge_c2sz = edge::service::create(&canvas_b, &node_c2.id, "top".to_string(), &shadow_z.id, "bottom".to_string(), false).unwrap();
        let nested_shadow_z = shadow_by_edge(&edge_c2sz.id).unwrap();
        assert_eq!(nested_shadow_z.canvas_id, canvas_c2);
        assert_eq!(nested_shadow_z.shadow_id.as_deref(), Some(edge_c2sz.id.as_str()));

        // 嵌套影子 list 视图级联合并：c2 内 shadow_x 的影子展示数据沿影子链级联到根原始节点 X
        // （title/sub_title/color 合并自 X；影子的 canvas_ref_id 恒为 None）；
        // shadow_direction 按直接来源推导（产生边源端是入向影子 shadow_x 本身）。
        let nodes_c2 = node::service::list(&canvas_c2, false).unwrap();
        let vo_nested_x = nodes_c2.iter().find(|n| n.id == nested_shadow_x.id).unwrap();
        assert_eq!(vo_nested_x.title, "origin-x");
        assert!(vo_nested_x.sub_title.is_empty());
        assert!(vo_nested_x.canvas_ref_id.is_none());
        assert_eq!(vo_nested_x.shadow_direction, Some(shadow::vo::ShadowDirection::Inflow));
        assert_eq!(vo_nested_x.shadow_origin_deleted, Some(false));
        let vo_nested_z = nodes_c2.iter().find(|n| n.id == nested_shadow_z.id).unwrap();
        assert_eq!(vo_nested_z.title, "canvas-z");
        assert_eq!(vo_nested_z.shadow_direction, Some(shadow::vo::ShadowDirection::Outflow));

        // 影子参与连线成功路径：入向影子作为源连接普通节点、普通节点连接出向影子。
        let edge_sx_n = edge::service::create(&canvas_b, &shadow_x.id, "right".to_string(), &node_n.id, "left".to_string(), false).unwrap();
        let edge_n_sz = edge::service::create(&canvas_b, &node_n.id, "right".to_string(), &shadow_z.id, "left".to_string(), false).unwrap();
        // 日志载荷成功路径：影子端点的标题落库为空串，日志沿产生边链解析为根本体标题
        // （嵌套影子链同样解析到根本体）。
        let logs = log::service::list(0, 1000, LogFilter::default()).unwrap();
        let has_edge_create = |expect_source: &str, expect_target: &str| {
            logs.items
                .iter()
                .any(|entry| matches!(&entry.action, entity::Action::EdgeCreate { source_title, target_title }
                    if source_title == expect_source && target_title == expect_target))
        };
        assert!(has_edge_create("origin-x", "canvas-c2"));
        assert!(has_edge_create("canvas-c2", "canvas-z"));
        assert!(has_edge_create("origin-x", "internal-n"));
        assert!(has_edge_create("internal-n", "canvas-z"));
        // 影子-影子互连失败路径：入向影子 shadow_x 连接出向影子 shadow_z，报 ShadowToShadowEdge
        // （影子-影子拦截先于方向约束）。
        assert!(matches!(
            edge::service::create(&canvas_b, &shadow_x.id, "right".to_string(), &shadow_z.id, "left".to_string(), false),
            Err(ErrorCode::ShadowToShadowEdge)
        ));
        // 普通节点连接画布节点成功路径：N→C2 在画布 c2 内创建 N 的入向影子。
        // c2 内已有的内容都是影子（影子车道参考只看非影子内容），所以入向车道取默认 x=0，
        // 堆叠在已有入向嵌套影子（nested_shadow_x，y=0）下方 y=120。
        let edge_n_c2 = edge::service::create(&canvas_b, &node_n.id, "right".to_string(), &node_c2.id, "left".to_string(), false).unwrap();
        let shadow_n = shadow_by_edge(&edge_n_c2.id).unwrap();
        assert_eq!((shadow_n.x, shadow_n.y), (0.0, 120.0));

        // 既有行为不回归：根画布内普通节点之间建边成功，且不产生任何影子（各画布影子数不变）。
        edge::service::create(&root.id, &node_x.id, "right".to_string(), &node_x2.id, "left".to_string(), false).unwrap();
        let count_shadows = |canvas_id: &str| {
            node::service::list(canvas_id, false)
                .unwrap()
                .into_iter()
                .filter(|n| n.shadow_id.is_some())
                .count()
        };
        // canvas_b：X 的入向、X2 的入向、X3 的入向、Z 的出向、Z2 的出向，共 5 个影子。
        assert_eq!(count_shadows(&canvas_b), 5);
        // canvas_y：B 的出向（Y→B 产生），共 1 个影子。
        assert_eq!(count_shadows(&canvas_y), 1);
        // canvas_c2：nested_shadow_x（入向）、nested_shadow_z（出向）、shadow_n（入向），共 3 个影子。
        assert_eq!(count_shadows(&canvas_c2), 3);
        // 既有校验不回归：替换语义下重复边走"删旧建新"路径，不再报 EdgeAlreadyExists；
        // 端到端验证同向同连接桩仍被 EdgeSameNodePort 拦截。
        assert!(matches!(
            edge::service::create(&root.id, &node_x.id, "right".to_string(), &node_x2.id, "right".to_string(), false),
            Err(ErrorCode::EdgeSameNodePort)
        ));
        // 自环无旧边时仍报 EdgeWouldFormCycle，覆盖 cycle 检查先于 replace 的语义。
        let node_x4 = node::service::create(&root.id, "origin-x4".to_string(), String::new(), 0.0, 600.0, None, false).unwrap();
        assert!(matches!(
            edge::service::create(&root.id, &node_x4.id, "right".to_string(), &node_x4.id, "left".to_string(), false),
            Err(ErrorCode::EdgeWouldFormCycle)
        ));

        // 日志载荷成功路径：影子端点边的更新与删除日志同样沿产生边链解析根本体标题。
        edge::service::update(&edge_sx_n.id, "edge-title".to_string(), String::new()).unwrap();
        edge::service::delete(&edge_n_sz.id, false).unwrap();
        let logs = log::service::list(0, 1000, LogFilter::default()).unwrap();
        assert!(logs.items.iter().any(|entry| matches!(
            &entry.action,
            entity::Action::EdgeUpdate { source_title, target_title, new_title, .. }
                if source_title == "origin-x" && target_title == "internal-n" && new_title == "edge-title"
        )));
        assert!(logs.items.iter().any(|entry| matches!(
            &entry.action,
            entity::Action::EdgePhysicalDelete { source_title, target_title }
                if source_title == "internal-n" && target_title == "canvas-z"
        )));

        // 保存并关闭数据库，清理测试数据目录。
        lifecycle::service::save().unwrap();
        lifecycle::service::close().unwrap();
        test::cleanup(&path);
    }

    /// 边删除的影子节点联动：有连接未确认时拒绝删除并给出受影响节点标题、确认后影子随边物理删除、
    /// 无连接时直接删除、出向影子的入边同样触发确认、原始节点物理删除时影子随外键级联删除。
    #[test]
    fn test_shadow_node_edge_delete() {
        let _guard = test::acquire_test_lock();

        // 初始化测试数据目录、metadata 数据库并打开一个全新的用户数据库。
        let path = test::create_test_path();
        crate::state::set_path(path.clone());
        metadata::service::initialize().unwrap();
        let registered = metadata::service::register("shadow-edge-del-test-db".to_string()).unwrap();
        lifecycle::service::initialize(&registered.id, test::test_key()).unwrap();
        let canvases = canvas::service::list(false).unwrap();
        let root = canvases[0].clone();

        // 影子行查询辅助：按产生边 id 从 connection 上取影子节点本体。
        let shadow_by_edge = |edge_id: &str| {
            let connection = state::lock_connection();
            shadow::dao::select_by_producing_edge_id(&connection, edge_id).unwrap()
        };

        // 准备：根画布内普通节点 X 与画布节点 B（引用画布 b），建边 X→B 自动创建入向影子。
        let node_x = node::service::create(&root.id, "origin-x".to_string(), String::new(), 0.0, 0.0, None, false).unwrap();
        let node_b = node::service::create(&root.id, "canvas-b".to_string(), String::new(), 200.0, 0.0, None, true).unwrap();
        let canvas_b = node_b.canvas_ref_id.clone().unwrap();
        let edge_xb = edge::service::create(&root.id, &node_x.id, "right".to_string(), &node_b.id, "left".to_string(), false).unwrap();
        let shadow_x = shadow_by_edge(&edge_xb.id).unwrap();

        // 画布 b 内建普通节点 M1、M2，并建边 shadow_x→M1、shadow_x→M2（入向影子有出边）。
        let node_m1 = node::service::create(&canvas_b, "internal-m1".to_string(), String::new(), 400.0, 0.0, None, false).unwrap();
        let node_m2 = node::service::create(&canvas_b, "internal-m2".to_string(), String::new(), 400.0, 200.0, None, false).unwrap();
        edge::service::create(&canvas_b, &shadow_x.id, "right".to_string(), &node_m1.id, "left".to_string(), false).unwrap();
        edge::service::create(&canvas_b, &shadow_x.id, "right".to_string(), &node_m2.id, "left".to_string(), false).unwrap();

        // 失败路径：入向影子有出边且未确认时，删除边报 EdgeDeleteDisconnectsNodes，
        // 载荷为受影响节点标题列表；边与影子均保持存在。
        let Err(ErrorCode::EdgeDeleteDisconnectsNodes { nodes: affected }) =
            edge::service::delete(&edge_xb.id, false)
        else {
            panic!("expected EdgeDeleteDisconnectsNodes");
        };
        assert_eq!(affected.len(), 2);
        assert!(affected.contains(&"internal-m1".to_string()));
        assert!(affected.contains(&"internal-m2".to_string()));
        assert!(edge::service::list(&root.id).unwrap().iter().any(|e| e.id == edge_xb.id));
        assert!(shadow_by_edge(&edge_xb.id).is_some());

        // 成功路径（确认后）：边被删除，影子节点随边物理删除（经 shadow_id 外键级联），
        // 影子在子画布内的出边由 edge.source_id/target_id 外键级联删除，
        // 子画布内的普通节点 M1/M2 本身保留。
        edge::service::delete(&edge_xb.id, true).unwrap();
        assert!(!edge::service::list(&root.id).unwrap().iter().any(|e| e.id == edge_xb.id));
        assert!(shadow_by_edge(&edge_xb.id).is_none());
        assert!(edge::service::list(&canvas_b).unwrap().is_empty());
        assert!(node::service::list(&canvas_b, false).unwrap().iter().any(|n| n.id == node_m1.id));
        assert!(node::service::list(&canvas_b, false).unwrap().iter().any(|n| n.id == node_m2.id));

        // 无连接快速路径：出向影子（B→Z）在子画布内没有任何关联边时，未确认也直接删除成功。
        // 新规则下产生出向影子须用画布节点 Z（建边 B→Z：画布→画布）。
        let node_z = node::service::create(&root.id, "canvas-z".to_string(), String::new(), 400.0, 0.0, None, true).unwrap();
        let edge_bz = edge::service::create(&root.id, &node_b.id, "right".to_string(), &node_z.id, "left".to_string(), false).unwrap();
        assert!(shadow_by_edge(&edge_bz.id).is_some());
        edge::service::delete(&edge_bz.id, false).unwrap();
        assert!(shadow_by_edge(&edge_bz.id).is_none());

        // 出向影子有入边同样触发确认：重建 B→Z（重新产生出向影子），建边 M1→shadow_z（影子有入边）。
        let edge_bz2 = edge::service::create(&root.id, &node_b.id, "right".to_string(), &node_z.id, "left".to_string(), false).unwrap();
        let shadow_z = shadow_by_edge(&edge_bz2.id).unwrap();
        edge::service::create(&canvas_b, &node_m1.id, "right".to_string(), &shadow_z.id, "left".to_string(), false).unwrap();
        let Err(ErrorCode::EdgeDeleteDisconnectsNodes { nodes: affected }) =
            edge::service::delete(&edge_bz2.id, false)
        else {
            panic!("expected EdgeDeleteDisconnectsNodes");
        };
        assert_eq!(affected, vec!["internal-m1".to_string()]);
        // 确认后删除：影子与它的入边一并消失。
        edge::service::delete(&edge_bz2.id, true).unwrap();
        assert!(shadow_by_edge(&edge_bz2.id).is_none());
        assert!(edge::service::list(&canvas_b).unwrap().is_empty());

        // 物理删除联动（端到端）：物理删除原始节点 X2 时，其入向影子与父画布内的边随外键级联一并消失。
        let node_x2 = node::service::create(&root.id, "origin-x2".to_string(), String::new(), 0.0, 200.0, None, false).unwrap();
        let edge_x2b = edge::service::create(&root.id, &node_x2.id, "right".to_string(), &node_b.id, "left".to_string(), false).unwrap();
        assert!(shadow_by_edge(&edge_x2b.id).is_some());
        node::service::physical_delete(&node_x2.id, false).unwrap();
        assert!(shadow_by_edge(&edge_x2b.id).is_none());
        assert!(!edge::service::list(&root.id).unwrap().iter().any(|e| e.id == edge_x2b.id));

        // 保存并关闭数据库，清理测试数据目录。
        lifecycle::service::save().unwrap();
        lifecycle::service::close().unwrap();
        test::cleanup(&path);
    }

    /// 嵌套影子（影子的影子）的创建、展示数据级联合并、删边递归断连检测与级联删除、
    /// 物理删除节点的双阶段确认。
    #[test]
    fn test_shadow_node_nested() {
        let _guard = test::acquire_test_lock();

        // 初始化测试数据目录、metadata 数据库并打开一个全新的用户数据库。
        let path = test::create_test_path();
        crate::state::set_path(path.clone());
        metadata::service::initialize().unwrap();
        let registered = metadata::service::register("shadow-nested-test-db".to_string()).unwrap();
        lifecycle::service::initialize(&registered.id, test::test_key()).unwrap();
        let canvases = canvas::service::list(false).unwrap();
        let root = canvases[0].clone();

        // 按画布 id 列出所有影子（包含嵌套）的辅助函数：新机制下影子由产生边唯一标识，
        // 通过 list 取得影子节点本体后用 shadow_origin_id 沿链向上找到根本体 X。
        let list_shadows = |canvas_id: &str| -> Vec<node::vo::NodeVO> {
            node::service::list(canvas_id, false)
                .unwrap()
                .into_iter()
                .filter(|n| n.shadow_id.is_some())
                .collect()
        };

        // ===== 第 1 阶段：构造嵌套影子 X → X_b → X_bc =====
        // 根画布：普通节点 X（X 是普通节点，canvas_ref_id 为 None）。
        let node_x = node::service::create(&root.id, "X 的标题".to_string(), "X 的副标题".to_string(), 0.0, 0.0, None, false).unwrap();
        // 画布节点 B 引用画布 b。
        let node_b = node::service::create(&root.id, "B 的标题".to_string(), String::new(), 200.0, 0.0, None, true).unwrap();
        let canvas_b = node_b.canvas_ref_id.clone().unwrap();

        // 建边 X→B → 画布 b 内产生 X 的入向影子 X_b。
        let edge_xb = edge::service::create(&root.id, &node_x.id, "right".to_string(), &node_b.id, "left".to_string(), false).unwrap();
        let shadows_b = list_shadows(&canvas_b);
        assert_eq!(shadows_b.len(), 1);
        let shadow_x_b = shadows_b.into_iter().next().unwrap();
        // 影子 shadow_id 指向产生边 edge_xb.id，根本体 id 通过 shadow_origin_id 给出。
        assert_eq!(shadow_x_b.shadow_id.as_deref(), Some(edge_xb.id.as_str()));
        assert_eq!(shadow_x_b.shadow_origin_id.as_deref(), Some(node_x.id.as_str()));

        // 画布 b 内：画布节点 C 引用画布 c、普通节点 M。
        let node_c = node::service::create(&canvas_b, "C 的标题".to_string(), String::new(), 600.0, 0.0, None, true).unwrap();
        let canvas_c = node_c.canvas_ref_id.clone().unwrap();
        let node_m = node::service::create(&canvas_b, "M 的标题".to_string(), String::new(), 400.0, 0.0, None, false).unwrap();

        // 入向影子作源连接普通节点成功路径：X_b→M（入向影子有出边）。
        edge::service::create(&canvas_b, &shadow_x_b.id, "right".to_string(), &node_m.id, "left".to_string(), false).unwrap();

        // 入向影子作源连接画布节点成功路径：X_b→C → 画布 c 内产生 X_b 的入向影子 X_bc（嵌套影子，
        // 其 shadow_id 直接指向产生边 edge_xbc，根本体通过 shadow_origin_id 仍是 X）。
        let edge_xbc = edge::service::create(&canvas_b, &shadow_x_b.id, "top".to_string(), &node_c.id, "bottom".to_string(), false).unwrap();
        let shadows_c = list_shadows(&canvas_c);
        assert_eq!(shadows_c.len(), 1);
        let shadow_x_bc = shadows_c.into_iter().next().unwrap();
        assert_eq!(shadow_x_bc.shadow_id.as_deref(), Some(edge_xbc.id.as_str()));
        assert_eq!(shadow_x_bc.shadow_origin_id.as_deref(), Some(node_x.id.as_str()));

        // 画布 c 内：普通节点 P；嵌套入向影子作源连接普通节点成功路径：X_bc→P。
        let node_p = node::service::create(&canvas_c, "P 的标题".to_string(), String::new(), 200.0, 0.0, None, false).unwrap();
        edge::service::create(&canvas_c, &shadow_x_bc.id, "right".to_string(), &node_p.id, "left".to_string(), false).unwrap();

        // ===== 第 2 阶段：list 视图级联合并断言 =====
        // X_bc 展示数据沿影子链向上级联到根原始节点 X（而非停留在 shadow_x_b 这一层）：
        // title / sub_title / color 合并为 X 的值；canvas_ref_id 恒为 None（X 是普通节点）；
        // shadow_direction 按直接来源推导为 Inflow。
        let nodes_c_list = node::service::list(&canvas_c, false).unwrap();
        let vo_x_bc = nodes_c_list.iter().find(|n| n.id == shadow_x_bc.id).unwrap();
        assert_eq!(vo_x_bc.title, "X 的标题");
        assert_eq!(vo_x_bc.sub_title, "X 的副标题");
        assert!(vo_x_bc.canvas_ref_id.is_none());
        assert_eq!(vo_x_bc.shadow_origin_id.as_deref(), Some(node_x.id.as_str()));
        assert_eq!(vo_x_bc.shadow_origin_deleted, Some(false));
        assert_eq!(vo_x_bc.shadow_direction, Some(shadow::vo::ShadowDirection::Inflow));

        // ===== 第 3 阶段：删边递归断连检测失败路径 =====
        // 删除边 X→B（未确认）应递归覆盖两层画布：
        // 第一层画布 b 内 X_b 的出边邻居为 M 与 C（均为受影响邻居）；
        // 第二层画布 c 内 X_bc 的出边邻居为 P；
        // 因此 affected 包含 M 的标题、C 的标题、P 的标题。
        let Err(ErrorCode::EdgeDeleteDisconnectsNodes { nodes: affected }) =
            edge::service::delete(&edge_xb.id, false)
        else {
            panic!("expected EdgeDeleteDisconnectsNodes");
        };
        assert_eq!(affected.len(), 3);
        assert!(affected.contains(&"M 的标题".to_string()));
        assert!(affected.contains(&"C 的标题".to_string()));
        assert!(affected.contains(&"P 的标题".to_string()));
        // 边与两级影子均保持存在。
        assert!(edge::service::list(&root.id).unwrap().iter().any(|e| e.id == edge_xb.id));
        assert!(list_shadows(&canvas_b).iter().any(|n| n.id == shadow_x_b.id));
        assert!(list_shadows(&canvas_c).iter().any(|n| n.id == shadow_x_bc.id));

        // ===== 第 4 阶段：确认后删边成功路径 =====
        edge::service::delete(&edge_xb.id, true).unwrap();
        // X→B 边消失。
        assert!(!edge::service::list(&root.id).unwrap().iter().any(|e| e.id == edge_xb.id));
        // X_b 与 X_bc 均被外键级联删除（影子链整体消失）。
        assert!(list_shadows(&canvas_b).is_empty());
        assert!(list_shadows(&canvas_c).is_empty());
        // 画布 b 与画布 c 内的边因节点删除被级联清空。
        assert!(edge::service::list(&canvas_b).unwrap().is_empty());
        assert!(edge::service::list(&canvas_c).unwrap().is_empty());
        // M / C / P 节点本身保留（只是断开了与影子的连接）。
        assert!(node::service::list(&canvas_b, false).unwrap().iter().any(|n| n.id == node_m.id));
        assert!(node::service::list(&canvas_b, false).unwrap().iter().any(|n| n.id == node_c.id));
        assert!(node::service::list(&canvas_c, false).unwrap().iter().any(|n| n.id == node_p.id));

        // ===== 第 5 阶段：物理删除双阶段确认 =====
        // 重建类似结构（X2→B2、X2_b2→C2、X2_bc2→P2）。
        let node_x2 = node::service::create(&root.id, "X2 的标题".to_string(), String::new(), 0.0, 200.0, None, false).unwrap();
        let node_b2 = node::service::create(&root.id, "B2 的标题".to_string(), String::new(), 200.0, 200.0, None, true).unwrap();
        let canvas_b2 = node_b2.canvas_ref_id.clone().unwrap();
        let edge_x2b2 = edge::service::create(&root.id, &node_x2.id, "right".to_string(), &node_b2.id, "left".to_string(), false).unwrap();
        let shadows_b2 = list_shadows(&canvas_b2);
        assert_eq!(shadows_b2.len(), 1);
        let shadow_x_b2 = shadows_b2.into_iter().next().unwrap();
        let node_c2 = node::service::create(&canvas_b2, "C2 的标题".to_string(), String::new(), 400.0, 0.0, None, true).unwrap();
        let canvas_c2 = node_c2.canvas_ref_id.clone().unwrap();
        let node_m2 = node::service::create(&canvas_b2, "M2 的标题".to_string(), String::new(), 600.0, 0.0, None, false).unwrap();
        edge::service::create(&canvas_b2, &shadow_x_b2.id, "right".to_string(), &node_m2.id, "left".to_string(), false).unwrap();
        let edge_x_b2_c2 = edge::service::create(&canvas_b2, &shadow_x_b2.id, "top".to_string(), &node_c2.id, "bottom".to_string(), false).unwrap();
        let shadows_c2 = list_shadows(&canvas_c2);
        assert_eq!(shadows_c2.len(), 1);
        let shadow_x_bc2 = shadows_c2.into_iter().next().unwrap();
        let node_p2 = node::service::create(&canvas_c2, "P2 的标题".to_string(), String::new(), 200.0, 0.0, None, false).unwrap();
        edge::service::create(&canvas_c2, &shadow_x_bc2.id, "right".to_string(), &node_p2.id, "left".to_string(), false).unwrap();
        // 抑制 unused 变量警告：edge_x2b2 用来验证后续物理删除时边被外键级联。
        let _ = (edge_x2b2, edge_x_b2_c2);

        // 失败路径：未确认时返回 NodeDeleteDisconnectsNodes，载荷含两层受影响标题。
        let Err(ErrorCode::NodeDeleteDisconnectsNodes { nodes: affected }) =
            node::service::physical_delete(&node_x2.id, false)
        else {
            panic!("expected NodeDeleteDisconnectsNodes");
        };
        assert_eq!(affected.len(), 3);
        assert!(affected.contains(&"M2 的标题".to_string()));
        assert!(affected.contains(&"C2 的标题".to_string()));
        assert!(affected.contains(&"P2 的标题".to_string()));
        // 节点与各级影子保持存在。
        assert!(node::dao::select_by_id(&state::lock_connection(), &node_x2.id)
            .unwrap()
            .is_some());
        assert!(list_shadows(&canvas_b2).iter().any(|n| n.id == shadow_x_b2.id));
        assert!(list_shadows(&canvas_c2).iter().any(|n| n.id == shadow_x_bc2.id));

        // 成功路径：确认后 X2 的全部后代影子与相关边均被级联删除；M2/C2/P2 保留。
        node::service::physical_delete(&node_x2.id, true).unwrap();
        assert!(node::dao::select_by_id(&state::lock_connection(), &node_x2.id)
            .unwrap()
            .is_none());
        assert!(list_shadows(&canvas_b2).is_empty());
        assert!(list_shadows(&canvas_c2).is_empty());
        assert!(edge::service::list(&canvas_b2).unwrap().is_empty());
        assert!(edge::service::list(&canvas_c2).unwrap().is_empty());
        assert!(node::service::list(&canvas_b2, false).unwrap().iter().any(|n| n.id == node_m2.id));
        assert!(node::service::list(&canvas_b2, false).unwrap().iter().any(|n| n.id == node_c2.id));
        assert!(node::service::list(&canvas_c2, false).unwrap().iter().any(|n| n.id == node_p2.id));

        // 保存并关闭数据库，清理测试数据目录。
        lifecycle::service::save().unwrap();
        lifecycle::service::close().unwrap();
        test::cleanup(&path);
    }
}
