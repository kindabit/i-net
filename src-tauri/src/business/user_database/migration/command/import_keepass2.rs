use crate::business::user_database::migration::service;
use crate::business::user_database::migration::vo::{ImportedEdgeVO, ImportedNodeVO};
use crate::error_code::ErrorCode;
use crate::util::preprocess_util;

/// 数据迁移聚合导入接口（KeePass 2.0）：接收前端已构造好的节点与边数据，
/// 在根画布内创建一个画布节点，其引用的新画布内批量写入全部节点、字段与父子边，
/// 所有写库操作聚合为一条日志条目。
///
/// # 参数
/// - `canvas_name`: 新画布的名称（画布节点标题与其保持一致），重名时自动追加 " 2"、" 3"…。
/// - `canvas_node_x`: 画布节点在根画布中的 x 坐标。
/// - `canvas_node_y`: 画布节点在根画布中的 y 坐标。
/// - `nodes`: 前端构造好的导入节点列表（第一个节点表示数据库本身，为树的根）。
/// - `edges`: 前端构造好的父子边列表（下标引用 `nodes`）。
///
/// # 返回值
/// 成功时返回 `Ok(新画布的 id)`（前端凭此跳转至新画布）；
/// 画布名称为空时返回 `ErrorCode::EmptyCanvasName`，
/// 发生其他错误时返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_migration_import_keepass2(
    canvas_name: String,
    canvas_node_x: f64,
    canvas_node_y: f64,
    nodes: Vec<ImportedNodeVO>,
    edges: Vec<ImportedEdgeVO>,
) -> Result<String, ErrorCode> {
    preprocess(canvas_name, canvas_node_x, canvas_node_y, nodes, edges)
}

/// `user_database_migration_import_keepass2` 的 preprocess 函数：校验画布名称非空后
/// 接入 service 层的 import_keepass2 函数。
///
/// # 参数
/// - `canvas_name`: 新画布的名称。
/// - `canvas_node_x`: 画布节点在根画布中的 x 坐标。
/// - `canvas_node_y`: 画布节点在根画布中的 y 坐标。
/// - `nodes`: 前端构造好的导入节点列表。
/// - `edges`: 前端构造好的父子边列表（下标引用 `nodes`）。
///
/// # 返回值
/// 成功时返回 `Ok(新画布的 id)`；画布名称为空时返回 `ErrorCode::EmptyCanvasName`，
/// 其他错误由 service 层返回。
pub fn preprocess(
    canvas_name: String,
    canvas_node_x: f64,
    canvas_node_y: f64,
    nodes: Vec<ImportedNodeVO>,
    edges: Vec<ImportedEdgeVO>,
) -> Result<String, ErrorCode> {
    let canvas_name = preprocess_util::preprocess_canvas_name(canvas_name)?;
    service::import_keepass2(&canvas_name, canvas_node_x, canvas_node_y, &nodes, &edges)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::business::metadata;
    use crate::business::user_database::canvas;
    use crate::business::user_database::lifecycle;
    use crate::business::user_database::node_field::vo::NodeFieldVO;
    use crate::test;

    /// 数据迁移聚合导入 command 层 preprocess：画布名称为空串或纯空白时报 EmptyCanvasName；
    /// 名称合法时接入 service 层并返回新建画布 id。
    #[test]
    fn test_import_keepass2_preprocess() {
        let _guard = test::acquire_test_lock();

        // 初始化测试数据目录、metadata 数据库并打开一个全新的用户数据库。
        let path = test::create_test_path();
        crate::state::set_path(path.clone());
        metadata::service::initialize().unwrap();
        let registered = metadata::service::register("migration-keepass2-cmd-test-db".to_string()).unwrap();
        lifecycle::service::initialize(&registered.id, test::test_key()).unwrap();

        // 构造字段的辅助闭包：dictionary_id 恒为 None。
        let field = |name: &str, field_type: &str, value: Option<&str>| NodeFieldVO {
            name: name.to_string(),
            field_type: field_type.to_string(),
            value: value.map(str::to_string),
            dictionary_id: None,
        };
        // 两个导入节点：第一个含 3 个字段（密码/访问链接有值、备注无值），第二个无字段。
        let nodes = vec![
            ImportedNodeVO {
                title: "User Name".to_string(),
                sub_title: "Sample Entry".to_string(),
                x: 0.0,
                y: 0.0,
                fields: vec![
                    field("密码", "string:password", Some("Password")),
                    field("访问链接", "string:url", Some("http://keepass.info/")),
                    field("备注", "string:multiple-line", None),
                ],
            },
            ImportedNodeVO {
                title: "General".to_string(),
                sub_title: String::new(),
                x: 240.0,
                y: 160.0,
                fields: Vec::new(),
            },
        ];
        // 一条父子边：节点 0（父）→ 节点 1（子）。
        let edges = vec![ImportedEdgeVO {
            source_index: 0,
            target_index: 1,
        }];

        // ===== 失败路径：command 层 preprocess 空画布名（空串与纯空白）返回 EmptyCanvasName =====
        assert!(matches!(
            preprocess(String::new(), 0.0, 240.0, nodes.clone(), edges.clone()),
            Err(ErrorCode::EmptyCanvasName)
        ));
        assert!(matches!(
            preprocess("  ".to_string(), 0.0, 240.0, nodes.clone(), edges.clone()),
            Err(ErrorCode::EmptyCanvasName)
        ));

        // 成功路径：preprocess 接入 service 层并返回新建画布 id，对应画布已创建且名称为 "cmd-import"。
        let imported_id =
            preprocess("cmd-import".to_string(), 0.0, 240.0, nodes, edges).unwrap();
        let canvases = canvas::service::list(false).unwrap();
        let imported = canvases
            .iter()
            .find(|c| c.id == imported_id)
            .expect("canvas should exist after command import");
        assert_eq!(imported.name, "cmd-import");

        lifecycle::service::save().unwrap();
        lifecycle::service::close().unwrap();
        test::cleanup(&path);
    }
}
