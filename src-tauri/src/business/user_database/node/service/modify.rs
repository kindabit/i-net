use crate::business::user_database::entity::Action;
use crate::business::user_database::node::dao;
use crate::business::user_database::{canvas, log, state};
use crate::error_code::ErrorCode;

/// 修改指定节点的标题和副标题。
///
/// 若节点是画布节点且标题实际发生变化，同步重命名其引用的画布以保持两侧标题一致：
/// 先检测新标题是否与其它画布重名，重名则整个修改失败（节点与画布均不落库）；
/// 同步重命名时产生 CanvasRename 日志。引用画布必然随节点存在（canvas_ref_id 外键
/// ON DELETE CASCADE），查不到即为数据损坏，返回 DataCorruptionCanvasRefMissing 触发受控崩溃。
///
/// 产生 NodeModify 日志，载荷为旧标题、副标题和新标题、副标题。
///
/// # 参数
/// - `id`: 节点 id。
/// - `title`: 新标题。
/// - `sub_title`: 新副标题。
///
/// # 返回值
/// 成功时返回 `Ok(())`；节点不存在时返回 `ErrorCode::NoNodeWithSuchId`，影子节点时返回 `ErrorCode::NodeIsShadow`，
/// 画布节点的新标题与其它画布重复时返回 `ErrorCode::CanvasNameAlreadyExists`，
/// 画布节点引用的画布不存在时返回 `ErrorCode::DataCorruptionCanvasRefMissing`，
/// 发生其他错误时返回对应的 `ErrorCode`。
pub fn modify(id: &str, title: String, sub_title: String) -> Result<(), ErrorCode> {
    let connection = state::lock_connection();
    let mut node = dao::select_by_id(&connection, id)?
        .ok_or_else(|| ErrorCode::NoNodeWithSuchId { id: id.to_string() })?;
    // 影子节点不允许此操作（展示数据从原始节点拉取，生命周期由边管理）。
    if node.shadow_id.is_some() {
        return Err(ErrorCode::NodeIsShadow);
    }
    // 画布节点的标题与引用画布的名称保持一致：标题变化时先检测新标题是否与其它画布重名，
    // 重名则整个修改失败（先于任何写操作，节点与画布均不落库）。
    let sync_canvas = match node.canvas_ref_id {
        Some(ref ref_id) if node.title != title => {
            if let Some(existing) = canvas::dao::select_by_name(&connection, &title)? {
                if existing.id != *ref_id {
                    return Err(ErrorCode::CanvasNameAlreadyExists { name: title });
                }
            }
            true
        }
        _ => false,
    };
    let old_title = std::mem::replace(&mut node.title, title);
    let old_sub_title = std::mem::replace(&mut node.sub_title, sub_title);
    dao::update(&connection, &node)?;
    // 同步重命名引用画布。画布必然随节点存在（canvas_ref_id 外键 ON DELETE CASCADE），
    // 查不到即为数据损坏，返回 DataCorruptionCanvasRefMissing 触发受控崩溃；
    // 画布名已等于新标题时不落库也不产生日志。
    if sync_canvas {
        if let Some(ref ref_id) = node.canvas_ref_id {
            let mut referenced = canvas::dao::select_by_id(&connection, ref_id)?
                .ok_or_else(|| ErrorCode::DataCorruptionCanvasRefMissing {
                    node_id: id.to_string(),
                    canvas_id: ref_id.clone(),
                })?;
            if referenced.name != node.title {
                let canvas_old_name =
                    std::mem::replace(&mut referenced.name, node.title.clone());
                canvas::dao::update(&connection, &referenced)?;
                log::service::create(
                    ref_id,
                    Action::CanvasRename {
                        old_name: canvas_old_name,
                        new_name: referenced.name,
                    },
                )?;
            }
        }
    }
    log::service::create(
        id,
        Action::NodeModify {
            old_title,
            old_sub_title,
            new_title: node.title,
            new_sub_title: node.sub_title,
        },
    )?;
    Ok(())
}
