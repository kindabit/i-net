use super::*;
use crate::business::metadata;
use crate::business::user_database::entity::Dictionary;
use crate::business::user_database::field_type::FieldValue;
use crate::business::user_database::node_field::vo::NodeFieldVO;
use crate::business::user_database::template::vo::TemplateFieldVO;
use crate::error_code::ErrorCode;
use crate::test;
use crate::util::file_system_util;

/// 模拟用户一次完整操作会话：注册数据库后，依次经历
/// 生命周期与画布/节点/边/注册表/视口/日志、node_field 与 dictionary、template、
/// attachment 四个阶段的操作，覆盖 user_database 模块所有子业务模块的所有 service 函数的
/// 成功与失败路径；数据库随会话推进多次打开与关闭。
#[test]
fn test_user_database_service_all_functions() {
    let _guard = test::acquire_test_lock();

    // 每个测试都在自己的数据目录下进行，初始化自己的数据目录和 metadata 数据库。
    let path = test::create_test_path();
    crate::state::set_path(path.clone());
    metadata::service::initialize().unwrap();

    // 注册一个用户数据库，拿到它的 id；register 只添加记录，不会实际创建数据库目录。
    let registered = metadata::service::register("db-1".to_string()).unwrap();

    // == 阶段一：lifecycle / canvas / node / edge / registry / viewport / log ==
    {
    let id = registered.id.clone();
    assert!(!file_system_util::try_exists(&path.user_database_directory(&id)).unwrap());

    // lifecycle::initialize 失败路径：id 不存在时报 NoDatabaseWithSuchId。
    assert!(matches!(
        lifecycle::service::initialize("no-such-id", test::test_key()),
        Err(ErrorCode::NoDatabaseWithSuchId { .. })
    ));

    // lifecycle::initialize 成功路径（新建）：数据库目录、附件目录和加密的数据库文件被创建，
    // 返回的元数据最后打开时间不早于注册时，state 处于打开状态，且根画布已插入 canvas 表。
    let opened = lifecycle::service::initialize(&id, test::test_key()).unwrap();
    assert!(opened.last_open_time >= registered.last_open_time);
    assert!(file_system_util::try_exists(&path.user_database_directory(&id)).unwrap());
    assert!(file_system_util::try_exists(&path.user_attachment_directory(&id)).unwrap());
    assert!(file_system_util::try_exists(&path.user_database_file(&id)).unwrap());
    assert!(state::is_open());
    // state 中保存的是更新过最后打开时间的元信息。
    assert_eq!(state::metadata().last_open_time, opened.last_open_time);
    let canvases = canvas::service::list(false).unwrap();
    assert_eq!(canvases.len(), 1);
    let root = canvases[0].clone();
    assert_eq!(root.name, entity::ROOT_CANVAS_NAME);
    assert!(root.parent_id.is_none());
    assert!(!root.deleted);

    // canvas::create 失败路径：父画布不存在时报 NoCanvasWithSuchId。
    assert!(matches!(
        canvas::service::create(&uuid::Uuid::new_v4().to_string(), "child-x".to_string()),
        Err(ErrorCode::NoCanvasWithSuchId { .. })
    ));

    // canvas::create 成功路径：在根画布下新建子画布，简易 layout 将其放在根画布附近（距离不小于 200）。
    let child = canvas::service::create(&root.id, "child".to_string()).unwrap();
    assert_eq!(child.parent_id.as_deref(), Some(root.id.as_str()));
    assert!(!child.deleted);
    assert!((child.x * child.x + child.y * child.y).sqrt() >= 200.0);

    // canvas::create 失败路径：名称重复时报 CanvasNameAlreadyExists。
    assert!(matches!(
        canvas::service::create(&root.id, "child".to_string()),
        Err(ErrorCode::CanvasNameAlreadyExists { .. })
    ));

    // canvas::move_canvas 失败路径：画布不存在时报 NoCanvasWithSuchId。
    assert!(matches!(
        canvas::service::move_canvas("no-such-id", 0.0, 0.0),
        Err(ErrorCode::NoCanvasWithSuchId { .. })
    ));

    // canvas::move_canvas 成功路径：子画布坐标被更新为 (1000, 1000)。
    canvas::service::move_canvas(&child.id, 1000.0, 1000.0).unwrap();
    let moved = canvas::service::list(false)
        .unwrap()
        .into_iter()
        .find(|canvas| canvas.id == child.id)
        .unwrap();
    assert_eq!((moved.x, moved.y), (1000.0, 1000.0));
    // canvas::move_canvas 成功路径（原地移动）：新坐标与旧坐标相同时成功返回，
    // 且不产生新日志（日志总数不变）。
    let log_total_before = log::service::list(0, 1).unwrap().total;
    canvas::service::move_canvas(&child.id, 1000.0, 1000.0).unwrap();
    assert_eq!(log::service::list(0, 1).unwrap().total, log_total_before);

    // canvas::create 成功路径：在子画布下新建孙画布，layout 以子画布新坐标为圆心。
    let grandchild = canvas::service::create(&child.id, "grandchild".to_string()).unwrap();
    assert_eq!(grandchild.parent_id.as_deref(), Some(child.id.as_str()));

    // canvas::rename 失败路径：画布不存在时报 NoCanvasWithSuchId；
    // 新名称与其它画布重复时报 CanvasNameAlreadyExists。
    assert!(matches!(
        canvas::service::rename("no-such-id", "x".to_string()),
        Err(ErrorCode::NoCanvasWithSuchId { .. })
    ));
    assert!(matches!(
        canvas::service::rename(&grandchild.id, "child".to_string()),
        Err(ErrorCode::CanvasNameAlreadyExists { .. })
    ));

    // canvas::rename 成功路径：名称被更新。
    canvas::service::rename(&grandchild.id, "grandchild-renamed".to_string()).unwrap();
    let renamed = canvas::service::list(false)
        .unwrap()
        .into_iter()
        .find(|canvas| canvas.id == grandchild.id)
        .unwrap();
    assert_eq!(renamed.name, "grandchild-renamed");

    // canvas::logical_delete 失败路径：画布不存在时报 NoCanvasWithSuchId。
    assert!(matches!(
        canvas::service::logical_delete("no-such-id"),
        Err(ErrorCode::NoCanvasWithSuchId { .. })
    ));

    // canvas::logical_delete 失败路径：目标是根画布时报 RootCanvasCannotBeDeleted。
    assert!(matches!(
        canvas::service::logical_delete(&root.id),
        Err(ErrorCode::RootCanvasCannotBeDeleted)
    ));

    // canvas::logical_delete 成功路径：子画布及其子孙画布一并被逻辑删除（进入已删除列表），根画布不受影响。
    canvas::service::logical_delete(&child.id).unwrap();
    let deleted = canvas::service::list(true).unwrap();
    assert!(deleted.iter().any(|canvas| canvas.id == child.id));
    assert!(deleted.iter().any(|canvas| canvas.id == grandchild.id));
    let normal = canvas::service::list(false).unwrap();
    assert!(normal.iter().all(|canvas| canvas.id != child.id));
    assert!(normal.iter().any(|canvas| canvas.id == root.id));

    // canvas::restore 失败路径：画布不存在时报 NoCanvasWithSuchId。
    assert!(matches!(
        canvas::service::restore("no-such-id", 0.0, 0.0),
        Err(ErrorCode::NoCanvasWithSuchId { .. })
    ));

    // canvas::restore 成功路径：恢复孙画布时祖先链上被逻辑删除的子画布一并恢复；
    // 孙画布移动到新坐标 (2000, 2000)，子画布跟随相同的位移。
    // 孙画布旧坐标为 (1240, 1000)（子画布 (1000, 1000) 第一圈 0 度方向半径 240），
    // 位移为 (760, 1000)，因此子画布的新坐标为 (1760, 2000)。
    canvas::service::restore(&grandchild.id, 2000.0, 2000.0).unwrap();
    let canvases = canvas::service::list(false).unwrap();
    let restored_grandchild = canvases
        .iter()
        .find(|canvas| canvas.id == grandchild.id)
        .unwrap();
    assert!(!restored_grandchild.deleted);
    assert_eq!(
        (restored_grandchild.x, restored_grandchild.y),
        (2000.0, 2000.0)
    );
    let restored_child = canvases
        .iter()
        .find(|canvas| canvas.id == child.id)
        .unwrap();
    assert!(!restored_child.deleted);
    assert_eq!((restored_child.x, restored_child.y), (1760.0, 2000.0));

    // canvas::restore 成功路径：目标未被逻辑删除时无操作直接成功，坐标保持不变。
    canvas::service::restore(&root.id, 123.0, 456.0).unwrap();
    let root_after = canvas::service::list(false)
        .unwrap()
        .into_iter()
        .find(|canvas| canvas.id == root.id)
        .unwrap();
    assert_eq!((root_after.x, root_after.y), (0.0, 0.0));

    // viewport::get 成功路径：未设置过时返回默认值；canvas_id 为 None 时表示画布宇宙视口。
    let universe = viewport::service::get(None).unwrap();
    assert_eq!(universe.canvas_id, entity::CANVAS_UNIVERSE_VIEWPORT_ID);
    assert_eq!((universe.x, universe.y, universe.zoom), (0.0, 0.0, 1.0));

    // viewport::set / get 成功路径：画布宇宙视口（None 路径）往返一致。
    viewport::service::set(None, 1.0, 2.0, 3.0).unwrap();
    let universe = viewport::service::get(None).unwrap();
    assert_eq!((universe.x, universe.y, universe.zoom), (1.0, 2.0, 3.0));

    // viewport::set / get 成功路径：指定画布的视口往返一致；未设置过的画布仍返回默认值。
    viewport::service::set(Some(child.id.clone()), 10.0, 20.0, 30.0).unwrap();
    let inner = viewport::service::get(Some(child.id.clone())).unwrap();
    assert_eq!(
        (inner.canvas_id, inner.x, inner.y, inner.zoom),
        (child.id.clone(), 10.0, 20.0, 30.0)
    );
    let other = viewport::service::get(Some(root.id.clone())).unwrap();
    assert_eq!((other.canvas_id, other.zoom), (root.id.clone(), 1.0));

    // registry::get 成功路径：变量不存在时返回 None。
    assert_eq!(registry::service::get("no-such-key").unwrap(), None);

    // registry::set / get 成功路径：写入后读出相同值；重复写入后读出更新值。
    registry::service::set("lastScene", "dark").unwrap();
    assert_eq!(
        registry::service::get("lastScene").unwrap(),
        Some("dark".to_string())
    );
    registry::service::set("lastScene", "light").unwrap();
    assert_eq!(
        registry::service::get("lastScene").unwrap(),
        Some("light".to_string())
    );

    // node::create 失败路径：画布不存在时报 NoCanvasWithSuchId。
    assert!(matches!(
        node::service::create(
            &uuid::Uuid::new_v4().to_string(),
            "title".to_string(),
            "sub".to_string(),
            0.0,
            0.0,
            None,
            false,
        ),
        Err(ErrorCode::NoCanvasWithSuchId { .. })
    ));

    // node::create 成功路径：在子画布内新建两个节点。
    let node_1 = node::service::create(
        &child.id,
        "title-1".to_string(),
        "sub-1".to_string(),
        10.0,
        20.0,
        None,
        false,
    )
    .unwrap();
    let node_2 = node::service::create(
        &child.id,
        "title-2".to_string(),
        "sub-2".to_string(),
        100.0,
        200.0,
        None,
        false,
    )
    .unwrap();
    assert_eq!(node_1.canvas_id, child.id);
    assert!(!node_1.deleted);

    // 在根画布内新建一个节点，用于测试跨画布建边的失败路径。
    let outsider = node::service::create(
        &root.id,
        "title-outsider".to_string(),
        "sub".to_string(),
        0.0,
        0.0,
        None,
        false,
    )
    .unwrap();

    // node::create 失败路径：create_canvas=true，宿主画布不存在时报 NoCanvasWithSuchId。
    assert!(matches!(
        node::service::create(
            &uuid::Uuid::new_v4().to_string(),
            "canvas-node".to_string(),
            String::new(),
            10.0,
            20.0,
            None,
            true,
        ),
        Err(ErrorCode::NoCanvasWithSuchId { .. })
    ));

    // node::create 成功路径：create_canvas=true，Node 落库且 canvas_ref_id 指向新建的画布，
    // canvas.parent_id == 宿主 id，title == 画布名。
    let canvas_node = node::service::create(
        &child.id,
        "canvas-node".to_string(),
        String::new(),
        50.0,
        60.0,
        None,
        true,
    )
    .unwrap();
    assert!(canvas_node.canvas_ref_id.is_some());
    assert_eq!((canvas_node.x, canvas_node.y), (50.0, 60.0));
    assert!(!canvas_node.deleted);
    let canvas_node_canvas = {
        let conn = state::lock_connection();
        let c = canvas::dao::select_by_id(
            &conn,
            canvas_node.canvas_ref_id.as_ref().unwrap(),
        )
        .unwrap()
        .unwrap();
        c
    };
    assert_eq!(canvas_node_canvas.parent_id.as_deref(), Some(child.id.as_str()));
    assert_eq!(canvas_node.title, canvas_node_canvas.name);
    assert!(!canvas_node_canvas.deleted);

    // node::create 名称去重：create_canvas=true，同宿主连续创建两次同名，第二次名称为 "基础名 2"。
    let node_2_name = node::service::create(
        &child.id,
        "canvas-node".to_string(),
        String::new(),
        70.0,
        80.0,
        None,
        true,
    )
    .unwrap();
    assert_eq!(node_2_name.title, "canvas-node 2");

    // node::create 失败路径：create_canvas=true，宿主画布已逻辑删除时报 NoCanvasWithSuchId。
    // 先新建一个临时子画布并逻辑删除它。
    let temp_canvas = canvas::service::create(&root.id, "temp-host".to_string()).unwrap();
    canvas::service::logical_delete(&temp_canvas.id).unwrap();
    assert!(matches!(
        node::service::create(
            &temp_canvas.id,
            "should-fail".to_string(),
            String::new(),
            0.0,
            0.0,
            None,
            true,
        ),
        Err(ErrorCode::NoCanvasWithSuchId { .. })
    ));
    // 清理临时画布。
    canvas::service::restore(&temp_canvas.id, temp_canvas.x, temp_canvas.y).unwrap();
    canvas::service::physical_delete(&temp_canvas.id).unwrap();

    // 构造用于后续双向级联测试的画布节点（基于 root 画布）。
    let cascade_node = node::service::create(
        &root.id,
        "cascade-canvas".to_string(),
        String::new(),
        100.0,
        100.0,
        None,
        true,
    )
    .unwrap();
    let cascade_canvas_id = cascade_node.canvas_ref_id.clone().unwrap();
    assert!(canvas::service::list(false)
        .unwrap()
        .iter()
        .any(|c| c.id == cascade_canvas_id));

    // 双向级联：node 逻辑删除 → 引用的子画布被逻辑删除。
    // 成功路径还校验返回的 Node 对象：id 一致、deleted == true、其余字段与原节点一致。
    let deleted_cascade = node::service::logical_delete(&cascade_node.id).unwrap();
    assert_eq!(deleted_cascade.id, cascade_node.id);
    assert!(deleted_cascade.deleted);
    assert_eq!(deleted_cascade.title, cascade_node.title);
    assert_eq!(deleted_cascade.sub_title, cascade_node.sub_title);
    assert_eq!(deleted_cascade.x, cascade_node.x);
    assert_eq!(deleted_cascade.y, cascade_node.y);
    assert_eq!(deleted_cascade.canvas_id, cascade_node.canvas_id);
    assert_eq!(deleted_cascade.canvas_ref_id, cascade_node.canvas_ref_id);
    let cascade_canvas_after = canvas::service::list(true)
        .unwrap()
        .into_iter()
        .find(|c| c.id == cascade_canvas_id)
        .unwrap();
    assert!(cascade_canvas_after.deleted);

    // 双向级联：node 恢复 → 引用的子画布恢复。
    node::service::restore(&cascade_node.id, 150.0, 150.0).unwrap();
    let cascade_canvas_restored = canvas::service::list(false)
        .unwrap()
        .into_iter()
        .find(|c| c.id == cascade_canvas_id)
        .unwrap();
    assert!(!cascade_canvas_restored.deleted);

    // 双向级联：canvas 逻辑删除 → 引用节点被逻辑删除。
    canvas::service::logical_delete(&cascade_canvas_id).unwrap();
    let ref_node_after = node::service::list(&root.id, true)
        .unwrap()
        .into_iter()
        .find(|n| n.id == cascade_node.id)
        .unwrap();
    assert!(ref_node_after.deleted);
    // 恢复以继续后续测试。
    canvas::service::restore(&cascade_canvas_id, cascade_canvas_restored.x, cascade_canvas_restored.y).unwrap();
    let ref_node_restored = node::service::list(&root.id, false)
        .unwrap()
        .into_iter()
        .find(|n| n.id == cascade_node.id)
        .unwrap();
    assert!(!ref_node_restored.deleted);

    // 双向级联：node 物理删除 → 引用的画布及其内容物理删除。
    node::service::physical_delete(&cascade_node.id).unwrap();
    assert!(canvas::service::list(false)
        .unwrap()
        .iter()
        .all(|c| c.id != cascade_canvas_id));
    assert!(node::service::list(&root.id, true)
        .unwrap()
        .iter()
        .all(|n| n.id != cascade_node.id));

    // 双向级联：canvas 物理删除 → 引用节点及其边物理删除。
    // 新建画布节点及其一条边，然后物理删除画布，验证节点和边一并消失。
    let c2_node = node::service::create(
        &root.id,
        "c2-canvas".to_string(),
        String::new(),
        200.0,
        200.0,
        None,
        true,
    )
    .unwrap();
    let c2_edge = edge::service::create(
        &root.id,
        &c2_node.id,
        "right".to_string(),
        &outsider.id,
        "left".to_string(),
    )
    .unwrap();
    let c2_node_id = c2_node.id.clone();
    let c2_edge_id = c2_edge.id.clone();
    let c2_canvas_id = c2_node.canvas_ref_id.clone().unwrap();
    canvas::service::physical_delete(&c2_canvas_id).unwrap();
    assert!(node::service::list(&root.id, false)
        .unwrap()
        .iter()
        .all(|n| n.id != c2_node_id));
    assert!(matches!(
        edge::service::delete(&c2_edge_id),
        Err(ErrorCode::NoEdgeWithSuchId { .. })
    ));
    assert!(canvas::service::list(false)
        .unwrap()
        .iter()
        .all(|c| c.id != c2_canvas_id));

    // edge::create 失败路径：节点不属于该画布时报 NoNodeWithSuchId；节点 id 不存在时同样报 NoNodeWithSuchId。
    assert!(matches!(
        edge::service::create(
            &child.id,
            &outsider.id,
            "right".to_string(),
            &node_1.id,
            "left".to_string()
        ),
        Err(ErrorCode::NoNodeWithSuchId { .. })
    ));
    assert!(matches!(
        edge::service::create(
            &child.id,
            &uuid::Uuid::new_v4().to_string(),
            "right".to_string(),
            &node_1.id,
            "left".to_string()
        ),
        Err(ErrorCode::NoNodeWithSuchId { .. })
    ));

    // edge::create 成功路径：node_1 -> node_2。
    let edge_1 = edge::service::create(
        &child.id,
        &node_1.id,
        "right".to_string(),
        &node_2.id,
        "left".to_string(),
    )
    .unwrap();
    assert_eq!(edge_1.source_id, node_1.id);
    assert_eq!(edge_1.target_id, node_2.id);

    // edge::update 失败路径：边不存在时报 NoEdgeWithSuchId。
    assert!(matches!(
        edge::service::update(
            "no-such-id",
            "title".to_string(),
            "desc".to_string()
        ),
        Err(ErrorCode::NoEdgeWithSuchId { .. })
    ));

    // edge::update 成功路径：标题和详情被更新。
    edge::service::update(
        &edge_1.id,
        "new title".to_string(),
        "new description".to_string(),
    )
    .unwrap();

    // edge::list 成功路径：返回该画布内的全部边，其它画布的边不受影响。
    let edges = edge::service::list(&child.id).unwrap();
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].id, edge_1.id);
    assert!(edge::service::list(&root.id).unwrap().is_empty());

    // edge::create 失败路径：同一对节点之间重复建边（即使连接桩不同）报 EdgeAlreadyExists。
    assert!(matches!(
        edge::service::create(
            &child.id,
            &node_1.id,
            "top".to_string(),
            &node_2.id,
            "bottom".to_string()
        ),
        Err(ErrorCode::EdgeAlreadyExists)
    ));

    // edge::create 失败路径：反向建边会成环报 EdgeWouldFormCycle；自环同样报 EdgeWouldFormCycle。
    assert!(matches!(
        edge::service::create(
            &child.id,
            &node_2.id,
            "right".to_string(),
            &node_1.id,
            "left".to_string()
        ),
        Err(ErrorCode::EdgeWouldFormCycle)
    ));
    assert!(matches!(
        edge::service::create(
            &child.id,
            &node_1.id,
            "right".to_string(),
            &node_1.id,
            "left".to_string()
        ),
        Err(ErrorCode::EdgeWouldFormCycle)
    ));

    // node::move_node 失败路径：节点不存在时报 NoNodeWithSuchId。
    assert!(matches!(
        node::service::move_node("no-such-id", 0.0, 0.0),
        Err(ErrorCode::NoNodeWithSuchId { .. })
    ));

    // node::move_node 成功路径：坐标被更新。
    node::service::move_node(&node_1.id, 30.0, 40.0).unwrap();

    // node::modify 失败路径：节点不存在时报 NoNodeWithSuchId。
    assert!(matches!(
        node::service::modify("no-such-id", "t".to_string(), "s".to_string()),
        Err(ErrorCode::NoNodeWithSuchId { .. })
    ));

    // node::modify 成功路径：标题和副标题被更新，坐标保持 move 后的值。
    node::service::modify(
        &node_1.id,
        "title-1-new".to_string(),
        "sub-1-new".to_string(),
    )
    .unwrap();
    let modified = node::service::list(&child.id, false)
        .unwrap()
        .into_iter()
        .find(|node| node.id == node_1.id)
        .unwrap();
    assert_eq!(modified.title, "title-1-new");
    assert_eq!(modified.sub_title, "sub-1-new");
    assert_eq!((modified.x, modified.y), (30.0, 40.0));
    // node::move_node 成功路径（原地移动）：新坐标与旧坐标相同时成功返回，
    // 且不产生新日志（日志总数不变）。
    let log_total_before_node = log::service::list(0, 1).unwrap().total;
    node::service::move_node(&node_1.id, 30.0, 40.0).unwrap();
    assert_eq!(log::service::list(0, 1).unwrap().total, log_total_before_node);

    // node::logical_delete 失败路径：节点不存在时报 NoNodeWithSuchId。
    assert!(matches!(
        node::service::logical_delete("no-such-id"),
        Err(ErrorCode::NoNodeWithSuchId { .. })
    ));

    // node::logical_delete 成功路径：节点从正常列表移动到已删除列表（list 按 deleted 分流）。
    // 同时校验返回值：返回的 Node 对象 id 匹配、deleted == true、字段与原节点一致。
    let deleted_node_1 = node::service::logical_delete(&node_1.id).unwrap();
    assert_eq!(deleted_node_1.id, node_1.id);
    assert!(deleted_node_1.deleted);
    assert_eq!(deleted_node_1.title, "title-1-new");
    assert_eq!(deleted_node_1.sub_title, "sub-1-new");
    assert_eq!(deleted_node_1.x, 30.0);
    assert_eq!(deleted_node_1.y, 40.0);
    assert_eq!(deleted_node_1.canvas_id, child.id);
    assert_eq!(deleted_node_1.canvas_ref_id, None);
    assert!(node::service::list(&child.id, false)
        .unwrap()
        .iter()
        .all(|node| node.id != node_1.id));
    assert!(node::service::list(&child.id, true)
        .unwrap()
        .iter()
        .any(|node| node.id == node_1.id));

    // node::restore 失败路径：节点不存在时报 NoNodeWithSuchId。
    assert!(matches!(
        node::service::restore("no-such-id", 0.0, 0.0),
        Err(ErrorCode::NoNodeWithSuchId { .. })
    ));

    // node::restore 成功路径：逻辑删除标志被清空且坐标更新为新坐标。
    node::service::restore(&node_1.id, 50.0, 60.0).unwrap();
    let restored_node = node::service::list(&child.id, false)
        .unwrap()
        .into_iter()
        .find(|node| node.id == node_1.id)
        .unwrap();
    assert_eq!((restored_node.x, restored_node.y), (50.0, 60.0));

    // node::physical_delete 失败路径：节点不存在时报 NoNodeWithSuchId。
    assert!(matches!(
        node::service::physical_delete("no-such-id"),
        Err(ErrorCode::NoNodeWithSuchId { .. })
    ));

    // node::physical_delete 成功路径：节点被物理删除，与它相连的边被一并删除
    // （再删除该边时报 NoEdgeWithSuchId，证明边已不存在）。
    node::service::physical_delete(&node_2.id).unwrap();
    assert!(node::service::list(&child.id, true)
        .unwrap()
        .iter()
        .all(|node| node.id != node_2.id));
    assert!(matches!(
        edge::service::delete(&edge_1.id),
        Err(ErrorCode::NoEdgeWithSuchId { .. })
    ));

    // edge::delete 成功路径：先建一条边（node_1 -> node_3）再删除它。
    let node_3 = node::service::create(
        &child.id,
        "title-3".to_string(),
        "sub-3".to_string(),
        200.0,
        300.0,
        None,
        false,
    )
    .unwrap();
    let edge_2 = edge::service::create(
        &child.id,
        &node_1.id,
        "bottom".to_string(),
        &node_3.id,
        "top".to_string(),
    )
    .unwrap();
    edge::service::delete(&edge_2.id).unwrap();

    // edge::delete 失败路径：重复删除同一条边报 NoEdgeWithSuchId。
    assert!(matches!(
        edge::service::delete(&edge_2.id),
        Err(ErrorCode::NoEdgeWithSuchId { .. })
    ));

    // canvas::physical_delete 失败路径：画布不存在时报 NoCanvasWithSuchId。
    assert!(matches!(
        canvas::service::physical_delete("no-such-id"),
        Err(ErrorCode::NoCanvasWithSuchId { .. })
    ));

    // canvas::physical_delete 失败路径：目标是根画布时报 RootCanvasCannotBeDeleted。
    assert!(matches!(
        canvas::service::physical_delete(&root.id),
        Err(ErrorCode::RootCanvasCannotBeDeleted)
    ));

    // canvas::physical_delete 成功路径：子树内的画布、节点、边全部被物理删除，根画布不受影响。
    canvas::service::physical_delete(&child.id).unwrap();
    let canvases = canvas::service::list(false).unwrap();
    assert_eq!(canvases.len(), 1);
    assert_eq!(canvases[0].id, root.id);
    assert!(canvas::service::list(true).unwrap().is_empty());
    assert!(node::service::list(&child.id, false).unwrap().is_empty());
    assert!(node::service::list(&child.id, true).unwrap().is_empty());

    // canvas::physical_delete 成功路径：被删除画布的视口一并删除（get 返回默认值 (0, 0, 1)），
    // 画布宇宙视口不属于任何画布，不受影响。
    let child_viewport = viewport::service::get(Some(child.id.clone())).unwrap();
    assert_eq!(
        (child_viewport.x, child_viewport.y, child_viewport.zoom),
        (0.0, 0.0, 1.0)
    );
    assert_eq!(viewport::service::get(None).unwrap().zoom, 3.0);

    // log::list 成功路径：以上操作产生了对应行为的日志，且载荷已正确解密重组。
    let logs = log::service::list(0, 1000).unwrap();
    // log::list 成功路径：limit 足够大时 total 与 items 长度一致。
    assert_eq!(logs.total, logs.items.len() as i64);
    let has = |object_id: &str, predicate: fn(&entity::Action) -> bool| {
        logs.items
            .iter()
            .any(|entry| entry.object_id == object_id && predicate(&entry.action))
    };
    // 子画布经历过创建、移动、逻辑删除、恢复、物理删除，五种行为的日志都应存在。
    assert!(has(
        &child.id,
        |action| matches!(action, entity::Action::CanvasCreate { name } if name == "child")
    ));
    assert!(has(
        &child.id,
        |action| matches!(action, entity::Action::CanvasMove { name, .. } if name == "child")
    ));
    assert!(has(
        &child.id,
        |action| matches!(action, entity::Action::CanvasLogicalDelete { name } if name == "child")
    ));
    assert!(has(
        &child.id,
        |action| matches!(action, entity::Action::CanvasRestore { name, .. } if name == "child")
    ));
    assert!(has(
        &child.id,
        |action| matches!(action, entity::Action::CanvasPhysicalDelete { name } if name == "child")
    ));
    // 节点和边的日志同样按对象 id 记录。
    assert!(has(
        &node_1.id,
        |action| matches!(action, entity::Action::NodeCreate { title, .. } if title == "title-1")
    ));
    assert!(has(
        &node_1.id,
        |action| matches!(action, entity::Action::NodeLogicalDelete { title } if title == "title-1-new")
    ));
    assert!(has(
        &node_1.id,
        |action| matches!(action, entity::Action::NodeRestore { title, .. } if title == "title-1-new")
    ));
    assert!(has(
        &edge_1.id,
        |action| matches!(action, entity::Action::EdgeCreate { source_title, target_title } if source_title == "title-1" && target_title == "title-2")
    ));
    assert!(has(
        &edge_2.id,
        |action| matches!(action, entity::Action::EdgePhysicalDelete { source_title, target_title } if source_title == "title-1-new" && target_title == "title-3")
    ));

    // log::list 成功路径：分页参数正确生效，且整体按时间倒序排列。
    let total = logs.total;
    assert!(total > 2);
    let first_page = log::service::list(0, 2).unwrap();
    assert_eq!(first_page.items.len(), 2);
    assert_eq!(first_page.items[0].id, logs.items[0].id);
    let rest = log::service::list(2, 1000).unwrap();
    assert_eq!(rest.items.len() as i64, total - 2);
    // log::list 成功路径：分页查询时 total 始终为总条数，不受分页参数影响。
    assert_eq!(first_page.total, total);
    assert_eq!(rest.total, total);
    for window in logs.items.windows(2) {
        assert!(window[0].time >= window[1].time);
    }

    // log::create 成功路径：直接插入一条日志后能在列表中查到，载荷解密并重组反序列化正确。
    let before = logs.total;
    log::service::create(
        "object-x",
        entity::Action::CanvasCreate {
            name: "手动日志".to_string(),
        },
    )
    .unwrap();
    let after = log::service::list(0, 1000).unwrap();
    assert_eq!(after.total, before + 1);
    assert!(after.items.iter().any(|entry| entry.object_id == "object-x"
        && matches!(
            entry.action,
            entity::Action::CanvasCreate { ref name } if name == "手动日志"
        )));

    // lifecycle::save / close 成功路径：保存并关闭后 state 被清空。
    lifecycle::service::save().unwrap();
    lifecycle::service::close().unwrap();
    assert!(!state::is_open());

    // lifecycle::initialize 成功路径（打开已存在的数据库）：保存过的数据仍在。
    lifecycle::service::initialize(&id, test::test_key()).unwrap();
    assert!(state::is_open());
    let canvases = canvas::service::list(false).unwrap();
    assert_eq!(canvases.len(), 1);
    assert_eq!(canvases[0].name, entity::ROOT_CANVAS_NAME);
    assert!(!log::service::list(0, 1000).unwrap().items.is_empty());

    // lifecycle::initialize 失败路径：错误密钥报 FailToDecrypt。
    lifecycle::service::close().unwrap();
    assert!(matches!(
        lifecycle::service::initialize(&id, [2u8; 32]),
        Err(ErrorCode::FailToDecrypt { .. })
    ));
    assert!(!state::is_open());

    // lifecycle::save / close 失败路径：数据库未打开时报 UserDatabaseNotOpen。
    assert!(matches!(
        lifecycle::service::save(),
        Err(ErrorCode::UserDatabaseNotOpen)
    ));
    assert!(matches!(
        lifecycle::service::close(),
        Err(ErrorCode::UserDatabaseNotOpen)
    ));
    }

    // == 阶段二：node_field 与 dictionary（重新打开同一数据库）==
    {
    let id = registered.id.clone();
    lifecycle::service::initialize(&id, test::test_key()).unwrap();

    // 在根画布下创建 test 节点。
    let canvases = canvas::service::list(false).unwrap();
    let root_id = &canvases[0].id;
    let node = node::service::create(
        root_id,
        "test-node".to_string(),
        "test-sub".to_string(),
        100.0,
        200.0,
        None,
        false,
    )
    .unwrap();

    // == node_field::get 失败路径：节点不存在 → NoNodeWithSuchId ==
    assert!(matches!(
        node_field::service::get(&uuid::Uuid::new_v4().to_string()),
        Err(ErrorCode::NoNodeWithSuchId { .. })
    ));

    // == 准备字典数据（供后续 dictionary_id 相关测试使用）==
    let dict_a = Dictionary {
        id: uuid::Uuid::new_v4().to_string(),
        parent_id: None,
        value: "条目A".to_string(),
        order: 1,
    };
    let dict_b = Dictionary {
        id: uuid::Uuid::new_v4().to_string(),
        parent_id: None,
        value: "条目B".to_string(),
        order: 2,
    };
    dictionary::service::set(&[dict_a.clone(), dict_b.clone()]).unwrap();

    // == node_field::set 失败路径：字段重名 → DuplicateNodeFieldName ==
    let dup_fields = vec![
        NodeFieldVO {
            name: "dup".to_string(),
            field_type: "TextSingleLine".to_string(),
            type_config: None,
            value: FieldValue::String(Some("a".to_string())),
            dictionary_id: None,
        },
        NodeFieldVO {
            name: "dup".to_string(),
            field_type: "TextSingleLine".to_string(),
            type_config: None,
            value: FieldValue::String(Some("b".to_string())),
            dictionary_id: None,
        },
    ];
    assert!(matches!(
        node_field::service::set(&node.id, &dup_fields),
        Err(ErrorCode::DuplicateNodeFieldName { .. })
    ));

    // == node_field::set 失败路径：非法 field_type → InvalidNodeFieldType ==
    assert!(matches!(
        node_field::service::set(
            &node.id,
            &[NodeFieldVO {
                name: "bad-type".to_string(),
                field_type: "NoSuchType".to_string(),
                type_config: None,
                value: FieldValue::String(Some("x".to_string())),
                dictionary_id: None,
            }]
        ),
        Err(ErrorCode::InvalidNodeFieldType { .. })
    ));

    // == node_field::set 失败路径：kind 不匹配（String 值配 Date 类型）→ NodeFieldValueKindMismatch ==
    assert!(matches!(
        node_field::service::set(
            &node.id,
            &[NodeFieldVO {
                name: "kind-mismatch".to_string(),
                field_type: "Date".to_string(),
                type_config: None,
                value: FieldValue::String(Some("2024-01-01".to_string())),
                dictionary_id: None,
            }]
        ),
        Err(ErrorCode::NodeFieldValueKindMismatch { .. })
    ));

    // == node_field::set 失败路径：值非法（"abc" 配 Number 类型）→ NodeFieldValueValidationFailed ==
    assert!(matches!(
        node_field::service::set(
            &node.id,
            &[NodeFieldVO {
                name: "bad-value".to_string(),
                field_type: "Number".to_string(),
                type_config: None,
                value: FieldValue::Decimal(Some("abc".to_string())),
                dictionary_id: None,
            }]
        ),
        Err(ErrorCode::NodeFieldValueValidationFailed { .. })
    ));

    // == node_field::set 失败路径：type_config 非法（Date 配 {"precision":"week"}）→ InvalidNodeFieldTypeConfig ==
    assert!(matches!(
        node_field::service::set(
            &node.id,
            &[NodeFieldVO {
                name: "bad-config".to_string(),
                field_type: "Date".to_string(),
                type_config: Some(serde_json::json!({"precision": "week"})),
                value: FieldValue::Instant(Some(1000)),
                dictionary_id: None,
            }]
        ),
        Err(ErrorCode::InvalidNodeFieldTypeConfig { .. })
    ));

    // == node_field::set 失败路径：dictionary_id 不存在 → NoDictionaryEntryWithSuchId ==
    assert!(matches!(
        node_field::service::set(
            &node.id,
            &[NodeFieldVO {
                name: "dict-missing".to_string(),
                field_type: "TextSingleLine".to_string(),
                type_config: None,
                value: FieldValue::String(Some("x".to_string())),
                dictionary_id: Some(uuid::Uuid::new_v4().to_string()),
            }]
        ),
        Err(ErrorCode::NoDictionaryEntryWithSuchId { .. })
    ));

    // == node_field::set 失败路径：Password 不支持字典 → FieldTypeNotSupportDictionary ==
    assert!(matches!(
        node_field::service::set(
            &node.id,
            &[NodeFieldVO {
                name: "password-dict".to_string(),
                field_type: "Password".to_string(),
                type_config: None,
                value: FieldValue::String(Some("pw".to_string())),
                dictionary_id: Some(dict_a.id.clone()),
            }]
        ),
        Err(ErrorCode::FieldTypeNotSupportDictionary { .. })
    ));

    // == node_field::set 成功路径：覆盖 string/decimal/instant/instantRange 四种值、
    //    带 type_config 的 Date、带 dictionary_id 的 TextSingleLine ==
    let now = 1712345678000i64;
    let fields = vec![
        NodeFieldVO {
            name: "文本".to_string(),
            field_type: "TextSingleLine".to_string(),
            type_config: None,
            value: FieldValue::String(Some("hello".to_string())),
            dictionary_id: None,
        },
        NodeFieldVO {
            name: "数字".to_string(),
            field_type: "Number".to_string(),
            type_config: None,
            value: FieldValue::Decimal(Some("123.456".to_string())),
            dictionary_id: None,
        },
        NodeFieldVO {
            name: "日期".to_string(),
            field_type: "Date".to_string(),
            type_config: Some(serde_json::json!({"precision": "day"})),
            value: FieldValue::Instant(Some(now)),
            dictionary_id: None,
        },
        NodeFieldVO {
            name: "日期区间".to_string(),
            field_type: "DateRange".to_string(),
            type_config: None,
            value: FieldValue::InstantRange(Some((1000, 2000))),
            dictionary_id: None,
        },
        NodeFieldVO {
            name: "字典引用".to_string(),
            field_type: "TextSingleLine".to_string(),
            type_config: None,
            value: FieldValue::String(Some("x".to_string())),
            dictionary_id: Some(dict_a.id.clone()),
        },
    ];
    node_field::service::set(&node.id, &fields).unwrap();

    // == node_field::get 成功路径：往返一致 ==
    let got = node_field::service::get(&node.id).unwrap();
    assert_eq!(got.len(), 5);
    assert_eq!(got[0], fields[0]);
    assert_eq!(got[1], fields[1]);
    assert_eq!(got[2], fields[2]);
    assert_eq!(got[3], fields[3]);
    assert_eq!(got[4], fields[4]);

    // == set 成功路径：四种类型的无值字段（各变体 None）set/get 往返 ==
    let none_fields = vec![
        NodeFieldVO {
            name: "空文本".to_string(),
            field_type: "TextSingleLine".to_string(),
            type_config: None,
            value: FieldValue::String(None),
            dictionary_id: None,
        },
        NodeFieldVO {
            name: "空数字".to_string(),
            field_type: "Number".to_string(),
            type_config: None,
            value: FieldValue::Decimal(None),
            dictionary_id: None,
        },
        NodeFieldVO {
            name: "空日期".to_string(),
            field_type: "Date".to_string(),
            type_config: None,
            value: FieldValue::Instant(None),
            dictionary_id: None,
        },
        NodeFieldVO {
            name: "空日期区间".to_string(),
            field_type: "DateRange".to_string(),
            type_config: None,
            value: FieldValue::InstantRange(None),
            dictionary_id: None,
        },
    ];
    node_field::service::set(&node.id, &none_fields).unwrap();
    let got_none = node_field::service::get(&node.id).unwrap();
    assert_eq!(got_none.len(), 4);
    for (i, f) in none_fields.iter().enumerate() {
        assert_eq!(got_none[i], *f);
    }

    // == dictionary::set 失败路径：id 重复 → DuplicateDictionaryId ==
    let dup_dict = vec![
        Dictionary {
            id: uuid::Uuid::new_v4().to_string(),
            parent_id: None,
            value: "val".to_string(),
            order: 1,
        },
        Dictionary {
            id: uuid::Uuid::new_v4().to_string(),
            parent_id: None,
            value: "val".to_string(),
            order: 2,
        },
    ];
    let dup_id = dup_dict[0].id.clone();
    let dup_entries = vec![
        dup_dict[0].clone(),
        Dictionary {
            id: dup_id.clone(),
            parent_id: None,
            value: "dup".to_string(),
            order: 3,
        },
    ];
    match dictionary::service::set(&dup_entries) {
        Err(ErrorCode::DuplicateDictionaryId { id }) => assert_eq!(id, dup_id),
        other => panic!("expected DuplicateDictionaryId, got {other:?}"),
    }

    // == dictionary::set 失败路径：parent 不在集合内 → NoDictionaryEntryWithSuchId ==
    let orphan = Dictionary {
        id: uuid::Uuid::new_v4().to_string(),
        parent_id: Some("no-such-parent-id".to_string()),
        value: "orphan".to_string(),
        order: 1,
    };
    assert!(matches!(
        dictionary::service::set(&[orphan]),
        Err(ErrorCode::NoDictionaryEntryWithSuchId { .. })
    ));

    // == dictionary 端到端：node_field 引用条目 A → set 移除 A → node_field::get 返回 dictionary_id 已置空 ==
    // 先设置引用 dict_a 的字段。
    node_field::service::set(
        &node.id,
        &[NodeFieldVO {
            name: "引用A".to_string(),
            field_type: "TextSingleLine".to_string(),
            type_config: None,
            value: FieldValue::String(Some("y".to_string())),
            dictionary_id: Some(dict_a.id.clone()),
        }],
    )
    .unwrap();
    // 重设字典全集（只保留 dict_b，移除 dict_a）。
    dictionary::service::set(&[dict_b.clone()]).unwrap();
    let after_prune = node_field::service::get(&node.id).unwrap();
    let ref_field = after_prune.iter().find(|f| f.name == "引用A").unwrap();
    assert!(ref_field.dictionary_id.is_none());

    // == node_field::set 覆盖写语义：先删后插，get 只剩新集合 ==
    let overwrite = vec![NodeFieldVO {
        name: "覆盖字段".to_string(),
        field_type: "TextSingleLine".to_string(),
        type_config: None,
        value: FieldValue::String(Some("overwritten".to_string())),
        dictionary_id: None,
    }];
    node_field::service::set(&node.id, &overwrite).unwrap();
    let after_overwrite = node_field::service::get(&node.id).unwrap();
    assert_eq!(after_overwrite.len(), 1);
    assert_eq!(after_overwrite[0].name, "覆盖字段");

    // == node_field::set 产生 NodeFieldsModify 日志的端到端验证 ==
    // 用全新的空字段节点进行一系列 set 操作并验证日志。
    let log_node = node::service::create(
        root_id,
        "log-node".to_string(),
        "log-sub".to_string(),
        0.0,
        0.0,
        None,
        false,
    )
    .unwrap();
    let before_total = log::service::list(0, 1000).unwrap().total;

    // 首次给无字段节点 set 两个字段 → 一条 NodeFieldsModify，changes 为两个 Added。
    let f1 = NodeFieldVO {
        name: "f1".to_string(),
        field_type: "TextSingleLine".to_string(),
        type_config: None,
        value: FieldValue::String(Some("hello".to_string())),
        dictionary_id: None,
    };
    let f2 = NodeFieldVO {
        name: "f2".to_string(),
        field_type: "Number".to_string(),
        type_config: None,
        value: FieldValue::Decimal(Some("13.5".to_string())),
        dictionary_id: None,
    };
    node_field::service::set(&log_node.id, &[f1.clone(), f2.clone()]).unwrap();
    let after_set = log::service::list(0, 1000).unwrap();
    assert_eq!(after_set.total, before_total + 1);
    // 取最新一条（时间倒序，第 0 条即最新），验证 NodeFieldsModify。
    let first_entry = after_set
        .items
        .iter()
        .find(|e| {
            e.object_id == log_node.id
                && matches!(e.action, entity::Action::NodeFieldsModify { .. })
        })
        .unwrap();
    let first_changes = match &first_entry.action {
        entity::Action::NodeFieldsModify { node_title, changes } => {
            assert_eq!(*node_title, "log-node");
            changes
        }
        _ => unreachable!(),
    };
    assert_eq!(first_changes.len(), 2);
    assert!(matches!(&first_changes[0],
        entity::NodeFieldChange::Added { name, field_type, value }
        if name == "f1" && field_type == "TextSingleLine" && value == &FieldValue::String(Some("hello".to_string()))));
    assert!(matches!(&first_changes[1],
        entity::NodeFieldChange::Added { name, field_type, value }
        if name == "f2" && field_type == "Number" && value == &FieldValue::Decimal(Some("13.5".to_string()))));

    // 再次 set 完全相同的内容 → 不产生新的 NodeFieldsModify 日志（日志总数不变）。
    node_field::service::set(&log_node.id, &[f1.clone(), f2.clone()]).unwrap();
    let after_same = log::service::list(0, 1000).unwrap();
    assert_eq!(after_same.total, after_set.total);

    // 修改 f1 的值 → 一条 Modified（old_value / new_value 正确）。
    let f1_modified = NodeFieldVO {
        name: "f1".to_string(),
        field_type: "TextSingleLine".to_string(),
        type_config: None,
        value: FieldValue::String(Some("updated".to_string())),
        dictionary_id: None,
    };
    node_field::service::set(&log_node.id, &[f1_modified, f2.clone()]).unwrap();
    let after_value_change = log::service::list(0, 1000).unwrap();
    assert_eq!(after_value_change.total, after_same.total + 1);
    let value_change_entry = after_value_change
        .items
        .iter()
        .find(|e| {
            e.object_id == log_node.id
                && matches!(e.action, entity::Action::NodeFieldsModify { .. })
        })
        .unwrap();
    let value_changes = match &value_change_entry.action {
        entity::Action::NodeFieldsModify { changes, .. } => changes,
        _ => unreachable!(),
    };
    assert_eq!(value_changes.len(), 1);
    assert!(matches!(&value_changes[0],
        entity::NodeFieldChange::Modified { name, old_field_type, new_field_type, old_value, new_value }
        if name == "f1"
        && old_field_type == "TextSingleLine" && new_field_type == "TextSingleLine"
        && old_value == &FieldValue::String(Some("hello".to_string()))
        && new_value == &FieldValue::String(Some("updated".to_string()))));

    // 修改 f1 的类型（TextSingleLine → Number）→ 一条 Modified（old/new field_type 正确）。
    let f1_type_change = NodeFieldVO {
        name: "f1".to_string(),
        field_type: "Number".to_string(),
        type_config: None,
        value: FieldValue::Decimal(Some("42".to_string())),
        dictionary_id: None,
    };
    node_field::service::set(&log_node.id, &[f1_type_change.clone(), f2.clone()]).unwrap();
    let after_type_change = log::service::list(0, 1000).unwrap();
    assert_eq!(after_type_change.total, after_value_change.total + 1);
    let type_change_entry = after_type_change
        .items
        .iter()
        .find(|e| {
            e.object_id == log_node.id
                && matches!(e.action, entity::Action::NodeFieldsModify { .. })
        })
        .unwrap();
    let type_changes = match &type_change_entry.action {
        entity::Action::NodeFieldsModify { changes, .. } => changes,
        _ => unreachable!(),
    };
    assert_eq!(type_changes.len(), 1);
    assert!(matches!(&type_changes[0],
        entity::NodeFieldChange::Modified { name, old_field_type, new_field_type, old_value, new_value }
        if name == "f1"
        && old_field_type == "TextSingleLine" && new_field_type == "Number"
        && old_value == &FieldValue::String(Some("updated".to_string()))
        && new_value == &FieldValue::Decimal(Some("42".to_string()))));

    // 删除 f2 → 一条 Removed。
    node_field::service::set(&log_node.id, &[f1_type_change.clone()]).unwrap();
    let after_remove = log::service::list(0, 1000).unwrap();
    assert_eq!(after_remove.total, after_type_change.total + 1);
    let remove_entry = after_remove
        .items
        .iter()
        .find(|e| {
            e.object_id == log_node.id
                && matches!(e.action, entity::Action::NodeFieldsModify { .. })
        })
        .unwrap();
    let remove_changes = match &remove_entry.action {
        entity::Action::NodeFieldsModify { changes, .. } => changes,
        _ => unreachable!(),
    };
    assert_eq!(remove_changes.len(), 1);
    assert!(matches!(&remove_changes[0],
        entity::NodeFieldChange::Removed { name, field_type, old_value }
        if name == "f2" && field_type == "Number" && old_value == &FieldValue::Decimal(Some("13.5".to_string()))));

    // 字段改名（f1 → f1-renamed）→ 一条 Removed（旧名）+ 一条 Added（新名）。
    let f1_renamed = NodeFieldVO {
        name: "f1-renamed".to_string(),
        field_type: "Number".to_string(),
        type_config: None,
        value: FieldValue::Decimal(Some("42".to_string())),
        dictionary_id: None,
    };
    node_field::service::set(&log_node.id, &[f1_renamed]).unwrap();
    let after_rename = log::service::list(0, 1000).unwrap();
    assert_eq!(after_rename.total, after_remove.total + 1);
    let rename_entry = after_rename
        .items
        .iter()
        .find(|e| {
            e.object_id == log_node.id
                && matches!(e.action, entity::Action::NodeFieldsModify { .. })
        })
        .unwrap();
    let rename_changes = match &rename_entry.action {
        entity::Action::NodeFieldsModify { changes, .. } => changes,
        _ => unreachable!(),
    };
    assert_eq!(rename_changes.len(), 2);
    assert!(matches!(&rename_changes[0],
        entity::NodeFieldChange::Removed { name, field_type, old_value }
        if name == "f1" && field_type == "Number" && old_value == &FieldValue::Decimal(Some("42".to_string()))));
    assert!(matches!(&rename_changes[1],
        entity::NodeFieldChange::Added { name, field_type, value }
        if name == "f1-renamed" && field_type == "Number" && value == &FieldValue::Decimal(Some("42".to_string()))));

    // == 清理 ==
    lifecycle::service::save().unwrap();
    lifecycle::service::close().unwrap();
    }

    // == 阶段三：template（再次重新打开同一数据库；含 node::create 带模板、
    //    node::physical_delete 级联 node_field、dictionary::set 清除 template_field 引用的端到端行为）==
    {
    let id = registered.id.clone();
    lifecycle::service::initialize(&id, test::test_key()).unwrap();

    // == template::create 成功路径 ==
    let tpl_a = template::service::create("模板A".to_string()).unwrap();
    assert_eq!(tpl_a.name, "模板A");
    assert_eq!(tpl_a.order, 0);

    // == template::create 失败路径：重名 → TemplateNameAlreadyExists ==
    assert!(matches!(
        template::service::create("模板A".to_string()),
        Err(ErrorCode::TemplateNameAlreadyExists { .. })
    ));

    // == template::create 成功路径：order 递增 ==
    let tpl_b = template::service::create("模板B".to_string()).unwrap();
    assert_eq!(tpl_b.order, 1);

    // == template::list 返回按 order 升序 ==
    let all = template::service::list().unwrap();
    assert_eq!(all.len(), 2);
    assert_eq!(all[0].id, tpl_a.id);
    assert_eq!(all[1].id, tpl_b.id);

    // == create_from_node 失败路径：节点不存在 → NoNodeWithSuchId ==
    assert!(matches!(
        template::service::create_from_node(
            &uuid::Uuid::new_v4().to_string(),
            "from-node".to_string()
        ),
        Err(ErrorCode::NoNodeWithSuchId { .. })
    ));

    // == create_from_node 失败路径：模板名与现有重复 → TemplateNameAlreadyExists ==
    let canvases = canvas::service::list(false).unwrap();
    let root_id = &canvases[0].id;
    let src_node = node::service::create(
        root_id,
        "src".to_string(),
        "sub".to_string(),
        0.0,
        0.0,
        None,
        false,
    ).unwrap();
    assert!(matches!(
        template::service::create_from_node(&src_node.id, "模板A".to_string()),
        Err(ErrorCode::TemplateNameAlreadyExists { .. })
    ));

    // == create_from_node 成功路径：复制节点的全部字段结构 ==
    let dict_d = Dictionary {
        id: uuid::Uuid::new_v4().to_string(),
        parent_id: None,
        value: "字典D".to_string(),
        order: 1,
    };
    dictionary::service::set(&[dict_d.clone()]).unwrap();
    let node_fields = vec![
        NodeFieldVO {
            name: "文本字段".to_string(),
            field_type: "TextSingleLine".to_string(),
            type_config: None,
            value: FieldValue::String(Some("hello".to_string())),
            dictionary_id: None,
        },
        NodeFieldVO {
            name: "日期字段".to_string(),
            field_type: "Date".to_string(),
            type_config: Some(serde_json::json!({"precision": "day"})),
            value: FieldValue::Instant(Some(1712345678000)),
            dictionary_id: None,
        },
        NodeFieldVO {
            name: "字典字段".to_string(),
            field_type: "TextSingleLine".to_string(),
            type_config: None,
            value: FieldValue::String(Some("v".to_string())),
            dictionary_id: Some(dict_d.id.clone()),
        },
    ];
    node_field::service::set(&src_node.id, &node_fields).unwrap();
    let tpl_c = template::service::create_from_node(&src_node.id, "模板C".to_string()).unwrap();
    let tpl_c_fields = template::service::get_fields(&tpl_c.id).unwrap();
    assert_eq!(tpl_c_fields.len(), 3);
    // 字段名、类型、配置、字典引用与源节点一致
    assert_eq!(tpl_c_fields[0].name, "文本字段");
    assert_eq!(tpl_c_fields[0].field_type, "TextSingleLine");
    assert!(tpl_c_fields[0].type_config.is_none());
    assert!(tpl_c_fields[0].dictionary_id.is_none());
    assert_eq!(tpl_c_fields[1].name, "日期字段");
    assert_eq!(tpl_c_fields[1].field_type, "Date");
    assert_eq!(tpl_c_fields[1].type_config, Some(serde_json::json!({"precision": "day"})));
    assert!(tpl_c_fields[1].dictionary_id.is_none());
    assert_eq!(tpl_c_fields[2].name, "字典字段");
    assert_eq!(tpl_c_fields[2].field_type, "TextSingleLine");
    assert_eq!(tpl_c_fields[2].dictionary_id.as_deref(), Some(dict_d.id.as_str()));

    // == rename 失败路径：模板不存在 → NoTemplateWithSuchId ==
    assert!(matches!(
        template::service::rename(&uuid::Uuid::new_v4().to_string(), "x".to_string()),
        Err(ErrorCode::NoTemplateWithSuchId { .. })
    ));

    // == rename 失败路径：新名与其它模板重复 → TemplateNameAlreadyExists ==
    assert!(matches!(
        template::service::rename(&tpl_a.id, "模板B".to_string()),
        Err(ErrorCode::TemplateNameAlreadyExists { .. })
    ));

    // == rename 成功路径 ==
    template::service::rename(&tpl_a.id, "模板A-新".to_string()).unwrap();
    let renamed = template::service::list()
        .unwrap()
        .into_iter()
        .find(|t| t.id == tpl_a.id)
        .unwrap();
    assert_eq!(renamed.name, "模板A-新");

    // == rename 成功路径：新旧同名不产生日志（直接返回 Ok） ==
    template::service::rename(&tpl_a.id, "模板A-新".to_string()).unwrap();

    // == set_fields 失败路径：模板不存在 → NoTemplateWithSuchId ==
    assert!(matches!(
        template::service::set_fields(
            &uuid::Uuid::new_v4().to_string(),
            &[TemplateFieldVO {
                name: "f".to_string(),
                field_type: "TextSingleLine".to_string(),
                type_config: None,
                dictionary_id: None,
            }]
        ),
        Err(ErrorCode::NoTemplateWithSuchId { .. })
    ));

    // == set_fields 失败路径：字段重名 → DuplicateNodeFieldName ==
    assert!(matches!(
        template::service::set_fields(
            &tpl_a.id,
            &[
                TemplateFieldVO {
                    name: "dup".to_string(),
                    field_type: "TextSingleLine".to_string(),
                    type_config: None,
                    dictionary_id: None,
                },
                TemplateFieldVO {
                    name: "dup".to_string(),
                    field_type: "TextSingleLine".to_string(),
                    type_config: None,
                    dictionary_id: None,
                },
            ]
        ),
        Err(ErrorCode::DuplicateNodeFieldName { .. })
    ));

    // == set_fields 失败路径：非法 field_type → InvalidNodeFieldType ==
    assert!(matches!(
        template::service::set_fields(
            &tpl_a.id,
            &[TemplateFieldVO {
                name: "bad-type".to_string(),
                field_type: "NoSuchType".to_string(),
                type_config: None,
                dictionary_id: None,
            }]
        ),
        Err(ErrorCode::InvalidNodeFieldType { .. })
    ));

    // == set_fields 失败路径：Date 配非法 precision → InvalidNodeFieldTypeConfig ==
    assert!(matches!(
        template::service::set_fields(
            &tpl_a.id,
            &[TemplateFieldVO {
                name: "bad-config".to_string(),
                field_type: "Date".to_string(),
                type_config: Some(serde_json::json!({"precision": "week"})),
                dictionary_id: None,
            }]
        ),
        Err(ErrorCode::InvalidNodeFieldTypeConfig { .. })
    ));

    // == set_fields 失败路径：Password 配字典 → FieldTypeNotSupportDictionary ==
    assert!(matches!(
        template::service::set_fields(
            &tpl_a.id,
            &[TemplateFieldVO {
                name: "pw-dict".to_string(),
                field_type: "Password".to_string(),
                type_config: None,
                dictionary_id: Some(dict_d.id.clone()),
            }]
        ),
        Err(ErrorCode::FieldTypeNotSupportDictionary { .. })
    ));

    // == set_fields 失败路径：字典引用不存在 → NoDictionaryEntryWithSuchId ==
    assert!(matches!(
        template::service::set_fields(
            &tpl_a.id,
            &[TemplateFieldVO {
                name: "missing-dict".to_string(),
                field_type: "TextSingleLine".to_string(),
                type_config: None,
                dictionary_id: Some(uuid::Uuid::new_v4().to_string()),
            }]
        ),
        Err(ErrorCode::NoDictionaryEntryWithSuchId { .. })
    ));

    // == set_fields / get_fields 成功往返 ==
    let tpl_fields = vec![
        TemplateFieldVO {
            name: "模板字段1".to_string(),
            field_type: "TextSingleLine".to_string(),
            type_config: None,
            dictionary_id: None,
        },
        TemplateFieldVO {
            name: "模板字段2".to_string(),
            field_type: "Date".to_string(),
            type_config: Some(serde_json::json!({"precision": "month"})),
            dictionary_id: None,
        },
        TemplateFieldVO {
            name: "模板字段3".to_string(),
            field_type: "TextSingleLine".to_string(),
            type_config: None,
            dictionary_id: Some(dict_d.id.clone()),
        },
    ];
    template::service::set_fields(&tpl_a.id, &tpl_fields).unwrap();
    let got = template::service::get_fields(&tpl_a.id).unwrap();
    assert_eq!(got.len(), 3);
    assert_eq!(got[0], tpl_fields[0]);
    assert_eq!(got[1], tpl_fields[1]);
    assert_eq!(got[2], tpl_fields[2]);

    // == node::service::create 带模板：tid 不存在 → NoTemplateWithSuchId ==
    assert!(matches!(
        node::service::create(
            root_id,
            "n".to_string(),
            "s".to_string(),
            0.0,
            0.0,
            Some(uuid::Uuid::new_v4().to_string()),
            false,
        ),
        Err(ErrorCode::NoTemplateWithSuchId { .. })
    ));

    // == node::service::create 带模板成功：节点字段结构从模板复制，值为 None ==
    let tpl_node = node::service::create(
        root_id,
        "模板节点".to_string(),
        "副标题".to_string(),
        50.0,
        60.0,
        Some(tpl_a.id.clone()),
        false,
    ).unwrap();
    let tpl_node_fields = node_field::service::get(&tpl_node.id).unwrap();
    assert_eq!(tpl_node_fields.len(), 3);
    assert_eq!(tpl_node_fields[0].name, "模板字段1");
    assert_eq!(tpl_node_fields[0].field_type, "TextSingleLine");
    assert!(matches!(tpl_node_fields[0].value, FieldValue::String(None)));
    assert_eq!(tpl_node_fields[1].name, "模板字段2");
    assert_eq!(tpl_node_fields[1].field_type, "Date");
    assert_eq!(tpl_node_fields[1].type_config, Some(serde_json::json!({"precision": "month"})));
    assert!(matches!(tpl_node_fields[1].value, FieldValue::Instant(None)));
    assert_eq!(tpl_node_fields[2].name, "模板字段3");
    assert_eq!(tpl_node_fields[2].dictionary_id.as_deref(), Some(dict_d.id.as_str()));

    // == node::service::create 画布节点带模板：sub_title 原样保留，模板字段结构同样复制 ==
    let tpl_canvas_node = node::service::create(
        root_id,
        "模板画布节点".to_string(),
        "自定义副标题".to_string(),
        0.0,
        0.0,
        Some(tpl_a.id.clone()),
        true,
    ).unwrap();
    assert!(tpl_canvas_node.canvas_ref_id.is_some());
    assert_eq!(tpl_canvas_node.sub_title, "自定义副标题");
    let tpl_canvas_node_fields = node_field::service::get(&tpl_canvas_node.id).unwrap();
    assert_eq!(tpl_canvas_node_fields.len(), 3);

    // == delete 成功路径：删除后 select_by_id 为 None，get_fields 报 NoTemplateWithSuchId ==
    let tpl_b_id = tpl_b.id.clone();
    template::service::delete(&tpl_b_id).unwrap();
    {
        let conn = state::lock_connection();
        assert!(template::dao::select_by_id(&conn, &tpl_b_id).unwrap().is_none());
    }
    assert!(matches!(
        template::service::get_fields(&tpl_b_id),
        Err(ErrorCode::NoTemplateWithSuchId { .. })
    ));

    // == delete 失败路径：模板不存在 → NoTemplateWithSuchId ==
    assert!(matches!(
        template::service::delete(&uuid::Uuid::new_v4().to_string()),
        Err(ErrorCode::NoTemplateWithSuchId { .. })
    ));

    // == node::physical_delete 级联删除 node_field ==
    let cascade_node = node::service::create(
        root_id,
        "cascade".to_string(),
        "s".to_string(),
        0.0,
        0.0,
        None,
        false,
    ).unwrap();
    node_field::service::set(
        &cascade_node.id,
        &[NodeFieldVO {
            name: "cf".to_string(),
            field_type: "TextSingleLine".to_string(),
            type_config: None,
            value: FieldValue::String(Some("v".to_string())),
            dictionary_id: None,
        }],
    ).unwrap();
    let cascade_id = cascade_node.id.clone();
    node::service::physical_delete(&cascade_id).unwrap();
    {
        let conn = state::lock_connection();
        assert!(node_field::dao::select_by_node_id(&conn, &cascade_id).unwrap().is_empty());
    }

    // == dictionary::set 移除条目后 template_field 引用置空 ==
    // 先 set_fields 带字典引用，再 dictionary::set 移除该条目，get_fields 返回的 dictionary_id 为 None
    let dict_e = Dictionary {
        id: uuid::Uuid::new_v4().to_string(),
        parent_id: None,
        value: "字典E".to_string(),
        order: 1,
    };
    dictionary::service::set(&[dict_e.clone()]).unwrap();
    template::service::set_fields(
        &tpl_a.id,
        &[TemplateFieldVO {
            name: "引用E".to_string(),
            field_type: "TextSingleLine".to_string(),
            type_config: None,
            dictionary_id: Some(dict_e.id.clone()),
        }],
    ).unwrap();
    // 重设字典全集为空（移除所有条目）
    dictionary::service::set(&[]).unwrap();
    let after_clear = template::service::get_fields(&tpl_a.id).unwrap();
    let cleared_field = after_clear.iter().find(|f| f.name == "引用E").unwrap();
    assert!(cleared_field.dictionary_id.is_none());

    // == 清理 ==
    lifecycle::service::save().unwrap();
    lifecycle::service::close().unwrap();
    }

    // == 阶段四：attachment（再次重新打开同一数据库；含级联删除、孤儿文件机制与 node_field 修复验证）==
    {
    let id = registered.id.clone();
    lifecycle::service::initialize(&id, test::test_key()).unwrap();

    let canvases = canvas::service::list(false).unwrap();
    let root_id = canvases[0].id.clone();
    let node = node::service::create(
        &root_id,
        "attach-node".to_string(),
        "sub".to_string(),
        0.0,
        0.0,
        None,
        false,
    )
    .unwrap();

    // 在测试数据目录下造已知字节内容的源文件。
    let source_dir = path.data_directory.join("attachment-test-source");
    file_system_util::create_dir_all(&source_dir).unwrap();
    let source_file = source_dir.join("report.pdf");
    let source_bytes: Vec<u8> = (0..4096u32).map(|i| (i % 251) as u8).collect();
    file_system_util::write(&source_file, &source_bytes).unwrap();
    let source_path = source_file.to_string_lossy().to_string();

    // == attachment::import 失败路径：节点不存在 → NoNodeWithSuchId ==
    assert!(matches!(
        attachment::service::import(&uuid::Uuid::new_v4().to_string(), &source_path),
        Err(ErrorCode::NoNodeWithSuchId { .. })
    ));

    // == attachment::import 失败路径：源路径取不到文件名（以 .. 结尾）→ EmptyFilePath ==
    assert!(matches!(
        attachment::service::import(&node.id, ".."),
        Err(ErrorCode::EmptyFilePath)
    ));

    // == attachment::import 失败路径：源文件不存在 → FailToReadFile ==
    let missing_source = source_dir.join("missing.pdf").to_string_lossy().to_string();
    assert!(matches!(
        attachment::service::import(&node.id, &missing_source),
        Err(ErrorCode::FailToReadFile { .. })
    ));

    // == attachment::import 成功路径：VO 字段正确、密文落盘且与源字节不同、解密后与源字节一致 ==
    let imported = attachment::service::import(&node.id, &source_path).unwrap();
    assert_eq!(imported.file_name, "report.pdf");
    assert_eq!(imported.size, source_bytes.len() as i64);
    assert!(imported.create_time > 0);
    assert!(!imported.missing_file);
    let attachment_file = path.user_attachment_file(&id, &imported.id);
    assert!(file_system_util::try_exists(&attachment_file).unwrap());
    let ciphertext = file_system_util::read(&attachment_file).unwrap();
    assert_ne!(ciphertext, source_bytes);
    let decrypted = crate::security::aes::decrypt(ciphertext, test::test_key()).unwrap();
    assert_eq!(decrypted, source_bytes);

    // == attachment::import 失败路径：明文超过大小上限 → AttachmentTooLarge ==
    let big_source = source_dir.join("big.bin");
    let big_bytes = vec![
        0u8;
        (attachment::service::MAX_ATTACHMENT_SIZE_MB + 1) as usize * 1024 * 1024
    ];
    file_system_util::write(&big_source, &big_bytes).unwrap();
    assert!(matches!(
        attachment::service::import(&node.id, &big_source.to_string_lossy()),
        Err(ErrorCode::AttachmentTooLarge { .. })
    ));

    // == attachment::list 成功路径：多附件按 create_time 升序，deleted 过滤正确 ==
    // 休眠 2 毫秒确保第二个附件的 create_time 严格大于第一个。
    std::thread::sleep(std::time::Duration::from_millis(2));
    let second = attachment::service::import(&node.id, &source_path).unwrap();
    let normal = attachment::service::list(&node.id, false).unwrap();
    assert_eq!(normal.len(), 2);
    assert_eq!(normal[0].id, imported.id);
    assert_eq!(normal[1].id, second.id);
    assert!(normal[0].create_time < normal[1].create_time);
    assert!(attachment::service::list(&node.id, true).unwrap().is_empty());

    // == attachment::list 成功路径：手动删除附件文件后 missing_file 标记为 true，其它附件不受影响 ==
    let second_file = path.user_attachment_file(&id, &second.id);
    file_system_util::remove_file(&second_file).unwrap();
    let normal = attachment::service::list(&node.id, false).unwrap();
    assert!(normal.iter().find(|a| a.id == second.id).unwrap().missing_file);
    assert!(!normal.iter().find(|a| a.id == imported.id).unwrap().missing_file);

    // == attachment::list 失败路径：节点不存在 → NoNodeWithSuchId ==
    assert!(matches!(
        attachment::service::list(&uuid::Uuid::new_v4().to_string(), false),
        Err(ErrorCode::NoNodeWithSuchId { .. })
    ));

    // == attachment::load 成功路径：解密后的明文与源字节一致 ==
    assert_eq!(attachment::service::load(&imported.id).unwrap(), source_bytes);

    // == 压缩层：文本附件压缩路径 - 高压缩率的文本内容应被压缩，load 端到端还原 ==
    let notes_path = source_dir.join("notes.txt");
    let notes_bytes = b"hello attachment compression layer. ".repeat(100).to_vec();
    file_system_util::write(&notes_path, &notes_bytes).unwrap();
    let notes_imported = attachment::service::import(&node.id, &notes_path.to_string_lossy()).unwrap();
    let notes_meta = attachment::service::get(&notes_imported.id).unwrap();
    assert!(notes_meta.compressed, "重复文本应被压缩");
    assert!(!notes_meta.compress_param.is_empty(), "压缩参数应非空");
    assert_eq!(attachment::service::load(&notes_imported.id).unwrap(), notes_bytes);

    // == 压缩层：zip-magic 直通路径 - infer 识别为 zip 应 bypass 压缩 ==
    let fake_path = source_dir.join("fake.txt");
    let mut fake_bytes = vec![0x50, 0x4B, 0x03, 0x04, 0x14, 0, 0, 0, 8, 0];
    fake_bytes.extend_from_slice(&[0u8; 22]);
    file_system_util::write(&fake_path, &fake_bytes).unwrap();
    let fake_imported = attachment::service::import(&node.id, &fake_path.to_string_lossy()).unwrap();
    let fake_meta = attachment::service::get(&fake_imported.id).unwrap();
    assert!(!fake_meta.compressed, "zip-magic 应直通不压缩");
    assert!(fake_meta.compress_param.is_empty(), "直通时压缩参数应为空串");
    assert_eq!(attachment::service::load(&fake_imported.id).unwrap(), fake_bytes);

    // == 压缩层：WAV flac 路径端到端 - 标准 PCM WAV 经 flac 压缩后 load 应 bit-exact 还原 ==
    let wav_path = source_dir.join("sound.wav");
    let pcm_data: Vec<u8> = (0..64u16).flat_map(|i| i.to_le_bytes()).collect();
    let mut wav_bytes: Vec<u8> = Vec::new();
    // RIFF header
    wav_bytes.extend_from_slice(b"RIFF");
    let file_size = (36 + pcm_data.len()) as u32;
    wav_bytes.extend_from_slice(&file_size.to_le_bytes());
    wav_bytes.extend_from_slice(b"WAVE");
    // fmt chunk
    wav_bytes.extend_from_slice(b"fmt ");
    wav_bytes.extend_from_slice(&16u32.to_le_bytes()); // chunk size
    wav_bytes.extend_from_slice(&1u16.to_le_bytes()); // PCM format
    wav_bytes.extend_from_slice(&1u16.to_le_bytes()); // channels
    wav_bytes.extend_from_slice(&44100u32.to_le_bytes()); // sample rate
    wav_bytes.extend_from_slice(&(44100u32 * 2).to_le_bytes()); // byte rate
    wav_bytes.extend_from_slice(&2u16.to_le_bytes()); // block align
    wav_bytes.extend_from_slice(&16u16.to_le_bytes()); // bits per sample (u16 两字节)
    // data chunk
    wav_bytes.extend_from_slice(b"data");
    wav_bytes.extend_from_slice(&(pcm_data.len() as u32).to_le_bytes());
    wav_bytes.extend_from_slice(&pcm_data);
    file_system_util::write(&wav_path, &wav_bytes).unwrap();
    let wav_imported = attachment::service::import(&node.id, &wav_path.to_string_lossy()).unwrap();
    let wav_meta = attachment::service::get(&wav_imported.id).unwrap();
    assert!(wav_meta.compressed, "标准 PCM WAV 应被 flac 压缩");
    assert_eq!(attachment::service::load(&wav_imported.id).unwrap(), wav_bytes);

    // == 压缩层：update_file 压缩路径 - 文本附件更新后 compressed 保持 true、size 与 load 同步 ==
    let updated_notes = b"updated text content. ".repeat(50).to_vec();
    attachment::service::update_file(&notes_imported.id, &updated_notes).unwrap();
    let notes_meta_after = attachment::service::get(&notes_imported.id).unwrap();
    assert!(notes_meta_after.compressed, "文本附件更新后仍应被压缩");
    assert_eq!(notes_meta_after.size, updated_notes.len() as i64);
    assert_eq!(attachment::service::load(&notes_imported.id).unwrap(), updated_notes);

    // == 压缩层：update_file 直通刷新路径 - zip-magic 附件写入另一段 zip-magic 保持 compressed=false ==
    let mut new_fake_bytes = vec![0x50, 0x4B, 0x03, 0x04, 0x0A, 0, 0, 0, 0, 0];
    new_fake_bytes.extend_from_slice(&[0u8; 22]);
    attachment::service::update_file(&fake_imported.id, &new_fake_bytes).unwrap();
    let fake_meta_after = attachment::service::get(&fake_imported.id).unwrap();
    assert!(!fake_meta_after.compressed, "zip-magic 更新后仍应直通不压缩");
    assert_eq!(attachment::service::load(&fake_imported.id).unwrap(), new_fake_bytes);

    // == attachment::load 失败路径：id 不存在 → NoAttachmentWithSuchId ==
    assert!(matches!(
        attachment::service::load(&uuid::Uuid::new_v4().to_string()),
        Err(ErrorCode::NoAttachmentWithSuchId { .. })
    ));

    // == attachment::load 失败路径：附件文件缺失（上面已删除 second 的文件）→ FailToReadFile ==
    assert!(matches!(
        attachment::service::load(&second.id),
        Err(ErrorCode::FailToReadFile { .. })
    ));

    // == attachment::get 成功路径：返回附件元数据，字段与导入时一致 ==
    let fetched = attachment::service::get(&imported.id).unwrap();
    assert_eq!(fetched.node_id, node.id);
    assert_eq!(fetched.file_name, "report.pdf");
    assert_eq!(fetched.size, source_bytes.len() as i64);
    assert!(!fetched.deleted);

    // == attachment::get 失败路径：id 不存在 → NoAttachmentWithSuchId ==
    assert!(matches!(
        attachment::service::get(&uuid::Uuid::new_v4().to_string()),
        Err(ErrorCode::NoAttachmentWithSuchId { .. })
    ));

    // == attachment::export 成功路径：导出文件与源字节一致 ==
    let export_file = source_dir.join("exported.pdf");
    attachment::service::export(&imported.id, &export_file.to_string_lossy()).unwrap();
    assert_eq!(file_system_util::read(&export_file).unwrap(), source_bytes);

    // == attachment::export 失败路径：id 不存在 → NoAttachmentWithSuchId ==
    assert!(matches!(
        attachment::service::export(
            &uuid::Uuid::new_v4().to_string(),
            &export_file.to_string_lossy()
        ),
        Err(ErrorCode::NoAttachmentWithSuchId { .. })
    ));

    // == attachment::update_file 成功路径：明文被覆盖、size 同步更新、产生 AttachmentUpdate 日志 ==
    // second 的附件文件在 missing_file 测试中被删掉，此时由 update_file 直接写入新内容重建。
    let new_bytes: Vec<u8> = (0..2048u32).map(|i| (i * 7 % 251) as u8).collect();
    let log_total_before = log::service::list(0, 1).unwrap().total;
    attachment::service::update_file(&second.id, &new_bytes).unwrap();
    assert_eq!(attachment::service::load(&second.id).unwrap(), new_bytes);
    let updated = attachment::service::get(&second.id).unwrap();
    assert_eq!(updated.size, new_bytes.len() as i64);
    assert_eq!(log::service::list(0, 1).unwrap().total, log_total_before + 1);

    // == attachment::update_file 失败路径：id 不存在 → NoAttachmentWithSuchId ==
    assert!(matches!(
        attachment::service::update_file(&uuid::Uuid::new_v4().to_string(), &new_bytes),
        Err(ErrorCode::NoAttachmentWithSuchId { .. })
    ));

    // == attachment::update_file 失败路径：明文超过大小上限 → AttachmentTooLarge ==
    let too_big = vec![
        0u8;
        (attachment::service::MAX_ATTACHMENT_SIZE_MB + 1) as usize * 1024 * 1024
    ];
    assert!(matches!(
        attachment::service::update_file(&second.id, &too_big),
        Err(ErrorCode::AttachmentTooLarge { .. })
    ));

    // == attachment::logical_delete 成功路径：deleted 翻转、文件保留不动、产生日志 ==
    let log_total_before = log::service::list(0, 1).unwrap().total;
    attachment::service::logical_delete(&imported.id).unwrap();
    // 文件保留不动。
    assert!(file_system_util::try_exists(&attachment_file).unwrap());
    // deleted 翻转：正常列表剩 second 与压缩层测试新增的 3 个附件，回收站列表出现 imported。
    let normal = attachment::service::list(&node.id, false).unwrap();
    assert_eq!(normal.len(), 4);
    assert_eq!(normal[0].id, second.id);
    let trash = attachment::service::list(&node.id, true).unwrap();
    assert_eq!(trash.len(), 1);
    assert_eq!(trash[0].id, imported.id);
    // 产生一条 AttachmentLogicalDelete 日志。
    assert_eq!(log::service::list(0, 1).unwrap().total, log_total_before + 1);

    // == attachment::load 成功路径：已逻辑删除的附件仍可加载（回收站中允许预览/导出）==
    assert_eq!(attachment::service::load(&imported.id).unwrap(), source_bytes);

    // == attachment::logical_delete 失败路径：id 不存在 → NoAttachmentWithSuchId ==
    assert!(matches!(
        attachment::service::logical_delete(&uuid::Uuid::new_v4().to_string()),
        Err(ErrorCode::NoAttachmentWithSuchId { .. })
    ));

    // == attachment::restore 成功路径：deleted 翻转回正常列表、产生日志 ==
    let log_total_before = log::service::list(0, 1).unwrap().total;
    attachment::service::restore(&imported.id).unwrap();
    assert_eq!(attachment::service::list(&node.id, false).unwrap().len(), 5);
    assert!(attachment::service::list(&node.id, true).unwrap().is_empty());
    assert_eq!(log::service::list(0, 1).unwrap().total, log_total_before + 1);

    // == attachment::restore 失败路径：id 不存在 → NoAttachmentWithSuchId ==
    assert!(matches!(
        attachment::service::restore(&uuid::Uuid::new_v4().to_string()),
        Err(ErrorCode::NoAttachmentWithSuchId { .. })
    ));

    // == attachment::physical_delete 成功路径：行与文件均消失、产生日志 ==
    // 先把 second 的附件文件补回来（上面为测 missing_file 删掉了）。
    file_system_util::write(
        &second_file,
        &crate::security::aes::encrypt(source_bytes.clone(), test::test_key()).unwrap(),
    )
    .unwrap();
    let log_total_before = log::service::list(0, 1).unwrap().total;
    attachment::service::physical_delete(&second.id).unwrap();
    {
        let conn = state::lock_connection();
        assert!(
            attachment::dao::select_by_id(&conn, &second.id)
                .unwrap()
                .is_none()
        );
    }
    assert!(!file_system_util::try_exists(&second_file).unwrap());
    assert_eq!(log::service::list(0, 1).unwrap().total, log_total_before + 1);

    // == attachment::physical_delete 失败路径：id 不存在 → NoAttachmentWithSuchId ==
    assert!(matches!(
        attachment::service::physical_delete(&uuid::Uuid::new_v4().to_string()),
        Err(ErrorCode::NoAttachmentWithSuchId { .. })
    ));

    // == attachment 日志载荷验证：import / export / logical_delete / restore / physical_delete
    //    均以节点 id 记录并携带节点标题与文件名 ==
    let logs = log::service::list(0, 1000).unwrap();
    let has = |object_id: &str, predicate: fn(&entity::Action) -> bool| {
        logs.items
            .iter()
            .any(|entry| entry.object_id == object_id && predicate(&entry.action))
    };
    assert!(has(
        &node.id,
        |action| matches!(action, entity::Action::AttachmentImport { node_title, file_name } if node_title == "attach-node" && file_name == "report.pdf")
    ));
    assert!(has(
        &node.id,
        |action| matches!(action, entity::Action::AttachmentExport { node_title, file_name } if node_title == "attach-node" && file_name == "report.pdf")
    ));
    assert!(has(
        &node.id,
        |action| matches!(action, entity::Action::AttachmentLogicalDelete { node_title, file_name } if node_title == "attach-node" && file_name == "report.pdf")
    ));
    assert!(has(
        &node.id,
        |action| matches!(action, entity::Action::AttachmentRestore { node_title, file_name } if node_title == "attach-node" && file_name == "report.pdf")
    ));
    assert!(has(
        &node.id,
        |action| matches!(action, entity::Action::AttachmentPhysicalDelete { node_title, file_name } if node_title == "attach-node" && file_name == "report.pdf")
    ));
    assert!(has(
        &node.id,
        |action| matches!(action, entity::Action::AttachmentUpdate { node_title, file_name } if node_title == "attach-node" && file_name == "report.pdf")
    ));

    // == 孤儿文件：手放 <uuid>.bin 与无法解析为 uuid 的异常文件到附件目录 ==
    let orphan_id = uuid::Uuid::new_v4().to_string();
    let orphan_file = path.user_attachment_file(&id, &orphan_id);
    file_system_util::write(&orphan_file, b"orphan").unwrap();
    let weird_file = path.user_attachment_directory(&id).join("weird.txt");
    file_system_util::write(&weird_file, b"weird").unwrap();

    // == attachment::list_orphan_files 成功路径：孤儿文件与异常文件被上报，有元数据的附件文件不在其中 ==
    let orphans = attachment::service::list_orphan_files().unwrap();
    assert!(orphans.contains(&orphan_id));
    assert!(orphans.contains(&"weird.txt".to_string()));
    assert!(!orphans.contains(&imported.id));

    // == attachment::remove_orphan_file 成功路径：文件消失且不再被上报；异常文件保留 ==
    attachment::service::remove_orphan_file(&orphan_id).unwrap();
    assert!(!file_system_util::try_exists(&orphan_file).unwrap());
    let orphans = attachment::service::list_orphan_files().unwrap();
    assert!(!orphans.contains(&orphan_id));
    assert!(orphans.contains(&"weird.txt".to_string()));

    // == attachment::remove_orphan_file 防御路径：文件不存在时不报错（try_exists 防御）==
    attachment::service::remove_orphan_file(&orphan_id).unwrap();
    // 清理异常文件。
    file_system_util::remove_file(&weird_file).unwrap();

    // == 级联：物理删除节点 → 附件行与文件一并消失 ==
    let cascade_node = node::service::create(
        &root_id,
        "cascade-node".to_string(),
        String::new(),
        0.0,
        0.0,
        None,
        false,
    )
    .unwrap();
    let cascade_attachment = attachment::service::import(&cascade_node.id, &source_path).unwrap();
    let cascade_file = path.user_attachment_file(&id, &cascade_attachment.id);
    assert!(file_system_util::try_exists(&cascade_file).unwrap());
    node::service::physical_delete(&cascade_node.id).unwrap();
    {
        let conn = state::lock_connection();
        assert!(
            attachment::dao::select_by_id(&conn, &cascade_attachment.id)
                .unwrap()
                .is_none()
        );
    }
    assert!(!file_system_util::try_exists(&cascade_file).unwrap());

    // == 级联：物理删除画布 → 其内节点（含回收站中已逻辑删除的节点）的附件行与文件消失；
    //    顺带验证 node_field 修复（字段行一并消失）==
    let cascade_canvas = canvas::service::create(&root_id, "cascade-canvas".to_string()).unwrap();
    let node_a = node::service::create(
        &cascade_canvas.id,
        "node-a".to_string(),
        String::new(),
        0.0,
        0.0,
        None,
        false,
    )
    .unwrap();
    let node_b = node::service::create(
        &cascade_canvas.id,
        "node-b".to_string(),
        String::new(),
        0.0,
        0.0,
        None,
        false,
    )
    .unwrap();
    // 把 node_b 放入回收站，验证回收站中的节点附件也被级联清理。
    node::service::logical_delete(&node_b.id).unwrap();
    let attach_a = attachment::service::import(&node_a.id, &source_path).unwrap();
    let attach_b = attachment::service::import(&node_b.id, &source_path).unwrap();
    node_field::service::set(
        &node_a.id,
        &[NodeFieldVO {
            name: "cf".to_string(),
            field_type: "TextSingleLine".to_string(),
            type_config: None,
            value: FieldValue::String(Some("v".to_string())),
            dictionary_id: None,
        }],
    )
    .unwrap();
    let file_a = path.user_attachment_file(&id, &attach_a.id);
    let file_b = path.user_attachment_file(&id, &attach_b.id);
    canvas::service::physical_delete(&cascade_canvas.id).unwrap();
    {
        let conn = state::lock_connection();
        assert!(
            attachment::dao::select_by_id(&conn, &attach_a.id)
                .unwrap()
                .is_none()
        );
        assert!(
            attachment::dao::select_by_id(&conn, &attach_b.id)
                .unwrap()
                .is_none()
        );
        // node_field 修复验证：画布内节点的字段行一并消失。
        assert!(
            node_field::dao::select_by_node_id(&conn, &node_a.id)
                .unwrap()
                .is_empty()
        );
    }
    assert!(!file_system_util::try_exists(&file_a).unwrap());
    assert!(!file_system_util::try_exists(&file_b).unwrap());

    // == 级联：物理删除被引用的画布 → 引用节点的附件行与文件、字段行一并消失 ==
    let cv_node = node::service::create(
        &root_id,
        "cv-node".to_string(),
        String::new(),
        0.0,
        0.0,
        None,
        true,
    )
    .unwrap();
    let cv_canvas_id = cv_node.canvas_ref_id.clone().unwrap();
    let attach_cv = attachment::service::import(&cv_node.id, &source_path).unwrap();
    node_field::service::set(
        &cv_node.id,
        &[NodeFieldVO {
            name: "cf".to_string(),
            field_type: "TextSingleLine".to_string(),
            type_config: None,
            value: FieldValue::String(Some("v".to_string())),
            dictionary_id: None,
        }],
    )
    .unwrap();
    let file_cv = path.user_attachment_file(&id, &attach_cv.id);
    canvas::service::physical_delete(&cv_canvas_id).unwrap();
    {
        let conn = state::lock_connection();
        assert!(
            attachment::dao::select_by_id(&conn, &attach_cv.id)
                .unwrap()
                .is_none()
        );
        // node_field 修复验证：引用节点的字段行一并消失。
        assert!(
            node_field::dao::select_by_node_id(&conn, &cv_node.id)
                .unwrap()
                .is_empty()
        );
    }
    assert!(!file_system_util::try_exists(&file_cv).unwrap());

    // == 清理 ==
    lifecycle::service::save().unwrap();
    lifecycle::service::close().unwrap();
    }

    // == 阶段五：canvas / node set_color 与 node color_list ==
    {
    let id = registered.id.clone();
    lifecycle::service::initialize(&id, test::test_key()).unwrap();

    let canvases = canvas::service::list(false).unwrap();
    let root_id = &canvases[0].id;

    // == canvas::set_color 失败路径：画布不存在 → NoCanvasWithSuchId ==
    assert!(matches!(
        canvas::service::set_color(&uuid::Uuid::new_v4().to_string(), "{\"fill\":\"#ff0000\"}".to_string()),
        Err(ErrorCode::NoCanvasWithSuchId { .. })
    ));

    // == canvas::set_color 成功路径：设置后通过 list 确认 color 已持久化 ==
    canvas::service::set_color(root_id, "{\"fill\":\"#112233\"}".to_string()).unwrap();
    let root_after = canvas::service::list(false).unwrap().into_iter().find(|c| c.id == *root_id).unwrap();
    assert_eq!(root_after.color, "{\"fill\":\"#112233\"}");

    // == node::set_color 失败路径：节点不存在 → NoNodeWithSuchId ==
    assert!(matches!(
        node::service::set_color(&uuid::Uuid::new_v4().to_string(), "{\"fill\":\"#ff0000\"}".to_string()),
        Err(ErrorCode::NoNodeWithSuchId { .. })
    ));

    // == node::set_color 成功路径：设置后通过 list 确认 color 已持久化 ==
    let node = node::service::create(
        root_id,
        "color-node".to_string(),
        "sub".to_string(),
        0.0,
        0.0,
        None,
        false,
    ).unwrap();
    node::service::set_color(&node.id, "{\"fill\":\"#aabbcc\"}".to_string()).unwrap();
    let node_after = node::service::list(root_id, false).unwrap().into_iter().find(|n| n.id == node.id).unwrap();
    assert_eq!(node_after.color, "{\"fill\":\"#aabbcc\"}");

    // == node::color_list 成功路径：设置若干节点颜色（含空色、已删除节点）后返回结果符合预期 ==
    // 再创建一个无色节点和一个已删除的带色节点。
    let plain_node = node::service::create(
        root_id,
        "plain-node".to_string(),
        String::new(),
        0.0,
        0.0,
        None,
        false,
    ).unwrap();
    let deleted_colored = node::service::create(
        root_id,
        "deleted-colored".to_string(),
        String::new(),
        0.0,
        0.0,
        None,
        false,
    ).unwrap();
    node::service::set_color(&deleted_colored.id, "{\"fill\":\"#0000ff\"}".to_string()).unwrap();
    node::service::logical_delete(&deleted_colored.id).unwrap();
    // 无色节点 color 保持空串，不应出现在结果中。
    let _ = plain_node;

    let entries = node::service::color_list().unwrap();
    // 只有 color-node 符合条件（未删除且 color 非空）
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].title, "color-node");
    assert_eq!(entries[0].color, "{\"fill\":\"#aabbcc\"}");

    // == canvas::color_list 成功路径：根画布已带色；另建无色画布与已删除带色画布，验证只返回未删除带色画布 ==
    let plain_canvas = canvas::service::create(root_id, "plain-canvas".to_string()).unwrap();
    let deleted_colored_canvas = canvas::service::create(root_id, "deleted-colored-canvas".to_string()).unwrap();
    canvas::service::set_color(&deleted_colored_canvas.id, "{\"fill\":\"#00ff00\"}".to_string()).unwrap();
    canvas::service::logical_delete(&deleted_colored_canvas.id).unwrap();
    // 无色画布 color 保持空串，不应出现在结果中。
    let _ = plain_canvas;

    let entries = canvas::service::color_list().unwrap();
    // 只有根画布符合条件（未删除且 color 非空）
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].name, entity::ROOT_CANVAS_NAME);
    assert!(entries[0].parent_id.is_none());
    assert_eq!(entries[0].color, "{\"fill\":\"#112233\"}");

    lifecycle::service::save().unwrap();
    lifecycle::service::close().unwrap();
    }

    // == 阶段六：move_nodes / move_canvases 自动布局批量移动 ==
    {
    let id = registered.id.clone();
    lifecycle::service::initialize(&id, test::test_key()).unwrap();

    // 根画布通过 parent_id 为 None 来识别（list 按名称排序，不能直接取 [0]）。
    let root_id = canvas::service::list(false)
        .unwrap()
        .into_iter()
        .find(|c| c.parent_id.is_none())
        .unwrap()
        .id;

    // 在根画布下创建一个子画布。
    let child = canvas::service::create(&root_id, "child-for-batch".to_string()).unwrap();

    // 在子画布内创建 3 个节点。
    let node_a = node::service::create(
        &child.id,
        "batch-node-a".to_string(),
        "sub-a".to_string(),
        10.0,
        20.0,
        None,
        false,
    )
    .unwrap();
    let node_b = node::service::create(
        &child.id,
        "batch-node-b".to_string(),
        "sub-b".to_string(),
        30.0,
        40.0,
        None,
        false,
    )
    .unwrap();
    let node_c = node::service::create(
        &child.id,
        "batch-node-c".to_string(),
        "sub-c".to_string(),
        50.0,
        60.0,
        None,
        false,
    )
    .unwrap();

    // move_nodes 失败路径：含不存在 id 时报 NoNodeWithSuchId 且其它节点坐标未被更新。
    let items_with_invalid = vec![
        node::vo::MoveNodeVO {
            id: node_a.id.clone(),
            x: 100.0,
            y: 200.0,
        },
        node::vo::MoveNodeVO {
            id: uuid::Uuid::new_v4().to_string(),
            x: 0.0,
            y: 0.0,
        },
    ];
    assert!(matches!(
        node::service::move_nodes(&items_with_invalid),
        Err(ErrorCode::NoNodeWithSuchId { .. })
    ));
    // 验证 node_a 坐标未被更新。
    let unchanged = node::service::list(&child.id, false)
        .unwrap()
        .into_iter()
        .find(|n| n.id == node_a.id)
        .unwrap();
    assert_eq!((unchanged.x, unchanged.y), (10.0, 20.0));

    // move_nodes 成功路径：批量移动（node_c 原地移动，实际位移 2 个）。
    let log_total_before = log::service::list(0, 1).unwrap().total;
    let items = vec![
        node::vo::MoveNodeVO {
            id: node_a.id.clone(),
            x: 100.0,
            y: 200.0,
        },
        node::vo::MoveNodeVO {
            id: node_b.id.clone(),
            x: 300.0,
            y: 400.0,
        },
        node::vo::MoveNodeVO {
            id: node_c.id.clone(),
            x: 50.0,
            y: 60.0,
        },
    ];
    node::service::move_nodes(&items).unwrap();
    // 验证坐标更新。
    let updated_a = node::service::list(&child.id, false)
        .unwrap()
        .into_iter()
        .find(|n| n.id == node_a.id)
        .unwrap();
    assert_eq!((updated_a.x, updated_a.y), (100.0, 200.0));
    let updated_b = node::service::list(&child.id, false)
        .unwrap()
        .into_iter()
        .find(|n| n.id == node_b.id)
        .unwrap();
    assert_eq!((updated_b.x, updated_b.y), (300.0, 400.0));
    // node_c 原地移动，坐标不变。
    let updated_c = node::service::list(&child.id, false)
        .unwrap()
        .into_iter()
        .find(|n| n.id == node_c.id)
        .unwrap();
    assert_eq!((updated_c.x, updated_c.y), (50.0, 60.0));
    // 验证只产生一条日志，action 为 AutoLayoutDataNodes，node_count=2，object_id 为画布 id。
    let logs_after = log::service::list(0, 1000).unwrap();
    assert_eq!(logs_after.total, log_total_before + 1);
    let auto_layout_log = logs_after
        .items
        .iter()
        .find(|e| matches!(e.action, entity::Action::AutoLayoutDataNodes { .. }))
        .unwrap();
    assert_eq!(auto_layout_log.object_id, child.id);
    assert!(matches!(
        auto_layout_log.action,
        entity::Action::AutoLayoutDataNodes { node_count } if node_count == 2
    ));

    // move_nodes 成功路径：空列表成功返回且不产日志。
    let log_total_before_empty = log::service::list(0, 1).unwrap().total;
    node::service::move_nodes(&[]).unwrap();
    assert_eq!(log::service::list(0, 1).unwrap().total, log_total_before_empty);

    // move_nodes 成功路径：全部原地移动成功返回且不产日志。
    let log_total_before_no_move = log::service::list(0, 1).unwrap().total;
    let items_no_move = vec![
        node::vo::MoveNodeVO {
            id: node_a.id.clone(),
            x: 100.0,
            y: 200.0,
        },
        node::vo::MoveNodeVO {
            id: node_b.id.clone(),
            x: 300.0,
            y: 400.0,
        },
    ];
    node::service::move_nodes(&items_no_move).unwrap();
    assert_eq!(
        log::service::list(0, 1).unwrap().total,
        log_total_before_no_move
    );

    // move_canvases 失败路径：含不存在 id 时报 NoCanvasWithSuchId 且其它画布坐标未被更新。
    let canvas_d = canvas::service::create(&root_id, "canvas-d".to_string()).unwrap();
    let canvas_e = canvas::service::create(&root_id, "canvas-e".to_string()).unwrap();
    let items_canvas_invalid = vec![
        canvas::vo::MoveNodeVO {
            id: canvas_d.id.clone(),
            x: 500.0,
            y: 600.0,
        },
        canvas::vo::MoveNodeVO {
            id: uuid::Uuid::new_v4().to_string(),
            x: 0.0,
            y: 0.0,
        },
    ];
    assert!(matches!(
        canvas::service::move_canvases(&items_canvas_invalid),
        Err(ErrorCode::NoCanvasWithSuchId { .. })
    ));
    // 验证 canvas_d 坐标未被更新（新创建的画布坐标为 layout 计算出的值，不等于 (0,0)）。
    let unchanged_d = canvas::service::list(false)
        .unwrap()
        .into_iter()
        .find(|c| c.id == canvas_d.id)
        .unwrap();
    // 只要没变成 (500, 600) 即可证明未更新。
    assert_ne!((unchanged_d.x, unchanged_d.y), (500.0, 600.0));

    // move_canvases 成功路径：批量移动（canvas_e 原地移动，实际位移 1 个）。
    // 先获取 canvas_d 和 canvas_e 当前坐标。
    let canvas_d_before = canvas::service::list(false)
        .unwrap()
        .into_iter()
        .find(|c| c.id == canvas_d.id)
        .unwrap();
    let canvas_e_before = canvas::service::list(false)
        .unwrap()
        .into_iter()
        .find(|c| c.id == canvas_e.id)
        .unwrap();
    let log_total_before_canvas = log::service::list(0, 1).unwrap().total;
    let items_canvas = vec![
        canvas::vo::MoveNodeVO {
            id: canvas_d.id.clone(),
            x: 500.0,
            y: 600.0,
        },
        canvas::vo::MoveNodeVO {
            id: canvas_e.id.clone(),
            x: canvas_e_before.x,
            y: canvas_e_before.y,
        },
    ];
    canvas::service::move_canvases(&items_canvas).unwrap();
    // 验证 canvas_d 坐标更新。
    let updated_d = canvas::service::list(false)
        .unwrap()
        .into_iter()
        .find(|c| c.id == canvas_d.id)
        .unwrap();
    assert_eq!((updated_d.x, updated_d.y), (500.0, 600.0));
    // 验证 canvas_e 坐标不变（原地移动）。
    let updated_e = canvas::service::list(false)
        .unwrap()
        .into_iter()
        .find(|c| c.id == canvas_e.id)
        .unwrap();
    assert_eq!((updated_e.x, updated_e.y), (canvas_e_before.x, canvas_e_before.y));
    // 验证只产生一条日志，action 为 AutoLayoutCanvasNodes，canvas_count=1，object_id 为根画布 id。
    let logs_after_canvas = log::service::list(0, 1000).unwrap();
    assert_eq!(logs_after_canvas.total, log_total_before_canvas + 1);
    let auto_layout_canvas_log = logs_after_canvas
        .items
        .iter()
        .find(|e| matches!(e.action, entity::Action::AutoLayoutCanvasNodes { .. }))
        .unwrap();
    assert_eq!(auto_layout_canvas_log.object_id, root_id);
    assert!(matches!(
        auto_layout_canvas_log.action,
        entity::Action::AutoLayoutCanvasNodes { canvas_count } if canvas_count == 1
    ));

    // move_canvases 成功路径：空列表成功返回且不产日志。
    let log_total_before_empty_canvas = log::service::list(0, 1).unwrap().total;
    canvas::service::move_canvases(&[]).unwrap();
    assert_eq!(
        log::service::list(0, 1).unwrap().total,
        log_total_before_empty_canvas
    );

    // move_canvases 成功路径：全部原地移动成功返回且不产日志。
    let log_total_before_no_move_canvas = log::service::list(0, 1).unwrap().total;
    let items_canvas_no_move = vec![canvas::vo::MoveNodeVO {
        id: canvas_d.id.clone(),
        x: 500.0,
        y: 600.0,
    }];
    canvas::service::move_canvases(&items_canvas_no_move).unwrap();
    assert_eq!(
        log::service::list(0, 1).unwrap().total,
        log_total_before_no_move_canvas
    );

    lifecycle::service::save().unwrap();
    lifecycle::service::close().unwrap();
    }

    test::cleanup(&path);
}
