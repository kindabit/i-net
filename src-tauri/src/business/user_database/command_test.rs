use super::*;
use crate::business::metadata;
use crate::business::user_database::entity::Dictionary;
use crate::business::user_database::field_type::FieldValue;
use crate::business::user_database::node_field::vo::NodeFieldVO;
use crate::business::user_database::template::vo::TemplateFieldVO;
use crate::error_code::ErrorCode;
use crate::test;

/// 模拟用户一次完整操作会话：注册数据库后，依次经历
/// 生命周期与画布/节点/边/注册表/视口/日志、node_field 与 dictionary、template、
/// attachment 四个阶段的操作，覆盖 user_database 模块所有子业务模块的所有 preprocess 函数的
/// 成功与失败路径；数据库随会话推进多次打开与关闭。
#[test]
fn test_user_database_command_all_functions() {
    let _guard = test::acquire_test_lock();

    // 每个测试都在自己的数据目录下进行，初始化自己的数据目录和 metadata 数据库。
    let path = test::create_test_path();
    crate::state::set_path(path.clone());
    metadata::service::initialize().unwrap();
    let registered = metadata::service::register("db-1".to_string()).unwrap();

    // == 阶段一：lifecycle / canvas / node / edge / registry / viewport / log ==
    {

    use lifecycle::command::{
        user_database_lifecycle_close, user_database_lifecycle_initialize,
        user_database_lifecycle_save,
    };

    // user_database_lifecycle_initialize::preprocess 失败路径：id 不是合法的 uuid 格式时报 InvalidUserDatabaseId。
    assert!(matches!(
        user_database_lifecycle_initialize::preprocess(
            "no-such-id".to_string(),
            "password".to_string()
        ),
        Err(ErrorCode::InvalidUserDatabaseId { .. })
    ));

    // user_database_lifecycle_initialize::preprocess 失败路径：密码为空时报 EmptyPassword。
    assert!(matches!(
        user_database_lifecycle_initialize::preprocess(registered.id.clone(), "".to_string()),
        Err(ErrorCode::EmptyPassword)
    ));

    // user_database_lifecycle_initialize::preprocess 失败路径：id 格式合法但不存在时报 NoDatabaseWithSuchId。
    assert!(matches!(
        user_database_lifecycle_initialize::preprocess(
            uuid::Uuid::new_v4().to_string(),
            "password".to_string()
        ),
        Err(ErrorCode::NoDatabaseWithSuchId { .. })
    ));

    // user_database_lifecycle_initialize::preprocess 成功路径：打开数据库后 state 处于打开状态。
    let opened = user_database_lifecycle_initialize::preprocess(
        registered.id.clone(),
        "password".to_string(),
    )
    .unwrap();
    assert_eq!(opened.id, registered.id);
    assert!(state::is_open());

    use canvas::command::{
        user_database_canvas_create, user_database_canvas_list,
        user_database_canvas_logical_delete, user_database_canvas_move_canvas,
        user_database_canvas_physical_delete, user_database_canvas_rename,
        user_database_canvas_restore,
    };

    // user_database_canvas_list::preprocess 成功路径：初始只有根画布。
    let canvases = user_database_canvas_list::preprocess(false).unwrap();
    assert_eq!(canvases.len(), 1);
    let root_id = canvases[0].id.clone();

    // user_database_canvas_create::preprocess 失败路径：父画布 id 非法时报 InvalidCanvasId。
    assert!(matches!(
        user_database_canvas_create::preprocess("no-such-id".to_string(), "c".to_string()),
        Err(ErrorCode::InvalidCanvasId { .. })
    ));

    // user_database_canvas_create::preprocess 失败路径：名称为空时报 EmptyCanvasName。
    assert!(matches!(
        user_database_canvas_create::preprocess(root_id.clone(), "  ".to_string()),
        Err(ErrorCode::EmptyCanvasName)
    ));

    // user_database_canvas_create::preprocess 成功路径：名称两侧空白字符被裁剪。
    let child =
        user_database_canvas_create::preprocess(root_id.clone(), " child ".to_string()).unwrap();
    assert_eq!(child.name, "child");

    // user_database_canvas_move_canvas::preprocess 失败路径：id 非法时报 InvalidCanvasId。
    assert!(matches!(
        user_database_canvas_move_canvas::preprocess("no-such-id".to_string(), 0.0, 0.0),
        Err(ErrorCode::InvalidCanvasId { .. })
    ));

    // user_database_canvas_move_canvas::preprocess 成功路径。
    user_database_canvas_move_canvas::preprocess(child.id.clone(), 100.0, 200.0).unwrap();

    // user_database_canvas_rename::preprocess 失败路径：id 非法时报 InvalidCanvasId，名称为空时报 EmptyCanvasName。
    assert!(matches!(
        user_database_canvas_rename::preprocess("no-such-id".to_string(), "x".to_string()),
        Err(ErrorCode::InvalidCanvasId { .. })
    ));
    assert!(matches!(
        user_database_canvas_rename::preprocess(child.id.clone(), " ".to_string()),
        Err(ErrorCode::EmptyCanvasName)
    ));

    // user_database_canvas_rename::preprocess 成功路径。
    user_database_canvas_rename::preprocess(child.id.clone(), "child-renamed".to_string()).unwrap();

    // user_database_canvas_logical_delete::preprocess 失败路径：id 非法时报 InvalidCanvasId。
    assert!(matches!(
        user_database_canvas_logical_delete::preprocess("no-such-id".to_string()),
        Err(ErrorCode::InvalidCanvasId { .. })
    ));

    // user_database_canvas_logical_delete::preprocess 失败路径：目标是根画布时报 RootCanvasCannotBeDeleted。
    assert!(matches!(
        user_database_canvas_logical_delete::preprocess(root_id.clone()),
        Err(ErrorCode::RootCanvasCannotBeDeleted)
    ));

    // user_database_canvas_logical_delete / restore::preprocess 成功路径：删除后再恢复。
    user_database_canvas_logical_delete::preprocess(child.id.clone()).unwrap();
    assert!(user_database_canvas_list::preprocess(true)
        .unwrap()
        .iter()
        .any(|canvas| canvas.id == child.id));

    // user_database_canvas_restore::preprocess 失败路径：id 非法时报 InvalidCanvasId。
    assert!(matches!(
        user_database_canvas_restore::preprocess("no-such-id".to_string(), 0.0, 0.0),
        Err(ErrorCode::InvalidCanvasId { .. })
    ));
    user_database_canvas_restore::preprocess(child.id.clone(), 300.0, 400.0).unwrap();
    assert!(user_database_canvas_list::preprocess(false)
        .unwrap()
        .iter()
        .any(|canvas| canvas.id == child.id));
    assert!(user_database_canvas_list::preprocess(true)
        .unwrap()
        .iter()
        .all(|canvas| canvas.id != child.id));

    // user_database_canvas_physical_delete::preprocess 失败路径：id 非法时报 InvalidCanvasId。
    assert!(matches!(
        user_database_canvas_physical_delete::preprocess("no-such-id".to_string()),
        Err(ErrorCode::InvalidCanvasId { .. })
    ));

    // user_database_canvas_physical_delete::preprocess 失败路径：目标是根画布时报 RootCanvasCannotBeDeleted。
    assert!(matches!(
        user_database_canvas_physical_delete::preprocess(root_id.clone()),
        Err(ErrorCode::RootCanvasCannotBeDeleted)
    ));

    use viewport::command::{user_database_viewport_get, user_database_viewport_set};

    // user_database_viewport_get::preprocess 失败路径：Some(非法 id) 时报 InvalidCanvasId。
    assert!(matches!(
        user_database_viewport_get::preprocess(Some("no-such-id".to_string())),
        Err(ErrorCode::InvalidCanvasId { .. })
    ));

    // user_database_viewport_get::preprocess 成功路径：None 返回画布宇宙视口的默认值。
    let universe = user_database_viewport_get::preprocess(None).unwrap();
    assert_eq!(universe.canvas_id, entity::CANVAS_UNIVERSE_VIEWPORT_ID);

    // user_database_viewport_set::preprocess 失败路径：Some(非法 id) 时报 InvalidCanvasId。
    assert!(matches!(
        user_database_viewport_set::preprocess(Some("no-such-id".to_string()), 0.0, 0.0, 1.0),
        Err(ErrorCode::InvalidCanvasId { .. })
    ));

    // user_database_viewport_set / get::preprocess 成功路径：None 与 Some 两种路径往返一致。
    user_database_viewport_set::preprocess(None, 1.0, 2.0, 3.0).unwrap();
    assert_eq!(
        user_database_viewport_get::preprocess(None).unwrap().zoom,
        3.0
    );
    user_database_viewport_set::preprocess(Some(child.id.clone()), 4.0, 5.0, 6.0).unwrap();
    assert_eq!(
        user_database_viewport_get::preprocess(Some(child.id.clone()))
            .unwrap()
            .zoom,
        6.0
    );

    use node::command::{
        user_database_node_copy, user_database_node_create, user_database_node_list,
        user_database_node_logical_delete, user_database_node_modify,
        user_database_node_move_node, user_database_node_physical_delete,
        user_database_node_restore,
    };

    // user_database_node_create::preprocess 失败路径：create_canvas=false，画布 id 非法时报 InvalidCanvasId。
    assert!(matches!(
        user_database_node_create::preprocess(
            "no-such-id".to_string(),
            "t".to_string(),
            "s".to_string(),
            0.0,
            0.0,
            None,
            false,
        ),
        Err(ErrorCode::InvalidCanvasId { .. })
    ));

    // user_database_node_create::preprocess 失败路径：create_canvas=true，画布 id 非法时报 InvalidCanvasId。
    assert!(matches!(
        user_database_node_create::preprocess(
            "no-such-id".to_string(),
            "cn".to_string(),
            "".to_string(),
            0.0,
            0.0,
            None,
            true,
        ),
        Err(ErrorCode::InvalidCanvasId { .. })
    ));

    // user_database_node_create::preprocess 失败路径：create_canvas=true，名称为空白时报 EmptyCanvasName。
    assert!(matches!(
        user_database_node_create::preprocess(
            child.id.clone(),
            "   ".to_string(),
            "".to_string(),
            0.0,
            0.0,
            None,
            true,
        ),
        Err(ErrorCode::EmptyCanvasName)
    ));

    // user_database_node_create::preprocess 成功路径：create_canvas=false，标题两侧空白字符被裁剪。
    let node_1 = user_database_node_create::preprocess(
        child.id.clone(),
        " title-1 ".to_string(),
        "sub-1".to_string(),
        10.0,
        20.0,
        None,
        false,
    )
    .unwrap();
    assert_eq!(node_1.title, "title-1");
    let node_2 = user_database_node_create::preprocess(
        child.id.clone(),
        "title-2".to_string(),
        "sub-2".to_string(),
        100.0,
        200.0,
        None,
        false,
    )
    .unwrap();

    // user_database_node_create::preprocess 成功路径：create_canvas=true，创建画布节点并返回 canvas_ref_id 为 Some。
    let cv_node_1 = user_database_node_create::preprocess(
        child.id.clone(),
        " cv-node ".to_string(),
        "".to_string(),
        50.0,
        60.0,
        None,
        true,
    )
    .unwrap();
    assert!(cv_node_1.canvas_ref_id.is_some());
    assert_eq!(cv_node_1.title, "cv-node");
    assert!(cv_node_1.sub_title.is_empty());
    // 交叉验证画布确实被创建
    {
        let conn = state::lock_connection();
        assert!(canvas::dao::select_by_id(
            &conn,
            cv_node_1.canvas_ref_id.as_ref().unwrap()
        )
        .unwrap()
        .is_some());
    }

    // user_database_node_create::preprocess 成功路径：create_canvas=true，重复名称时自动追加 " 2"。
    let cv_node_2 = user_database_node_create::preprocess(
        child.id.clone(),
        "cv-node".to_string(),
        "".to_string(),
        70.0,
        80.0,
        None,
        true,
    )
    .unwrap();
    assert_eq!(cv_node_2.title, "cv-node 2");
    assert!(cv_node_2.canvas_ref_id.is_some());

    // user_database_node_move_node::preprocess 失败路径：id 非法时报 InvalidNodeId。
    assert!(matches!(
        user_database_node_move_node::preprocess("no-such-id".to_string(), 0.0, 0.0),
        Err(ErrorCode::InvalidNodeId { .. })
    ));

    // user_database_node_move_node::preprocess 成功路径。
    user_database_node_move_node::preprocess(node_1.id.clone(), 30.0, 40.0).unwrap();

    // user_database_node_modify::preprocess 失败路径：id 非法时报 InvalidNodeId。
    assert!(matches!(
        user_database_node_modify::preprocess(
            "no-such-id".to_string(),
            "t".to_string(),
            "s".to_string()
        ),
        Err(ErrorCode::InvalidNodeId { .. })
    ));

    // user_database_node_modify::preprocess 成功路径。
    user_database_node_modify::preprocess(
        node_1.id.clone(),
        "title-1-new".to_string(),
        "sub-1-new".to_string(),
    )
    .unwrap();

    // user_database_node_logical_delete / restore::preprocess 失败路径：id 非法时报 InvalidNodeId。
    assert!(matches!(
        user_database_node_logical_delete::preprocess("no-such-id".to_string()),
        Err(ErrorCode::InvalidNodeId { .. })
    ));
    assert!(matches!(
        user_database_node_restore::preprocess("no-such-id".to_string(), 0.0, 0.0),
        Err(ErrorCode::InvalidNodeId { .. })
    ));

    // user_database_node_logical_delete / restore / list::preprocess 成功路径：list 按 deleted 分流。
    // 同时校验返回值：返回的 Node 对象 id 匹配、deleted == true。
    let deleted_node_1 = user_database_node_logical_delete::preprocess(node_1.id.clone()).unwrap();
    assert_eq!(deleted_node_1.id, node_1.id);
    assert!(deleted_node_1.deleted);
    assert_eq!(deleted_node_1.title, "title-1-new");
    assert_eq!(
        user_database_node_list::preprocess(child.id.clone(), true)
            .unwrap()
            .len(),
        1
    );
    user_database_node_restore::preprocess(node_1.id.clone(), 50.0, 60.0).unwrap();
    assert_eq!(
        user_database_node_list::preprocess(child.id.clone(), false)
            .unwrap()
            .len(),
        4
    );

    // user_database_node_copy::preprocess 失败路径：id 非法时报 InvalidNodeId。
    assert!(matches!(
        user_database_node_copy::preprocess("no-such-id".to_string(), 0.0, 0.0),
        Err(ErrorCode::InvalidNodeId { .. })
    ));

    // user_database_node_copy::preprocess 成功路径：副本继承标题与副标题，id 全新、坐标取入参。
    let copied_node_2 =
        user_database_node_copy::preprocess(node_2.id.clone(), 500.0, 600.0).unwrap();
    assert_ne!(copied_node_2.id, node_2.id);
    assert_eq!(copied_node_2.title, "title-2");
    assert_eq!(copied_node_2.sub_title, "sub-2");
    assert_eq!((copied_node_2.x, copied_node_2.y), (500.0, 600.0));
    assert!(copied_node_2.canvas_ref_id.is_none());
    assert!(copied_node_2.shadow_id.is_none());

    // user_database_node_list::preprocess 失败路径：画布 id 非法时报 InvalidCanvasId。
    assert!(matches!(
        user_database_node_list::preprocess("no-such-id".to_string(), false),
        Err(ErrorCode::InvalidCanvasId { .. })
    ));

    use registry::command::{user_database_registry_get, user_database_registry_set};

    // user_database_registry_set::preprocess 失败路径：name 为空时报 EmptyRegistryName。
    assert!(matches!(
        user_database_registry_set::preprocess("".to_string(), "v".to_string()),
        Err(ErrorCode::EmptyRegistryName)
    ));
    assert!(matches!(
        user_database_registry_set::preprocess("  ".to_string(), "v".to_string()),
        Err(ErrorCode::EmptyRegistryName)
    ));

    // user_database_registry_get::preprocess 失败路径：name 为空时报 EmptyRegistryName。
    assert!(matches!(
        user_database_registry_get::preprocess("".to_string()),
        Err(ErrorCode::EmptyRegistryName)
    ));

    // user_database_registry_set / get::preprocess 成功路径：写入后读出相同值；name 两侧空白被裁剪。
    user_database_registry_set::preprocess(" lastScene ".to_string(), "dark".to_string()).unwrap();
    assert_eq!(
        user_database_registry_get::preprocess(" lastScene ".to_string()).unwrap(),
        Some("dark".to_string())
    );

    use edge::command::{
        user_database_edge_create, user_database_edge_delete, user_database_edge_list,
        user_database_edge_update,
    };

    // user_database_edge_create::preprocess 失败路径：画布 id 非法时报 InvalidCanvasId。
    assert!(matches!(
        user_database_edge_create::preprocess(
            "no-such-id".to_string(),
            node_1.id.clone(),
            "right".to_string(),
            node_2.id.clone(),
            "left".to_string(),
            false
        ),
        Err(ErrorCode::InvalidCanvasId { .. })
    ));

    // user_database_edge_create::preprocess 失败路径：节点 id 非法时报 InvalidNodeId。
    assert!(matches!(
        user_database_edge_create::preprocess(
            child.id.clone(),
            "no-such-id".to_string(),
            "right".to_string(),
            node_2.id.clone(),
            "left".to_string(),
            false
        ),
        Err(ErrorCode::InvalidNodeId { .. })
    ));

    // user_database_edge_create::preprocess 失败路径：连接桩非法时报 InvalidNodePort。
    assert!(matches!(
        user_database_edge_create::preprocess(
            child.id.clone(),
            node_1.id.clone(),
            "middle".to_string(),
            node_2.id.clone(),
            "left".to_string(),
            false
        ),
        Err(ErrorCode::InvalidNodePort { .. })
    ));

    // user_database_edge_create::preprocess 成功路径：连接桩两侧空白字符被裁剪。
    let edge_1 = user_database_edge_create::preprocess(
        child.id.clone(),
        node_1.id.clone(),
        " right ".to_string(),
        node_2.id.clone(),
        "left".to_string(),
        false,
    )
    .unwrap();
    assert_eq!(edge_1.source_port, "right");

    // user_database_edge_create::preprocess 失败路径：两端连接桩相同时报 EdgeSameNodePort
    // （preprocess 不再吃掉端口字符串，业务层拒收）。
    assert!(matches!(
        user_database_edge_create::preprocess(
            child.id.clone(),
            node_1.id.clone(),
            "top".to_string(),
            node_2.id.clone(),
            "top".to_string(),
            false
        ),
        Err(ErrorCode::EdgeSameNodePort)
    ));

    // user_database_edge_update::preprocess 失败路径：id 非法时报 InvalidEdgeId。
    assert!(matches!(
        user_database_edge_update::preprocess(
            "no-such-id".to_string(),
            "title".to_string(),
            "desc".to_string()
        ),
        Err(ErrorCode::InvalidEdgeId { .. })
    ));

    // user_database_edge_update::preprocess 成功路径：标题和详情被更新（两侧空白字符被裁剪）。
    user_database_edge_update::preprocess(
        edge_1.id.clone(),
        " new title ".to_string(),
        " new desc ".to_string(),
    )
    .unwrap();

    let updated_edge = user_database_edge_list::preprocess(child.id.clone())
        .unwrap()
        .into_iter()
        .find(|e| e.id == edge_1.id)
        .unwrap();
    assert_eq!(updated_edge.title, "new title");
    assert_eq!(updated_edge.description, "new desc");

    // user_database_edge_delete::preprocess 失败路径：id 非法时报 InvalidEdgeId。
    assert!(matches!(
        user_database_edge_delete::preprocess("no-such-id".to_string(), false),
        Err(ErrorCode::InvalidEdgeId { .. })
    ));

    // user_database_edge_delete::preprocess 成功路径。
    user_database_edge_delete::preprocess(edge_1.id.clone(), false).unwrap();

    // user_database_edge_list::preprocess 失败路径：画布 id 非法时报 InvalidCanvasId。
    assert!(matches!(
        user_database_edge_list::preprocess("no-such-id".to_string()),
        Err(ErrorCode::InvalidCanvasId { .. })
    ));

    // user_database_edge_list::preprocess 成功路径：边已被删除，返回空列表。
    assert!(user_database_edge_list::preprocess(child.id.clone())
        .unwrap()
        .is_empty());

    // user_database_node_physical_delete::preprocess 失败路径：id 非法时报 InvalidNodeId。
    assert!(matches!(
        user_database_node_physical_delete::preprocess("no-such-id".to_string(), false),
        Err(ErrorCode::InvalidNodeId { .. })
    ));

    // user_database_node_physical_delete::preprocess 成功路径。
    user_database_node_physical_delete::preprocess(node_2.id.clone(), false).unwrap();

    use log::command::user_database_log_list;

    // user_database_log_list::preprocess 成功路径：前面各操作自动生成的日志，可被正常分页查询并解密反序列化。
    let logs = user_database_log_list::preprocess(0, 1000).unwrap();
    assert!(!logs.items.is_empty());
    // 断言至少包含本次测试中 node 创建产生的日志。
    assert!(logs.items.iter().any(|entry| entry.object_id == node_1.id));

    // user_database_canvas_physical_delete::preprocess 成功路径：物理删除子画布后列表只剩根画布。
    user_database_canvas_physical_delete::preprocess(child.id.clone()).unwrap();
    assert_eq!(
        user_database_canvas_list::preprocess(false).unwrap().len(),
        1
    );

    // user_database_lifecycle_save / close::preprocess 成功路径：保存后关闭，state 被清空。
    user_database_lifecycle_save::preprocess().unwrap();
    assert!(
        crate::util::file_system_util::try_exists(&path.user_database_file(&registered.id))
            .unwrap()
    );
    user_database_lifecycle_close::preprocess().unwrap();
    assert!(!state::is_open());

    // user_database_lifecycle_save / close::preprocess 失败路径：数据库未打开时报 UserDatabaseNotOpen。
    assert!(matches!(
        user_database_lifecycle_save::preprocess(),
        Err(ErrorCode::UserDatabaseNotOpen)
    ));
    assert!(matches!(
        user_database_lifecycle_close::preprocess(),
        Err(ErrorCode::UserDatabaseNotOpen)
    ));

    // user_database_lifecycle_initialize::preprocess 失败路径：密码错误时报 FailToDecrypt。
    assert!(matches!(
        user_database_lifecycle_initialize::preprocess(registered.id.clone(), "wrong".to_string()),
        Err(ErrorCode::FailToDecrypt { .. })
    ));
    }

    // == 阶段二：node_field 与 dictionary（重新打开同一数据库）==
    {
    lifecycle::command::user_database_lifecycle_initialize::preprocess(
        registered.id.clone(),
        "password".to_string(),
    )
    .unwrap();

    use node_field::command::{
        user_database_node_field_get, user_database_node_field_set,
    };

    // == user_database_node_field_get::preprocess 失败路径：非法 node_id → InvalidNodeId ==
    assert!(matches!(
        user_database_node_field_get::preprocess("no-such-id".to_string()),
        Err(ErrorCode::InvalidNodeId { .. })
    ));

    // == user_database_node_field_set::preprocess 失败路径：非法 node_id → InvalidNodeId ==
    assert!(matches!(
        user_database_node_field_set::preprocess(
            "no-such-id".to_string(),
            vec![]
        ),
        Err(ErrorCode::InvalidNodeId { .. })
    ));

    // == user_database_node_field_set::preprocess 失败路径：字段名 trim 后为空 → EmptyNodeFieldName ==
    assert!(matches!(
        user_database_node_field_set::preprocess(
            uuid::Uuid::new_v4().to_string(),
            vec![NodeFieldVO {
                name: "  ".to_string(),
                field_type: "TextSingleLine".to_string(),
                type_config: None,
                value: FieldValue::String(Some("v".to_string())),
                dictionary_id: None,
            }]
        ),
        Err(ErrorCode::EmptyNodeFieldName)
    ));

    // == user_database_node_field_set::preprocess 失败路径：dictionary_id 非法 uuid → InvalidDictionaryId ==
    assert!(matches!(
        user_database_node_field_set::preprocess(
            uuid::Uuid::new_v4().to_string(),
            vec![NodeFieldVO {
                name: "f".to_string(),
                field_type: "TextSingleLine".to_string(),
                type_config: None,
                value: FieldValue::String(Some("v".to_string())),
                dictionary_id: Some("not-a-uuid".to_string()),
            }]
        ),
        Err(ErrorCode::InvalidDictionaryId { .. })
    ));

    use dictionary::command::{
        user_database_dictionary_list, user_database_dictionary_set,
    };

    // == user_database_dictionary_list::preprocess 成功路径：空列表 ==
    assert!(user_database_dictionary_list::preprocess()
        .unwrap()
        .is_empty());

    // == user_database_dictionary_set::preprocess 失败路径：id 非法 uuid → InvalidDictionaryId ==
    assert!(matches!(
        user_database_dictionary_set::preprocess(vec![Dictionary {
            id: "not-a-uuid".to_string(),
            parent_id: None,
            value: "val".to_string(),
            order: 1,
        }]),
        Err(ErrorCode::InvalidDictionaryId { .. })
    ));

    // == user_database_dictionary_set::preprocess 失败路径：parent_id 非法 uuid → InvalidDictionaryId ==
    assert!(matches!(
        user_database_dictionary_set::preprocess(vec![Dictionary {
            id: uuid::Uuid::new_v4().to_string(),
            parent_id: Some("not-a-uuid".to_string()),
            value: "val".to_string(),
            order: 1,
        }]),
        Err(ErrorCode::InvalidDictionaryId { .. })
    ));

    // == user_database_dictionary_set::preprocess 失败路径：value trim 后为空 → EmptyDictionaryValue ==
    assert!(matches!(
        user_database_dictionary_set::preprocess(vec![Dictionary {
            id: uuid::Uuid::new_v4().to_string(),
            parent_id: None,
            value: "  ".to_string(),
            order: 1,
        }]),
        Err(ErrorCode::EmptyDictionaryValue)
    ));

    // == user_database_dictionary_set::preprocess 成功路径：trim 与 uuid 校验通过后走通到 service ==
    let dict_id = uuid::Uuid::new_v4().to_string();
    user_database_dictionary_set::preprocess(vec![Dictionary {
        id: dict_id.clone(),
        parent_id: None,
        value: " trimmed ".to_string(),
        order: 1,
    }])
    .unwrap();
    let list = user_database_dictionary_list::preprocess().unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].id, dict_id);
    assert_eq!(list[0].value, "trimmed");

    // == user_database_node_field_get / set::preprocess 成功路径：
    //    创建节点后 set 再 get，往返一致 ==
    let canvases = canvas::service::list(false).unwrap();
    let root_id = &canvases[0].id;
    let node = node::service::create(
        root_id,
        "cmd-node".to_string(),
        "sub".to_string(),
        0.0,
        0.0,
        None,
        false,
    )
    .unwrap();
    let field = NodeFieldVO {
        name: " cmd-field ".to_string(),
        field_type: "TextSingleLine".to_string(),
        type_config: None,
        value: FieldValue::String(Some("cmd-value".to_string())),
        dictionary_id: None,
    };
    user_database_node_field_set::preprocess(node.id.clone(), vec![field]).unwrap();
    let got = user_database_node_field_get::preprocess(node.id.clone()).unwrap();
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].name, "cmd-field");
    assert_eq!(
        got[0].value,
        FieldValue::String(Some("cmd-value".to_string()))
    );

    lifecycle::command::user_database_lifecycle_save::preprocess().unwrap();
    lifecycle::command::user_database_lifecycle_close::preprocess().unwrap();
    }

    // == 阶段三：template（再次重新打开同一数据库；含 node create preprocess 带模板 id 的路径）==
    {
    lifecycle::command::user_database_lifecycle_initialize::preprocess(
        registered.id.clone(),
        "password".to_string(),
    )
    .unwrap();

    use template::command::{
        user_database_template_create, user_database_template_create_from_node,
        user_database_template_delete, user_database_template_get_fields,
        user_database_template_list, user_database_template_rename,
        user_database_template_set_fields,
    };

    // == user_database_template_create::preprocess 失败路径：名称为空 → EmptyTemplateName ==
    assert!(matches!(
        user_database_template_create::preprocess("  ".to_string()),
        Err(ErrorCode::EmptyTemplateName)
    ));

    // == user_database_template_create::preprocess 成功路径 ==
    let tpl = user_database_template_create::preprocess(" 模板1 ".to_string()).unwrap();
    assert_eq!(tpl.name, "模板1");

    // == user_database_template_list::preprocess 成功路径 ==
    let all = user_database_template_list::preprocess().unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].id, tpl.id);

    // == user_database_template_create_from_node::preprocess 失败路径：非法 node_id → InvalidNodeId ==
    assert!(matches!(
        user_database_template_create_from_node::preprocess(
            "not-a-uuid".to_string(),
            "x".to_string()
        ),
        Err(ErrorCode::InvalidNodeId { .. })
    ));

    // == user_database_template_create_from_node::preprocess 失败路径：名称为空 → EmptyTemplateName ==
    assert!(matches!(
        user_database_template_create_from_node::preprocess(
            uuid::Uuid::new_v4().to_string(),
            "  ".to_string()
        ),
        Err(ErrorCode::EmptyTemplateName)
    ));

    // == user_database_template_rename::preprocess 失败路径：非法 id → InvalidTemplateId ==
    assert!(matches!(
        user_database_template_rename::preprocess(
            "not-a-uuid".to_string(),
            "x".to_string()
        ),
        Err(ErrorCode::InvalidTemplateId { .. })
    ));

    // == user_database_template_rename::preprocess 失败路径：名称为空 → EmptyTemplateName ==
    assert!(matches!(
        user_database_template_rename::preprocess(
            tpl.id.clone(),
            "  ".to_string()
        ),
        Err(ErrorCode::EmptyTemplateName)
    ));

    // == user_database_template_rename::preprocess 成功路径 ==
    user_database_template_rename::preprocess(tpl.id.clone(), "模板1-改".to_string()).unwrap();

    // == user_database_template_delete::preprocess 失败路径：非法 id → InvalidTemplateId ==
    assert!(matches!(
        user_database_template_delete::preprocess("not-a-uuid".to_string()),
        Err(ErrorCode::InvalidTemplateId { .. })
    ));

    // == user_database_template_get_fields::preprocess 失败路径：非法 id → InvalidTemplateId ==
    assert!(matches!(
        user_database_template_get_fields::preprocess("not-a-uuid".to_string()),
        Err(ErrorCode::InvalidTemplateId { .. })
    ));

    // == user_database_template_get_fields::preprocess 成功路径 ==
    let fields = user_database_template_get_fields::preprocess(tpl.id.clone()).unwrap();
    assert!(fields.is_empty());

    // == user_database_template_set_fields::preprocess 失败路径：非法 id → InvalidTemplateId ==
    assert!(matches!(
        user_database_template_set_fields::preprocess(
            "not-a-uuid".to_string(),
            vec![]
        ),
        Err(ErrorCode::InvalidTemplateId { .. })
    ));

    // == user_database_template_set_fields::preprocess 失败路径：字段名 trim 后为空 → EmptyNodeFieldName ==
    assert!(matches!(
        user_database_template_set_fields::preprocess(
            tpl.id.clone(),
            vec![TemplateFieldVO {
                name: "  ".to_string(),
                field_type: "TextSingleLine".to_string(),
                type_config: None,
                dictionary_id: None,
            }]
        ),
        Err(ErrorCode::EmptyNodeFieldName)
    ));

    // == user_database_template_set_fields::preprocess 失败路径：dictionary_id 非法 uuid → InvalidDictionaryId ==
    assert!(matches!(
        user_database_template_set_fields::preprocess(
            tpl.id.clone(),
            vec![TemplateFieldVO {
                name: "f".to_string(),
                field_type: "TextSingleLine".to_string(),
                type_config: None,
                dictionary_id: Some("not-a-uuid".to_string()),
            }]
        ),
        Err(ErrorCode::InvalidDictionaryId { .. })
    ));

    // == user_database_template_set_fields::preprocess 成功路径 ==
    user_database_template_set_fields::preprocess(
        tpl.id.clone(),
        vec![TemplateFieldVO {
            name: " f1 ".to_string(),
            field_type: "TextSingleLine".to_string(),
            type_config: None,
            dictionary_id: None,
        }],
    ).unwrap();
    let fields = user_database_template_get_fields::preprocess(tpl.id.clone()).unwrap();
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].name, "f1");

    // == user_database_template_delete::preprocess 成功路径：走通到 service 层 ==
    let tpl2 = user_database_template_create::preprocess("待删".to_string()).unwrap();
    user_database_template_delete::preprocess(tpl2.id.clone()).unwrap();
    assert!(matches!(
        user_database_template_get_fields::preprocess(tpl2.id.clone()),
        Err(ErrorCode::NoTemplateWithSuchId { .. })
    ));

    // == node create preprocess 失败路径：带非法模板 id → InvalidTemplateId ==
    use node::command::user_database_node_create;
    let canvases = canvas::service::list(false).unwrap();
    let root_id = &canvases[0].id;
    assert!(matches!(
        user_database_node_create::preprocess(
            root_id.clone(),
            "t".to_string(),
            "s".to_string(),
            0.0,
            0.0,
            Some("not-a-uuid".to_string()),
            false,
        ),
        Err(ErrorCode::InvalidTemplateId { .. })
    ));

    // == node create preprocess 成功路径：带合法模板 id ==
    let node = user_database_node_create::preprocess(
        root_id.clone(),
        "模板节点".to_string(),
        "副标题".to_string(),
        0.0,
        0.0,
        Some(tpl.id.clone()),
        false,
    ).unwrap();
    assert_eq!(node.title, "模板节点");
    // 模板字段已被复制到节点字段
    let nf = node_field::service::get(&node.id).unwrap();
    assert_eq!(nf.len(), 1);
    assert_eq!(nf[0].name, "f1");
    assert!(matches!(nf[0].value, FieldValue::String(None)));

    lifecycle::command::user_database_lifecycle_save::preprocess().unwrap();
    lifecycle::command::user_database_lifecycle_close::preprocess().unwrap();
    }

    // == 阶段四：attachment（再次重新打开同一数据库）==
    {
    lifecycle::command::user_database_lifecycle_initialize::preprocess(
        registered.id.clone(),
        "password".to_string(),
    )
    .unwrap();

    use attachment::command::{
        user_database_attachment_export, user_database_attachment_import,
        user_database_attachment_list, user_database_attachment_list_orphan_files,
        user_database_attachment_load, user_database_attachment_logical_delete,
        user_database_attachment_physical_delete, user_database_attachment_remove_orphan_file,
        user_database_attachment_restore,
    };

    // 造一个节点与已知字节内容的源文件。
    let canvases = canvas::service::list(false).unwrap();
    let root_id = canvases[0].id.clone();
    let node = node::service::create(
        &root_id,
        "cmd-attach-node".to_string(),
        String::new(),
        0.0,
        0.0,
        None,
        false,
    )
    .unwrap();
    let source_dir = path.data_directory.join("attachment-cmd-source");
    crate::util::file_system_util::create_dir_all(&source_dir).unwrap();
    let source_file = source_dir.join("cmd.pdf");
    let source_bytes = b"cmd-attachment-bytes".to_vec();
    crate::util::file_system_util::write(&source_file, &source_bytes).unwrap();
    let source_path = source_file.to_string_lossy().to_string();

    // == user_database_attachment_import::preprocess 失败路径：node_id 非法 → InvalidNodeId ==
    assert!(matches!(
        user_database_attachment_import::preprocess("no-such-id".to_string(), source_path.clone()),
        Err(ErrorCode::InvalidNodeId { .. })
    ));

    // == user_database_attachment_import::preprocess 失败路径：source_path 为空白 → EmptyFilePath ==
    assert!(matches!(
        user_database_attachment_import::preprocess(node.id.clone(), "  ".to_string()),
        Err(ErrorCode::EmptyFilePath)
    ));

    // == user_database_attachment_import::preprocess 成功路径：导入后 VO 字段正确 ==
    let imported =
        user_database_attachment_import::preprocess(node.id.clone(), source_path.clone()).unwrap();
    assert_eq!(imported.file_name, "cmd.pdf");
    assert_eq!(imported.size, source_bytes.len() as i64);
    assert!(!imported.missing_file);

    // == user_database_attachment_list::preprocess 失败路径：node_id 非法 → InvalidNodeId ==
    assert!(matches!(
        user_database_attachment_list::preprocess("no-such-id".to_string(), false),
        Err(ErrorCode::InvalidNodeId { .. })
    ));

    // == user_database_attachment_list::preprocess 成功路径：列表含刚导入的附件 ==
    let list = user_database_attachment_list::preprocess(node.id.clone(), false).unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].id, imported.id);

    // == user_database_attachment_load::preprocess 失败路径：id 非法 → InvalidAttachmentId ==
    assert!(matches!(
        user_database_attachment_load::preprocess("no-such-id".to_string()),
        Err(ErrorCode::InvalidAttachmentId { .. })
    ));

    // == user_database_attachment_load::preprocess 成功路径：返回明文与源字节一致 ==
    let loaded = user_database_attachment_load::preprocess(imported.id.clone()).unwrap();
    assert_eq!(loaded, source_bytes);

    // == user_database_attachment_export::preprocess 失败路径：id 非法 → InvalidAttachmentId ==
    assert!(matches!(
        user_database_attachment_export::preprocess("no-such-id".to_string(), "x".to_string()),
        Err(ErrorCode::InvalidAttachmentId { .. })
    ));

    // == user_database_attachment_export::preprocess 失败路径：target_path 为空白 → EmptyFilePath ==
    assert!(matches!(
        user_database_attachment_export::preprocess(imported.id.clone(), " ".to_string()),
        Err(ErrorCode::EmptyFilePath)
    ));

    // == user_database_attachment_export::preprocess 失败路径：
    //    target_path 指向数据目录内的用户数据库文件 → InvalidExportTargetPath ==
    assert!(matches!(
        user_database_attachment_export::preprocess(
            imported.id.clone(),
            path.user_database_file(&registered.id)
                .to_string_lossy()
                .to_string(),
        ),
        Err(ErrorCode::InvalidExportTargetPath { .. })
    ));

    // == user_database_attachment_export::preprocess 失败路径：
    //    target_path 经 `..` 穿透后仍落在数据目录内 → InvalidExportTargetPath ==
    let traversal_target = path
        .data_directory
        .join("..")
        .join(
            path.data_directory
                .file_name()
                .expect("test data directory has no file name"),
        )
        .join("user_database_set")
        .join(&registered.id)
        .join("user_database.sqlite");
    assert!(matches!(
        user_database_attachment_export::preprocess(
            imported.id.clone(),
            traversal_target.to_string_lossy().to_string(),
        ),
        Err(ErrorCode::InvalidExportTargetPath { .. })
    ));

    // == user_database_attachment_export::preprocess 成功路径：
    //    目标在数据目录外（target/test-i-net-data 下）不被防呆拦截，导出文件与源字节一致 ==
    let outside_dir = path
        .data_directory
        .parent()
        .expect("test data directory has no parent")
        .join("attachment-cmd-export-outside");
    crate::util::file_system_util::create_dir_all(&outside_dir).unwrap();
    let export_file = outside_dir.join("cmd-exported.pdf");
    user_database_attachment_export::preprocess(
        imported.id.clone(),
        export_file.to_string_lossy().to_string(),
    )
    .unwrap();
    assert_eq!(
        crate::util::file_system_util::read(&export_file).unwrap(),
        source_bytes
    );
    // 数据目录外的导出文件不受 test::cleanup 管辖，手动清理。
    let _ = std::fs::remove_dir_all(&outside_dir);

    // == user_database_attachment_logical_delete / restore / physical_delete::preprocess
    //    失败路径：id 非法 → InvalidAttachmentId ==
    assert!(matches!(
        user_database_attachment_logical_delete::preprocess("no-such-id".to_string()),
        Err(ErrorCode::InvalidAttachmentId { .. })
    ));
    assert!(matches!(
        user_database_attachment_restore::preprocess("no-such-id".to_string()),
        Err(ErrorCode::InvalidAttachmentId { .. })
    ));
    assert!(matches!(
        user_database_attachment_physical_delete::preprocess("no-such-id".to_string()),
        Err(ErrorCode::InvalidAttachmentId { .. })
    ));

    // == user_database_attachment_logical_delete / restore::preprocess 成功路径：
    //    删除后回收站可见，恢复后回到正常列表 ==
    user_database_attachment_logical_delete::preprocess(imported.id.clone()).unwrap();
    assert_eq!(
        user_database_attachment_list::preprocess(node.id.clone(), true)
            .unwrap()
            .len(),
        1
    );
    user_database_attachment_restore::preprocess(imported.id.clone()).unwrap();
    assert_eq!(
        user_database_attachment_list::preprocess(node.id.clone(), false)
            .unwrap()
            .len(),
        1
    );

    // == user_database_attachment_physical_delete::preprocess 成功路径：附件从列表消失 ==
    user_database_attachment_physical_delete::preprocess(imported.id.clone()).unwrap();
    assert!(user_database_attachment_list::preprocess(node.id.clone(), false)
        .unwrap()
        .is_empty());

    // == user_database_attachment_list_orphan_files::preprocess 成功路径：手放的孤儿文件被上报 ==
    let orphan_id = uuid::Uuid::new_v4().to_string();
    let orphan_file = path.user_attachment_file(&registered.id, &orphan_id);
    crate::util::file_system_util::write(&orphan_file, b"orphan").unwrap();
    let orphans = user_database_attachment_list_orphan_files::preprocess().unwrap();
    assert!(orphans.contains(&orphan_id));

    // == user_database_attachment_remove_orphan_file::preprocess 失败路径：
    //    id 非法（路径穿越企图）→ InvalidAttachmentId ==
    assert!(matches!(
        user_database_attachment_remove_orphan_file::preprocess("../evil".to_string()),
        Err(ErrorCode::InvalidAttachmentId { .. })
    ));

    // == user_database_attachment_remove_orphan_file::preprocess 成功路径：孤儿文件消失且不再被上报 ==
    user_database_attachment_remove_orphan_file::preprocess(orphan_id.clone()).unwrap();
    assert!(!crate::util::file_system_util::try_exists(&orphan_file).unwrap());
    assert!(!user_database_attachment_list_orphan_files::preprocess()
        .unwrap()
        .contains(&orphan_id));

    lifecycle::command::user_database_lifecycle_save::preprocess().unwrap();
    lifecycle::command::user_database_lifecycle_close::preprocess().unwrap();
    }

    // == 阶段五：canvas / node set_color 与 node color_list ==
    {
    lifecycle::command::user_database_lifecycle_initialize::preprocess(
        registered.id.clone(),
        "password".to_string(),
    )
    .unwrap();

    use canvas::command::{
        user_database_canvas_color_list, user_database_canvas_create, user_database_canvas_list,
        user_database_canvas_logical_delete, user_database_canvas_set_color,
    };
    use node::command::{
        user_database_node_create, user_database_node_list, user_database_node_logical_delete,
        user_database_node_color_list, user_database_node_set_color,
    };

    // 获取根画布 id。
    let canvases = user_database_canvas_list::preprocess(false).unwrap();
    let root_id = canvases[0].id.clone();

    // == user_database_canvas_set_color::preprocess 失败路径：id 非法 → InvalidCanvasId ==
    assert!(matches!(
        user_database_canvas_set_color::preprocess("no-such-id".to_string(), "{}".to_string()),
        Err(ErrorCode::InvalidCanvasId { .. })
    ));

    // == user_database_canvas_set_color::preprocess 成功路径：设置后通过 list 确认 color 已持久化 ==
    let color_json = "{\"fill\":\"#112233\"}".to_string();
    user_database_canvas_set_color::preprocess(root_id.clone(), format!(" {color_json} ")).unwrap();
    let root_after = user_database_canvas_list::preprocess(false).unwrap().into_iter().find(|c| c.id == root_id).unwrap();
    assert_eq!(root_after.color, color_json);

    // == user_database_node_set_color::preprocess 失败路径：id 非法 → InvalidNodeId ==
    assert!(matches!(
        user_database_node_set_color::preprocess("no-such-id".to_string(), "{}".to_string()),
        Err(ErrorCode::InvalidNodeId { .. })
    ));

    // == user_database_node_set_color::preprocess 成功路径：创建节点后设置颜色，验证持久化与 trim ==
    let node = user_database_node_create::preprocess(
        root_id.clone(),
        "color-node".to_string(),
        "sub".to_string(),
        0.0,
        0.0,
        None,
        false,
    )
    .unwrap();
    let node_color = "{\"fill\":\"#aabbcc\"}".to_string();
    user_database_node_set_color::preprocess(node.id.clone(), format!(" {node_color} ")).unwrap();
    let node_after = user_database_node_list::preprocess(root_id.clone(), false).unwrap().into_iter().find(|n| n.id == node.id).unwrap();
    assert_eq!(node_after.color, node_color);

    // == user_database_node_color_list::preprocess 成功路径：设置若干节点颜色（含空色、已删除节点）后返回结果符合预期 ==
    let plain_node = user_database_node_create::preprocess(
        root_id.clone(),
        "plain-node".to_string(),
        String::new(),
        0.0,
        0.0,
        None,
        false,
    )
    .unwrap();
    let deleted_colored = user_database_node_create::preprocess(
        root_id.clone(),
        "deleted-colored".to_string(),
        String::new(),
        0.0,
        0.0,
        None,
        false,
    )
    .unwrap();
    user_database_node_set_color::preprocess(deleted_colored.id.clone(), "{\"fill\":\"#0000ff\"}".to_string()).unwrap();
    user_database_node_logical_delete::preprocess(deleted_colored.id.clone()).unwrap();
    let _ = plain_node;

    let entries = user_database_node_color_list::preprocess().unwrap();
    // 只有 color-node 符合条件（未删除且 color 非空）
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].title, "color-node");
    assert_eq!(entries[0].color, node_color);

    // == user_database_canvas_color_list::preprocess 成功路径：根画布已带色；另建无色画布与已删除带色画布，验证只返回未删除带色画布 ==
    let plain_canvas = user_database_canvas_create::preprocess(root_id.clone(), "plain-canvas".to_string()).unwrap();
    let deleted_colored_canvas = user_database_canvas_create::preprocess(root_id.clone(), "deleted-colored-canvas".to_string()).unwrap();
    user_database_canvas_set_color::preprocess(deleted_colored_canvas.id.clone(), "{\"fill\":\"#00ff00\"}".to_string()).unwrap();
    user_database_canvas_logical_delete::preprocess(deleted_colored_canvas.id.clone()).unwrap();
    // 无色画布 color 保持空串，不应出现在结果中。
    let _ = plain_canvas;

    let entries = user_database_canvas_color_list::preprocess().unwrap();
    // 只有根画布符合条件（未删除且 color 非空）
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].name, entity::ROOT_CANVAS_NAME);
    assert!(entries[0].parent_id.is_none());
    assert_eq!(entries[0].color, color_json);

    lifecycle::command::user_database_lifecycle_save::preprocess().unwrap();
    lifecycle::command::user_database_lifecycle_close::preprocess().unwrap();
    }

    // == 阶段六：move_nodes / move_canvases preprocess ==
    {
    lifecycle::command::user_database_lifecycle_initialize::preprocess(
        registered.id.clone(),
        "password".to_string(),
    )
    .unwrap();

    use node::command::user_database_node_move_nodes;
    use canvas::command::user_database_canvas_move_canvases;
    use node::command::user_database_node_relocate_nodes;

    // user_database_node_move_nodes::preprocess 失败路径：列表中含非法 id 时报 InvalidNodeId。
    let invalid_items = vec![
        node::vo::MoveNodeVO {
            id: "not-a-uuid".to_string(),
            x: 1.0,
            y: 2.0,
        },
    ];
    assert!(matches!(
        user_database_node_move_nodes::preprocess(invalid_items),
        Err(ErrorCode::InvalidNodeId { .. })
    ));

    // user_database_node_move_nodes::preprocess 成功路径：空列表正常走到 service（service 对空列表返回 Ok）。
    user_database_node_move_nodes::preprocess(vec![]).unwrap();

    // user_database_node_move_nodes::preprocess 成功路径：合法 id 列表正常走到 service。
    let canvases = canvas::service::list(false).unwrap();
    let root_id = canvases[0].id.clone();
    let child = canvas::service::create(&root_id, "cmd-child".to_string()).unwrap();
    let node = node::service::create(
        &child.id,
        "cmd-batch-node".to_string(),
        "sub".to_string(),
        0.0,
        0.0,
        None,
        false,
    )
    .unwrap();
    let valid_items = vec![node::vo::MoveNodeVO {
        id: node.id.clone(),
        x: 10.0,
        y: 20.0,
    }];
    user_database_node_move_nodes::preprocess(valid_items).unwrap();
    // 验证坐标确实被更新。
    let updated = node::service::list(&child.id, false)
        .unwrap()
        .into_iter()
        .find(|n| n.id == node.id)
        .unwrap();
    assert_eq!((updated.x, updated.y), (10.0, 20.0));

    // user_database_canvas_move_canvases::preprocess 失败路径：列表中含非法 id 时报 InvalidCanvasId。
    let invalid_canvas_items = vec![canvas::vo::MoveNodeVO {
        id: "not-a-uuid".to_string(),
        x: 1.0,
        y: 2.0,
    }];
    assert!(matches!(
        user_database_canvas_move_canvases::preprocess(invalid_canvas_items),
        Err(ErrorCode::InvalidCanvasId { .. })
    ));

    // user_database_canvas_move_canvases::preprocess 成功路径：空列表正常走到 service（service 对空列表返回 Ok）。
    user_database_canvas_move_canvases::preprocess(vec![]).unwrap();

    // user_database_canvas_move_canvases::preprocess 成功路径：合法 id 列表正常走到 service。
    let canvas_d = canvas::service::create(&root_id, "cmd-canvas-d".to_string()).unwrap();
    let valid_canvas_items = vec![canvas::vo::MoveNodeVO {
        id: canvas_d.id.clone(),
        x: 100.0,
        y: 200.0,
    }];
    user_database_canvas_move_canvases::preprocess(valid_canvas_items).unwrap();
    // 验证坐标确实被更新。
    let updated_canvas = canvas::service::list(false)
        .unwrap()
        .into_iter()
        .find(|c| c.id == canvas_d.id)
        .unwrap();
    assert_eq!((updated_canvas.x, updated_canvas.y), (100.0, 200.0));

    // user_database_node_relocate_nodes::preprocess 失败路径：items 含非法 id 时报 InvalidNodeId。
    let invalid_relocate_items = vec![node::vo::MoveNodeVO {
        id: "not-a-uuid".to_string(),
        x: 1.0,
        y: 2.0,
    }];
    assert!(matches!(
        user_database_node_relocate_nodes::preprocess(
            invalid_relocate_items,
            canvas_d.id.clone(),
        ),
        Err(ErrorCode::InvalidNodeId { .. })
    ));

    // user_database_node_relocate_nodes::preprocess 失败路径：目标画布 id 非法时报 InvalidCanvasId。
    let relocate_invalid_canvas = vec![node::vo::MoveNodeVO {
        id: node.id.clone(),
        x: 1.0,
        y: 2.0,
    }];
    assert!(matches!(
        user_database_node_relocate_nodes::preprocess(
            relocate_invalid_canvas,
            "not-a-uuid".to_string(),
        ),
        Err(ErrorCode::InvalidCanvasId { .. })
    ));

    // user_database_node_relocate_nodes::preprocess 成功路径：合法参数正常走到 service。
    // 把 cmd-batch-node 从 cmd-child 迁移到 cmd-canvas-d，验证画布归属确实改变。
    let node_x_before = node.x;
    let node_y_before = node.y;
    user_database_node_relocate_nodes::preprocess(
        vec![node::vo::MoveNodeVO {
            id: node.id.clone(),
            x: node_x_before + 100.0,
            y: node_y_before + 100.0,
        }],
        canvas_d.id.clone(),
    )
    .unwrap();
    let relocated_node = node::service::list(&canvas_d.id, false)
        .unwrap()
        .into_iter()
        .find(|n| n.id == node.id)
        .unwrap();
    assert_eq!(relocated_node.canvas_id, canvas_d.id);
    assert_eq!((relocated_node.x, relocated_node.y), (node_x_before + 100.0, node_y_before + 100.0));

    lifecycle::command::user_database_lifecycle_save::preprocess().unwrap();
    lifecycle::command::user_database_lifecycle_close::preprocess().unwrap();
    }

    test::cleanup(&path);
}
