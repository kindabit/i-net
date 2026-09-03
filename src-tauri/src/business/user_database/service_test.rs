use super::*;
use crate::business::metadata;
use crate::business::user_database::entity::Dictionary;
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

    // ===== 画布节点标题与画布名称双向同步 =====

    // node::modify 同步成功路径：修改画布节点标题，引用画布的名称随之更新，
    // 产生 NodeModify + CanvasRename 两条日志。
    let log_total_before_sync = log::service::list(0, 1).unwrap().total;
    node::service::modify(
        &canvas_node.id,
        "canvas-node-renamed".to_string(),
        "canvas-node-sub".to_string(),
    )
    .unwrap();
    {
        let conn = state::lock_connection();
        let synced_node = node::dao::select_by_id(&conn, &canvas_node.id)
            .unwrap()
            .unwrap();
        assert_eq!(synced_node.title, "canvas-node-renamed");
        assert_eq!(synced_node.sub_title, "canvas-node-sub");
        let synced_canvas = canvas::dao::select_by_id(
            &conn,
            canvas_node.canvas_ref_id.as_ref().unwrap(),
        )
        .unwrap()
        .unwrap();
        assert_eq!(synced_canvas.name, "canvas-node-renamed");
    }
    assert_eq!(
        log::service::list(0, 1).unwrap().total,
        log_total_before_sync + 2
    );

    // node::modify 同步失败路径：新标题与其它画布（"canvas-node 2"）重名时报 CanvasNameAlreadyExists，
    // 节点标题/副标题与画布名称均保持原值（无任何落库，也不产生日志）。
    let log_total_before_conflict = log::service::list(0, 1).unwrap().total;
    assert!(matches!(
        node::service::modify(
            &canvas_node.id,
            "canvas-node 2".to_string(),
            "should-not-persist".to_string(),
        ),
        Err(ErrorCode::CanvasNameAlreadyExists { .. })
    ));
    {
        let conn = state::lock_connection();
        let unchanged_node = node::dao::select_by_id(&conn, &canvas_node.id)
            .unwrap()
            .unwrap();
        assert_eq!(unchanged_node.title, "canvas-node-renamed");
        assert_eq!(unchanged_node.sub_title, "canvas-node-sub");
        let unchanged_canvas = canvas::dao::select_by_id(
            &conn,
            canvas_node.canvas_ref_id.as_ref().unwrap(),
        )
        .unwrap()
        .unwrap();
        assert_eq!(unchanged_canvas.name, "canvas-node-renamed");
    }
    assert_eq!(
        log::service::list(0, 1).unwrap().total,
        log_total_before_conflict
    );

    // node::modify 标题未变化路径：仅修改副标题，不触发画布同步，只产生 NodeModify 一条日志。
    let log_total_before_sub_only = log::service::list(0, 1).unwrap().total;
    node::service::modify(
        &canvas_node.id,
        "canvas-node-renamed".to_string(),
        "canvas-node-sub-2".to_string(),
    )
    .unwrap();
    assert_eq!(
        log::service::list(0, 1).unwrap().total,
        log_total_before_sub_only + 1
    );

    // canvas::rename 同步成功路径：重命名画布，引用节点的标题随之更新（副标题不变），
    // 产生 CanvasRename + NodeModify 两条日志。
    let canvas_node_ref_id = canvas_node.canvas_ref_id.clone().unwrap();
    let log_total_before_rename = log::service::list(0, 1).unwrap().total;
    canvas::service::rename(&canvas_node_ref_id, "canvas-node-back".to_string()).unwrap();
    {
        let conn = state::lock_connection();
        let synced_node = node::dao::select_by_id(&conn, &canvas_node.id)
            .unwrap()
            .unwrap();
        assert_eq!(synced_node.title, "canvas-node-back");
        assert_eq!(synced_node.sub_title, "canvas-node-sub-2");
    }
    assert_eq!(
        log::service::list(0, 1).unwrap().total,
        log_total_before_rename + 2
    );

    // canvas::rename 同步回收站节点：引用节点已逻辑删除（引用画布被级联逻辑删除）时标题仍同步。
    node::service::logical_delete(&canvas_node.id).unwrap();
    canvas::service::rename(&canvas_node_ref_id, "canvas-node-deleted".to_string()).unwrap();
    {
        let conn = state::lock_connection();
        let deleted_synced = node::dao::select_by_id(&conn, &canvas_node.id)
            .unwrap()
            .unwrap();
        assert!(deleted_synced.deleted);
        assert_eq!(deleted_synced.title, "canvas-node-deleted");
    }
    // 恢复节点（级联恢复引用画布），避免影响后续测试。
    node::service::restore(&canvas_node.id, 50.0, 60.0).unwrap();

    // node::modify 数据损坏路径：画布节点引用的画布不存在（理论不可能）时
    // 报 DataCorruptionCanvasRefMissing 触发受控崩溃，载荷为节点 id 与缺失的画布 id。
    // 临时关闭外键以注入不一致数据（直接删除画布而保留引用节点），断言后清理现场并恢复外键。
    let corrupt_node = node::service::create(
        &child.id,
        "corrupt-canvas-node".to_string(),
        String::new(),
        90.0,
        100.0,
        None,
        true,
    )
    .unwrap();
    {
        let connection = state::lock_connection();
        connection.execute_batch("PRAGMA foreign_keys = OFF;").unwrap();
        canvas::dao::delete_by_id(
            &connection,
            corrupt_node.canvas_ref_id.as_ref().unwrap(),
        )
        .unwrap();
        assert!(matches!(
            node::service::modify(
                &corrupt_node.id,
                "corrupt-renamed".to_string(),
                String::new(),
            ),
            Err(ErrorCode::DataCorruptionCanvasRefMissing { node_id, canvas_id })
                if node_id == corrupt_node.id
                    && canvas_id == corrupt_node.canvas_ref_id.as_deref().unwrap()
        ));
        node::dao::delete_by_id(&connection, &corrupt_node.id).unwrap();
        connection.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
    }

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
    node::service::physical_delete(&cascade_node.id, false).unwrap();
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
    // 新建边规则下画布节点→普通节点被禁止（CanvasToPlainNodeEdge），故此处改用普通节点 → 画布节点：
    // 该边在画布节点引用的子画布内产生入向影子（建边规则 2）。
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
        &outsider.id,
        "right".to_string(),
        &c2_node.id,
        "left".to_string(),
        false,
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
        edge::service::delete(&c2_edge_id, false),
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
            "left".to_string(),
            false
        ),
        Err(ErrorCode::NoNodeWithSuchId { .. })
    ));
    assert!(matches!(
        edge::service::create(
            &child.id,
            &uuid::Uuid::new_v4().to_string(),
            "right".to_string(),
            &node_1.id,
            "left".to_string(),
            false
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
        false,
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

    // edge::create 同向重建路径：同一对节点之间同向重建时直接更新旧边的连接桩——
    // 边 id 不变、标题与详情保留、不产生新日志。
    let edge_1_id = edge_1.id.clone();
    let log_total_before = log::service::list(0, 1).unwrap().total;
    let edge_1 = edge::service::create(
        &child.id,
        &node_1.id,
        "top".to_string(),
        &node_2.id,
        "bottom".to_string(),
        false,
    )
    .unwrap();
    assert_eq!(edge_1.id, edge_1_id);
    assert_eq!(edge_1.source_port, "top");
    assert_eq!(edge_1.target_port, "bottom");
    assert_eq!(edge_1.title, "new title");
    assert_eq!(edge_1.description, "new description");
    assert_eq!(log::service::list(0, 1).unwrap().total, log_total_before);

    // edge::create 失败路径：两端连接桩相同时报 EdgeSameNodePort（早于查重与成环检查）。
    assert!(matches!(
        edge::service::create(
            &child.id,
            &node_1.id,
            "top".to_string(),
            &node_2.id,
            "top".to_string(),
            false
        ),
        Err(ErrorCode::EdgeSameNodePort)
    ));
    // 自环 + 同连接桩同样报 EdgeSameNodePort，覆盖同 port 检查先于 would_form_cycle 的语义。
    assert!(matches!(
        edge::service::create(
            &child.id,
            &node_1.id,
            "right".to_string(),
            &node_1.id,
            "right".to_string(),
            false
        ),
        Err(ErrorCode::EdgeSameNodePort)
    ));

    // edge::create 替换路径：反向建边（换向）在排除旧边后不成环时，删除旧边并建立反向新边。
    let reversed = edge::service::create(
        &child.id,
        &node_2.id,
        "right".to_string(),
        &node_1.id,
        "left".to_string(),
        false,
    )
    .unwrap();
    assert_eq!(reversed.source_id, node_2.id);
    assert_eq!(reversed.target_id, node_1.id);
    // 再次换向恢复 node_1 -> node_2 拓扑，供后续测试使用。
    let edge_1 = edge::service::create(
        &child.id,
        &node_1.id,
        "right".to_string(),
        &node_2.id,
        "left".to_string(),
        false,
    )
    .unwrap();

    // edge::create 失败路径：自环无旧边时仍报 EdgeWouldFormCycle。
    assert!(matches!(
        edge::service::create(
            &child.id,
            &node_1.id,
            "right".to_string(),
            &node_1.id,
            "left".to_string(),
            false
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
        node::service::physical_delete("no-such-id", false),
        Err(ErrorCode::NoNodeWithSuchId { .. })
    ));

    // node::physical_delete 成功路径：节点被物理删除，与它相连的边被一并删除
    // （再删除该边时报 NoEdgeWithSuchId，证明边已不存在）。
    node::service::physical_delete(&node_2.id, false).unwrap();
    assert!(node::service::list(&child.id, true)
        .unwrap()
        .iter()
        .all(|node| node.id != node_2.id));
    assert!(matches!(
        edge::service::delete(&edge_1.id, false),
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
        false,
    )
    .unwrap();
    edge::service::delete(&edge_2.id, false).unwrap();

    // edge::delete 失败路径：重复删除同一条边报 NoEdgeWithSuchId。
    assert!(matches!(
        edge::service::delete(&edge_2.id, false),
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
        |action| matches!(action, entity::Action::EdgeReplace { source_title, target_title, old_source_title, old_target_title }
            if source_title == "title-1" && target_title == "title-2"
                && old_source_title == "title-2" && old_target_title == "title-1")
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
            field_type: "string:single-line".to_string(),
            value: Some("a".to_string()),
            dictionary_id: None,
        },
        NodeFieldVO {
            name: "dup".to_string(),
            field_type: "string:single-line".to_string(),
            value: Some("b".to_string()),
            dictionary_id: None,
        },
    ];
    assert!(matches!(
        node_field::service::set(&node.id, &dup_fields),
        Err(ErrorCode::DuplicateNodeFieldName { .. })
    ));

    // == node_field::set 失败路径：dictionary_id 不存在 → NoDictionaryEntryWithSuchId ==
    assert!(matches!(
        node_field::service::set(
            &node.id,
            &[NodeFieldVO {
                name: "dict-missing".to_string(),
                field_type: "string:single-line".to_string(),
                value: Some("x".to_string()),
                dictionary_id: Some(uuid::Uuid::new_v4().to_string()),
            }]
        ),
        Err(ErrorCode::NoDictionaryEntryWithSuchId { .. })
    ));

    // == node_field::set 成功路径：覆盖多种字段类型的字段值字符串与带 dictionary_id 的字段。
    //    后端不校验字段类型与值内容，value 原样加密存取 ==
    let fields = vec![
        NodeFieldVO {
            name: "文本".to_string(),
            field_type: "string:single-line".to_string(),
            value: Some("hello".to_string()),
            dictionary_id: None,
        },
        NodeFieldVO {
            name: "数字".to_string(),
            field_type: "decimal:decimal".to_string(),
            value: Some("123.456".to_string()),
            dictionary_id: None,
        },
        NodeFieldVO {
            name: "日期".to_string(),
            field_type: "instant:instant".to_string(),
            value: Some(
                "2024-04-05 12:34:38.000|+8"
                    .to_string(),
            ),
            dictionary_id: None,
        },
        NodeFieldVO {
            name: "日期区间".to_string(),
            field_type: "instant-range:instant-range".to_string(),
            value: Some(
                "2024-01-01 00:00:00.000 ~ 2024-12-31 23:59:59.999|+8"
                    .to_string(),
            ),
            dictionary_id: None,
        },
        NodeFieldVO {
            name: "字典引用".to_string(),
            field_type: "string:single-line".to_string(),
            value: Some("x".to_string()),
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

    // == set 成功路径：无值字段（value 为 None）set/get 往返 ==
    let none_fields = vec![
        NodeFieldVO {
            name: "空文本".to_string(),
            field_type: "string:single-line".to_string(),
            value: None,
            dictionary_id: None,
        },
        NodeFieldVO {
            name: "空日期".to_string(),
            field_type: "instant:instant".to_string(),
            value: None,
            dictionary_id: None,
        },
    ];
    node_field::service::set(&node.id, &none_fields).unwrap();
    let got_none = node_field::service::get(&node.id).unwrap();
    assert_eq!(got_none.len(), 2);
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
            field_type: "string:single-line".to_string(),
            value: Some("y".to_string()),
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
        field_type: "string:single-line".to_string(),
        value: Some("overwritten".to_string()),
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
        field_type: "string:single-line".to_string(),
        value: Some("hello".to_string()),
        dictionary_id: None,
    };
    let f2 = NodeFieldVO {
        name: "f2".to_string(),
        field_type: "decimal:decimal".to_string(),
        value: Some("13.5".to_string()),
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
        if name == "f1" && field_type == "string:single-line" && value == &Some("hello".to_string())));
    assert!(matches!(&first_changes[1],
        entity::NodeFieldChange::Added { name, field_type, value }
        if name == "f2" && field_type == "decimal:decimal" && value == &Some("13.5".to_string())));

    // 再次 set 完全相同的内容 → 不产生新的 NodeFieldsModify 日志（日志总数不变）。
    node_field::service::set(&log_node.id, &[f1.clone(), f2.clone()]).unwrap();
    let after_same = log::service::list(0, 1000).unwrap();
    assert_eq!(after_same.total, after_set.total);

    // 修改 f1 的值 → 一条 Modified（old_value / new_value 正确）。
    let f1_modified = NodeFieldVO {
        name: "f1".to_string(),
        field_type: "string:single-line".to_string(),
        value: Some("updated".to_string()),
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
        && old_field_type == "string:single-line" && new_field_type == "string:single-line"
        && old_value == &Some("hello".to_string())
        && new_value == &Some("updated".to_string())));

    // 修改 f1 的类型（string:single-line → decimal:decimal）→ 一条 Modified（old/new field_type 正确）。
    let f1_type_change = NodeFieldVO {
        name: "f1".to_string(),
        field_type: "decimal:decimal".to_string(),
        value: Some("42".to_string()),
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
        && old_field_type == "string:single-line" && new_field_type == "decimal:decimal"
        && old_value == &Some("updated".to_string())
        && new_value == &Some("42".to_string())));

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
        if name == "f2" && field_type == "decimal:decimal" && old_value == &Some("13.5".to_string())));

    // 字段改名（f1 → f1-renamed）→ 一条 Removed（旧名）+ 一条 Added（新名）。
    let f1_renamed = NodeFieldVO {
        name: "f1-renamed".to_string(),
        field_type: "decimal:decimal".to_string(),
        value: Some("42".to_string()),
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
        if name == "f1" && field_type == "decimal:decimal" && old_value == &Some("42".to_string())));
    assert!(matches!(&rename_changes[1],
        entity::NodeFieldChange::Added { name, field_type, value }
        if name == "f1-renamed" && field_type == "decimal:decimal" && value == &Some("42".to_string())));

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
            field_type: "string:single-line".to_string(),
            value: Some("hello".to_string()),
            dictionary_id: None,
        },
        NodeFieldVO {
            name: "日期字段".to_string(),
            field_type: "instant:instant".to_string(),
            value: Some(
                "2024-04-05 12:34:38.000|+8"
                    .to_string(),
            ),
            dictionary_id: None,
        },
        NodeFieldVO {
            name: "字典字段".to_string(),
            field_type: "string:single-line".to_string(),
            value: Some("v".to_string()),
            dictionary_id: Some(dict_d.id.clone()),
        },
    ];
    node_field::service::set(&src_node.id, &node_fields).unwrap();
    let tpl_c = template::service::create_from_node(&src_node.id, "模板C".to_string()).unwrap();
    let tpl_c_fields = template::service::get_fields(&tpl_c.id).unwrap();
    assert_eq!(tpl_c_fields.len(), 3);
    // 字段名、类型、字典引用与源节点一致
    assert_eq!(tpl_c_fields[0].name, "文本字段");
    assert_eq!(tpl_c_fields[0].field_type, "string:single-line");
    assert!(tpl_c_fields[0].dictionary_id.is_none());
    assert_eq!(tpl_c_fields[1].name, "日期字段");
    assert_eq!(tpl_c_fields[1].field_type, "instant:instant");
    assert!(tpl_c_fields[1].dictionary_id.is_none());
    assert_eq!(tpl_c_fields[2].name, "字典字段");
    assert_eq!(tpl_c_fields[2].field_type, "string:single-line");
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
                field_type: "string:single-line".to_string(),
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
                    field_type: "string:single-line".to_string(),
                    dictionary_id: None,
                },
                TemplateFieldVO {
                    name: "dup".to_string(),
                    field_type: "string:single-line".to_string(),
                    dictionary_id: None,
                },
            ]
        ),
        Err(ErrorCode::DuplicateNodeFieldName { .. })
    ));

    // == set_fields 失败路径：字典引用不存在 → NoDictionaryEntryWithSuchId ==
    assert!(matches!(
        template::service::set_fields(
            &tpl_a.id,
            &[TemplateFieldVO {
                name: "missing-dict".to_string(),
                field_type: "string:single-line".to_string(),
                dictionary_id: Some(uuid::Uuid::new_v4().to_string()),
            }]
        ),
        Err(ErrorCode::NoDictionaryEntryWithSuchId { .. })
    ));

    // == set_fields / get_fields 成功往返（后端不校验字段类型，原样存取） ==
    let tpl_fields = vec![
        TemplateFieldVO {
            name: "模板字段1".to_string(),
            field_type: "string:single-line".to_string(),
            dictionary_id: None,
        },
        TemplateFieldVO {
            name: "模板字段2".to_string(),
            field_type: "instant:instant".to_string(),
            dictionary_id: None,
        },
        TemplateFieldVO {
            name: "模板字段3".to_string(),
            field_type: "string:single-line".to_string(),
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
    assert_eq!(tpl_node_fields[0].field_type, "string:single-line");
    assert_eq!(tpl_node_fields[0].value, None);
    assert_eq!(tpl_node_fields[1].name, "模板字段2");
    assert_eq!(tpl_node_fields[1].field_type, "instant:instant");
    assert_eq!(tpl_node_fields[1].value, None);
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
            field_type: "string:single-line".to_string(),
            value: Some("v".to_string()),
            dictionary_id: None,
        }],
    ).unwrap();
    let cascade_id = cascade_node.id.clone();
    node::service::physical_delete(&cascade_id, false).unwrap();
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
            field_type: "string:single-line".to_string(),
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
    node::service::physical_delete(&cascade_node.id, false).unwrap();
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
            field_type: "string:single-line".to_string(),
            value: Some("v".to_string()),
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
            field_type: "string:single-line".to_string(),
            value: Some("v".to_string()),
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

    // == node::copy 失败路径：源节点不存在 → NoNodeWithSuchId ==
    assert!(matches!(
        node::service::copy(&uuid::Uuid::new_v4().to_string(), 0.0, 0.0),
        Err(ErrorCode::NoNodeWithSuchId { .. })
    ));

    // == node::copy 失败路径：源节点是画布节点 → NodeIsCanvasNode ==
    let copy_canvas_node = node::service::create(
        root_id,
        "copy-canvas-node".to_string(),
        String::new(),
        0.0,
        0.0,
        None,
        true,
    ).unwrap();
    assert!(matches!(
        node::service::copy(&copy_canvas_node.id, 0.0, 0.0),
        Err(ErrorCode::NodeIsCanvasNode)
    ));

    // == node::copy 成功路径：副本继承标题、副标题、颜色和字段结构（值为 None），
    //    id 全新、坐标取入参、非删除态、canvas_ref_id 与 shadow_id 均为 None ==
    let copy_source = node::service::create(
        root_id,
        "copy-source".to_string(),
        "copy-sub".to_string(),
        10.0,
        20.0,
        None,
        false,
    ).unwrap();
    node::service::set_color(&copy_source.id, "{\"fill\":\"#aabbcc\"}".to_string()).unwrap();
    node_field::service::set(
        &copy_source.id,
        &[
            NodeFieldVO {
                name: "文本".to_string(),
                field_type: "string:single-line".to_string(),
                value: Some("secret".to_string()),
                dictionary_id: None,
            },
            NodeFieldVO {
                name: "日期".to_string(),
                field_type: "instant:instant".to_string(),
                value: Some(
                    "2024-04-05 12:34:38.000|+8"
                        .to_string(),
                ),
                dictionary_id: None,
            },
        ],
    ).unwrap();

    let copied = node::service::copy(&copy_source.id, 300.0, 400.0).unwrap();
    assert_ne!(copied.id, copy_source.id);
    assert_eq!(copied.canvas_id, *root_id);
    assert_eq!((copied.x, copied.y), (300.0, 400.0));
    assert_eq!(copied.title, "copy-source");
    assert_eq!(copied.sub_title, "copy-sub");
    assert_eq!(copied.color, "{\"fill\":\"#aabbcc\"}");
    assert!(!copied.deleted);
    assert!(copied.canvas_ref_id.is_none());
    assert!(copied.shadow_id.is_none());

    // 字段结构随副本复制且顺序保持，但字段值一律为 None。
    let copied_fields = node_field::service::get(&copied.id).unwrap();
    assert_eq!(copied_fields.len(), 2);
    assert_eq!(copied_fields[0].name, "文本");
    assert_eq!(copied_fields[0].field_type, "string:single-line");
    assert_eq!(copied_fields[0].value, None);
    assert_eq!(copied_fields[1].name, "日期");
    assert_eq!(copied_fields[1].field_type, "instant:instant");
    assert_eq!(copied_fields[1].value, None);
    // 源节点字段不受复制影响，值仍在。
    let source_fields = node_field::service::get(&copy_source.id).unwrap();
    assert_eq!(
        source_fields[0].value,
        Some("secret".to_string())
    );

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

    // == 阶段七：relocate_nodes 批量跨画布迁移节点 ==
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

    // 源画布、目标画布（在根画布下创建两个子画布，分别作为源与目标）。
    let source_canvas = canvas::service::create(&root_id, "relocate-source".to_string()).unwrap();
    let target_canvas = canvas::service::create(&root_id, "relocate-target".to_string()).unwrap();
    // 另一个目标画布（用于源==目标成功路径），并事先将其逻辑删除（用于已删除目标失败路径）。
    let same_canvas = canvas::service::create(&root_id, "relocate-same".to_string()).unwrap();
    let deleted_target_canvas = canvas::service::create(&root_id, "relocate-deleted-target".to_string()).unwrap();
    canvas::service::logical_delete(&deleted_target_canvas.id).unwrap();
    // 源画布内准备夹具：画布节点（引用子画布 C）+ 一个普通节点 + 普通节点→画布节点的边。
    // 该边在子画布 C 内自动生成影子节点（指向普通节点）。
    let canvas_node_in_source = node::service::create(
        &source_canvas.id,
        "relocate-canvas-node".to_string(),
        String::new(),
        10.0,
        20.0,
        None,
        true,
    )
    .unwrap();
    let child_canvas = canvas_node_in_source.canvas_ref_id.clone().unwrap();
    let normal_node = node::service::create(
        &source_canvas.id,
        "relocate-normal".to_string(),
        "sub".to_string(),
        30.0,
        40.0,
        None,
        false,
    )
    .unwrap();
    edge::service::create(
        &source_canvas.id,
        &normal_node.id,
        "right".to_string(),
        &canvas_node_in_source.id,
        "left".to_string(),
        false,
    )
    .unwrap();
    // 在子画布 C 内查到的影子节点（指向 normal_node）。
    // 新机制下影子 shadow_id 指向产生边，不再指向原始节点；此处改用 shadow_origin_id
    // （沿影子链解析到的根本体 id）匹配原始节点 id。
    let shadow_node = node::service::list(&child_canvas, false)
        .unwrap()
        .into_iter()
        .find(|n| n.shadow_origin_id.as_deref() == Some(normal_node.id.as_str()))
        .unwrap();

    // relocate_nodes 失败路径：目标画布不存在时报 NoCanvasWithSuchId。
    assert!(matches!(
        node::service::relocate_nodes(
            &[node::vo::MoveNodeVO {
                id: normal_node.id.clone(),
                x: 1.0,
                y: 2.0,
            }],
            &uuid::Uuid::new_v4().to_string(),
        ),
        Err(ErrorCode::NoCanvasWithSuchId { .. })
    ));

    // relocate_nodes 失败路径：目标画布已逻辑删除时报 NoCanvasWithSuchId。
    assert!(matches!(
        node::service::relocate_nodes(
            &[node::vo::MoveNodeVO {
                id: normal_node.id.clone(),
                x: 1.0,
                y: 2.0,
            }],
            &deleted_target_canvas.id,
        ),
        Err(ErrorCode::NoCanvasWithSuchId { .. })
    ));

    // relocate_nodes 失败路径：节点不存在时报 NoNodeWithSuchId。
    assert!(matches!(
        node::service::relocate_nodes(
            &[node::vo::MoveNodeVO {
                id: uuid::Uuid::new_v4().to_string(),
                x: 1.0,
                y: 2.0,
            }],
            &target_canvas.id,
        ),
        Err(ErrorCode::NoNodeWithSuchId { .. })
    ));

    // relocate_nodes 失败路径：items 含画布节点时报 NodeIsCanvasNode。
    assert!(matches!(
        node::service::relocate_nodes(
            &[node::vo::MoveNodeVO {
                id: canvas_node_in_source.id.clone(),
                x: 1.0,
                y: 2.0,
            }],
            &target_canvas.id,
        ),
        Err(ErrorCode::NodeIsCanvasNode)
    ));

    // relocate_nodes 失败路径：items 含影子节点（在子画布内查到的）时报 NodeIsShadow。
    assert!(matches!(
        node::service::relocate_nodes(
            &[node::vo::MoveNodeVO {
                id: shadow_node.id.clone(),
                x: 1.0,
                y: 2.0,
            }],
            &target_canvas.id,
        ),
        Err(ErrorCode::NodeIsShadow)
    ));

    // relocate_nodes 失败路径：items 只含该普通节点（它与画布节点之间有边即外部边）时报 NodeSetHasExternalEdges。
    assert!(matches!(
        node::service::relocate_nodes(
            &[node::vo::MoveNodeVO {
                id: normal_node.id.clone(),
                x: 1.0,
                y: 2.0,
            }],
            &target_canvas.id,
        ),
        Err(ErrorCode::NodeSetHasExternalEdges)
    ));

    // relocate_nodes 失败路径：两个节点分属两个画布时报 NodeNotInSameCanvas。
    let in_target_node = node::service::create(
        &target_canvas.id,
        "relocate-other-canvas".to_string(),
        "sub".to_string(),
        0.0,
        0.0,
        None,
        false,
    )
    .unwrap();
    assert!(matches!(
        node::service::relocate_nodes(
            &[
                node::vo::MoveNodeVO {
                    id: normal_node.id.clone(),
                    x: 1.0,
                    y: 2.0,
                },
                node::vo::MoveNodeVO {
                    id: in_target_node.id.clone(),
                    x: 1.0,
                    y: 2.0,
                },
            ],
            &same_canvas.id,
        ),
        Err(ErrorCode::NodeNotInSameCanvas)
    ));

    // relocate_nodes 成功路径：源画布 == 目标画布，无操作且不产日志。
    // 在 same_canvas 内新建一个普通节点（不带任何外部边）。
    let same_canvas_node = node::service::create(
        &same_canvas.id,
        "relocate-same-canvas-node".to_string(),
        "sub".to_string(),
        50.0,
        60.0,
        None,
        false,
    )
    .unwrap();
    let log_total_before_same = log::service::list(0, 1).unwrap().total;
    node::service::relocate_nodes(
        &[node::vo::MoveNodeVO {
            id: same_canvas_node.id.clone(),
            x: 999.0,
            y: 999.0,
        }],
        &same_canvas.id,
    )
    .unwrap();
    // 日志数未变。
    assert_eq!(
        log::service::list(0, 1).unwrap().total,
        log_total_before_same
    );
    // 坐标未被更新（仍是创建时的 50.0, 60.0）。
    let unchanged = node::service::list(&same_canvas.id, false)
        .unwrap()
        .into_iter()
        .find(|n| n.id == same_canvas_node.id)
        .unwrap();
    assert_eq!((unchanged.x, unchanged.y), (50.0, 60.0));

    // relocate_nodes 成功路径：空列表直接返回 Ok 且不产日志。
    let log_total_before_empty = log::service::list(0, 1).unwrap().total;
    node::service::relocate_nodes(&[], &target_canvas.id).unwrap();
    assert_eq!(
        log::service::list(0, 1).unwrap().total,
        log_total_before_empty
    );

    // relocate_nodes 成功路径：源画布中两个普通节点 + 它们之间一条边，迁移到目标画布。
    // 先在源画布内新建一对普通节点并建边。
    let reloc_a = node::service::create(
        &source_canvas.id,
        "reloc-a".to_string(),
        "sub-a".to_string(),
        100.0,
        200.0,
        None,
        false,
    )
    .unwrap();
    let reloc_b = node::service::create(
        &source_canvas.id,
        "reloc-b".to_string(),
        "sub-b".to_string(),
        300.0,
        400.0,
        None,
        false,
    )
    .unwrap();
    let reloc_edge = edge::service::create(
        &source_canvas.id,
        &reloc_a.id,
        "right".to_string(),
        &reloc_b.id,
        "left".to_string(),
        false,
    )
    .unwrap();
    let log_total_before = log::service::list(0, 1).unwrap().total;
    node::service::relocate_nodes(
        &[
            node::vo::MoveNodeVO {
                id: reloc_a.id.clone(),
                x: 111.0,
                y: 222.0,
            },
            node::vo::MoveNodeVO {
                id: reloc_b.id.clone(),
                x: 333.0,
                y: 444.0,
            },
        ],
        &target_canvas.id,
    )
    .unwrap();
    // 节点坐标与画布归属已落库为传入值。
    let after_a = node::service::list(&target_canvas.id, false)
        .unwrap()
        .into_iter()
        .find(|n| n.id == reloc_a.id)
        .unwrap();
    assert_eq!(after_a.canvas_id, target_canvas.id);
    assert_eq!((after_a.x, after_a.y), (111.0, 222.0));
    let after_b = node::service::list(&target_canvas.id, false)
        .unwrap()
        .into_iter()
        .find(|n| n.id == reloc_b.id)
        .unwrap();
    assert_eq!(after_b.canvas_id, target_canvas.id);
    assert_eq!((after_b.x, after_b.y), (333.0, 444.0));
    // 源画布不再含这两个节点（其它夹具节点仍在）。
    assert!(node::service::list(&source_canvas.id, false)
        .unwrap()
        .iter()
        .all(|n| n.id != reloc_a.id && n.id != reloc_b.id));
    // 目标画布含这两个节点。
    let target_nodes = node::service::list(&target_canvas.id, false).unwrap();
    assert!(target_nodes.iter().any(|n| n.id == reloc_a.id));
    assert!(target_nodes.iter().any(|n| n.id == reloc_b.id));
    // 边的 canvas_id 随迁到目标画布。
    assert!(edge::service::list(&source_canvas.id)
        .unwrap()
        .iter()
        .all(|e| e.id != reloc_edge.id));
    let target_edges = edge::service::list(&target_canvas.id).unwrap();
    assert!(target_edges.iter().any(|e| e.id == reloc_edge.id
        && e.canvas_id == target_canvas.id));
    // 产生恰好一条 Action::NodeRelocate 日志，且 node_count=2、source/target 画布名称正确、object_id 为目标画布 id。
    let logs_after = log::service::list(0, 1000).unwrap();
    assert_eq!(logs_after.total, log_total_before + 1);
    let relocate_log = logs_after
        .items
        .iter()
        .find(|e| matches!(e.action, entity::Action::NodeRelocate { .. }))
        .unwrap();
    assert_eq!(relocate_log.object_id, target_canvas.id);
    assert!(matches!(
        &relocate_log.action,
        entity::Action::NodeRelocate {
            node_count,
            source_canvas_name,
            target_canvas_name,
        } if *node_count == 2
            && source_canvas_name == "relocate-source"
            && target_canvas_name == "relocate-target"
    ));

    lifecycle::service::save().unwrap();
    lifecycle::service::close().unwrap();
    }

    test::cleanup(&path);
}

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
    let shadow_x = node::dao::select_by_producing_edge_id(&connection, &edge_xb.id).unwrap().unwrap();
    let shadow_z = node::dao::select_by_producing_edge_id(&connection, &edge_bz.id).unwrap().unwrap();
    drop(connection);

    // list 合并成功路径：影子的 title 合并自原始节点，shadow_id 指向产生边；
    // shadow_direction 由产生边源端节点类型决定：X 普通节点 → Inflow，B 画布节点 → Outflow。
    let nodes_b = node::service::list(&canvas_b, false).unwrap();
    let vo_x = nodes_b.iter().find(|n| n.id == shadow_x.id).unwrap();
    let vo_z = nodes_b.iter().find(|n| n.id == shadow_z.id).unwrap();
    assert_eq!(vo_x.title, "origin-x");
    assert_eq!(vo_x.shadow_id.as_deref(), Some(edge_xb.id.as_str()));
    assert_eq!(vo_x.shadow_direction, Some(node::vo::ShadowDirection::Inflow));
    assert!(vo_x.canvas_ref_id.is_none());
    assert_eq!(vo_z.title, "canvas-z");
    assert_eq!(vo_z.shadow_id.as_deref(), Some(edge_bz.id.as_str()));
    assert_eq!(vo_z.shadow_direction, Some(node::vo::ShadowDirection::Outflow));

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
    node::service::move_node(&shadow_x.id, 10.0, 20.0).unwrap();
    let moved = node::service::list(&canvas_b, false)
        .unwrap()
        .into_iter()
        .find(|n| n.id == shadow_x.id)
        .unwrap();
    assert_eq!((moved.x, moved.y), (10.0, 20.0));

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
        node::dao::select_by_producing_edge_id(&connection, edge_id).unwrap()
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
        Some(node::vo::ShadowDirection::Outflow)
    );

    // list 视图断言：影子展示数据合并自根本体节点且方向正确。
    let nodes_b = node::service::list(&canvas_b, false).unwrap();
    let vo_z = nodes_b.iter().find(|n| n.id == shadow_z.id).unwrap();
    assert_eq!(vo_z.shadow_direction, Some(node::vo::ShadowDirection::Outflow));

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
    assert_eq!(vo_nested_x.shadow_direction, Some(node::vo::ShadowDirection::Inflow));
    assert_eq!(vo_nested_x.shadow_origin_deleted, Some(false));
    let vo_nested_z = nodes_c2.iter().find(|n| n.id == nested_shadow_z.id).unwrap();
    assert_eq!(vo_nested_z.title, "canvas-z");
    assert_eq!(vo_nested_z.shadow_direction, Some(node::vo::ShadowDirection::Outflow));

    // 影子参与连线成功路径：入向影子作为源连接普通节点、普通节点连接出向影子。
    edge::service::create(&canvas_b, &shadow_x.id, "right".to_string(), &node_n.id, "left".to_string(), false).unwrap();
    edge::service::create(&canvas_b, &node_n.id, "right".to_string(), &shadow_z.id, "left".to_string(), false).unwrap();
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
        node::dao::select_by_producing_edge_id(&connection, edge_id).unwrap()
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
        node::dao::select_by_producing_edge_id(&connection, edge_id).unwrap()
    };

    // ===== 第 1 阶段：同向重建（无影子），仅更新连接桩，id 不变且 title/description 保留，不记日志 =====
    // 准备根画布内两个普通节点 A、B，建边 A→B。
    let node_a = node::service::create(&root.id, "title-a".to_string(), "sub-a".to_string(), 0.0, 0.0, None, false).unwrap();
    let node_b = node::service::create(&root.id, "title-b".to_string(), "sub-b".to_string(), 200.0, 0.0, None, false).unwrap();
    let edge_ab = edge::service::create(&root.id, &node_a.id, "right".to_string(), &node_b.id, "left".to_string(), false).unwrap();
    // 写入标题与详情，便于断言保留。
    edge::service::update(&edge_ab.id, "inherited title".to_string(), "inherited desc".to_string()).unwrap();
    // 同向用不同连接桩重建 → 触发端口更新路径。
    let log_total_before_port = log::service::list(0, 1).unwrap().total;
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
    assert_eq!(log::service::list(0, 1).unwrap().total, log_total_before_port);
    // 幂等用例：连接桩完全相同的重复拖线也成功，且日志总数仍不变。
    let edge_ab_idempotent = edge::service::create(&root.id, &node_a.id, "top".to_string(), &node_b.id, "bottom".to_string(), false).unwrap();
    assert_eq!(edge_ab_idempotent.id, edge_ab.id);
    assert_eq!(edge_ab_idempotent.source_port, "top");
    assert_eq!(edge_ab_idempotent.target_port, "bottom");
    assert_eq!(log::service::list(0, 1).unwrap().total, log_total_before_port);
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
    let replace_log = log::service::list(0, 1000).unwrap()
        .items
        .into_iter()
        .find(|e| e.object_id == edge_reversed.id
            && matches!(e.action, entity::Action::EdgeReplace { .. }))
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
    assert_eq!(shadow_b_vo.shadow_direction, Some(node::vo::ShadowDirection::Outflow));

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
    assert_eq!(shadow_a_in_b_vo.shadow_direction, Some(node::vo::ShadowDirection::Outflow));

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
    assert_eq!(still_outflow.shadow_direction, Some(node::vo::ShadowDirection::Outflow));

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
    assert_eq!(vo_x_bc.shadow_direction, Some(node::vo::ShadowDirection::Inflow));

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
