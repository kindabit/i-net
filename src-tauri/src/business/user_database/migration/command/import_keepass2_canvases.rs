use crate::business::user_database::migration::service;
use crate::business::user_database::migration::vo::ImportedCanvasVO;
use crate::error_code::ErrorCode;
use crate::util::preprocess_util;

/// 数据迁移多画布聚合导入接口（KeePass 2.0）：接收前端按迁移源 group 层级构造好的画布列表，
/// 在根画布下批量创建整棵画布子树（画布内写入数据节点、字段与引用直接子画布的画布数据节点），
/// 所有写库操作聚合为一条日志条目。
///
/// # 参数
/// - `canvases`: 前端构造好的导入画布列表（第一个元素为迁移源根 group 对应的画布；
///   宇宙坐标与画布内矩阵坐标均已由前端计算）。
///
/// # 返回值
/// 成功时返回 `Ok(根 group 画布的 id)`（前端凭此跳转至顶层新画布）；
/// 任一画布名称为空时返回 `ErrorCode::EmptyCanvasName`，
/// 发生其他错误时返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_migration_import_keepass2_canvases(
    canvases: Vec<ImportedCanvasVO>,
) -> Result<String, ErrorCode> {
    preprocess(canvases)
}

/// `user_database_migration_import_keepass2_canvases` 的 preprocess 函数：逐个预处理
/// 画布名称（去首尾空白、拒绝空名）后接入 service 层的 import_keepass2_canvases 函数。
///
/// # 参数
/// - `canvases`: 前端构造好的导入画布列表。
///
/// # 返回值
/// 成功时返回 `Ok(根 group 画布的 id)`；任一画布名称为空时返回 `ErrorCode::EmptyCanvasName`，
/// 其他错误由 service 层返回。
pub fn preprocess(canvases: Vec<ImportedCanvasVO>) -> Result<String, ErrorCode> {
    let mut preprocessed = Vec::with_capacity(canvases.len());
    for canvas in canvases {
        preprocessed.push(ImportedCanvasVO {
            name: preprocess_util::preprocess_canvas_name(canvas.name)?,
            ..canvas
        });
    }
    service::import_keepass2_canvases(&preprocessed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::business::metadata;
    use crate::business::user_database::canvas;
    use crate::business::user_database::lifecycle;
    use crate::business::user_database::migration::vo::ImportedNodeVO;
    use crate::test;

    /// 构造单个无内容画布的辅助闭包（parent_index 为 None，挂到根画布）。
    fn blank_canvas(name: &str) -> ImportedCanvasVO {
        ImportedCanvasVO {
            name: name.to_string(),
            x: 0.0,
            y: 0.0,
            parent_index: None,
            nodes: Vec::<ImportedNodeVO>::new(),
            canvas_nodes: Vec::new(),
        }
    }

    /// 数据迁移多画布聚合导入 command 层 preprocess：任一画布名称为空串或纯空白时报
    /// EmptyCanvasName（其余画布不落库）；名称合法时接入 service 层并返回顶层新建画布 id，
    /// 名称首尾空白被去除。
    #[test]
    fn test_import_keepass2_canvases_preprocess() {
        let _guard = test::acquire_test_lock();

        // 初始化测试数据目录、metadata 数据库并打开一个全新的用户数据库。
        let path = test::create_test_path();
        crate::state::set_path(path.clone());
        metadata::service::initialize().unwrap();
        let registered =
            metadata::service::register("migration-keepass2-canvases-cmd-test-db".to_string())
                .unwrap();
        lifecycle::service::initialize(&registered.id, test::test_key()).unwrap();

        // ===== 失败路径：任一画布名称为空（空串与纯空白）返回 EmptyCanvasName，且不写库 =====
        assert!(matches!(
            preprocess(vec![blank_canvas("ok"), blank_canvas("")]),
            Err(ErrorCode::EmptyCanvasName)
        ));
        assert!(matches!(
            preprocess(vec![blank_canvas("  ")]),
            Err(ErrorCode::EmptyCanvasName)
        ));
        assert_eq!(canvas::service::list(false).unwrap().len(), 1);

        // 成功路径：preprocess 接入 service 层并返回顶层新建画布 id，
        // 名称首尾空白被去除（" cmd-canvases " 落库为 "cmd-canvases"）。
        let imported_id = preprocess(vec![blank_canvas(" cmd-canvases ")]).unwrap();
        let canvases = canvas::service::list(false).unwrap();
        let imported = canvases
            .iter()
            .find(|c| c.id == imported_id)
            .expect("canvas should exist after command import");
        assert_eq!(imported.name, "cmd-canvases");

        lifecycle::service::save().unwrap();
        lifecycle::service::close().unwrap();
        test::cleanup(&path);
    }
}
