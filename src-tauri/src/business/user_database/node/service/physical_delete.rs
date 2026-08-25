use crate::business::user_database::entity::Action;
use crate::business::user_database::node::dao;
use crate::business::user_database::{attachment, canvas, log, state};
use crate::error_code::ErrorCode;
use crate::util::file_system_util;

/// 物理删除指定节点；与它相连的全部边、该节点的全部字段和附件元数据由外键
/// ON DELETE CASCADE 随节点行的删除一并删除，附件的磁盘文件在删行之前逐个清理。
/// 如果是画布节点，同时物理删除其引用的子画布。
///
/// 影子子树级联删除：节点的全部直接影子由 `node.shadow_id` 自引用外键随节点行一并
/// 删除；嵌套影子（影子的影子）则由外键逐层递归删除，影子自身相连的边由
/// `edge.source_id/target_id` 外键随之级联删除；应用层禁止手写递归删除。
///
/// 双阶段确认：删除会断开其它画布中影子子树所连接的节点（这些节点与本节点所在画布
/// 不在同一画布，是不可见的副作用）。受影响节点非空且 `confirmed = false` 时返回
/// `ErrorCode::NodeDeleteDisconnectsNodes`，由前端向用户确认后以 `confirmed = true`
/// 重调；本节点所在画布的邻居不断连收集（这些边随本节点一起删除是用户可见的预期行为）。
///
/// 产生 NodePhysicalDelete 日志，载荷为节点的标题。级联删除（边、字段、附件、影子）不记日志。
///
/// # 参数
/// - `id`: 节点 id。
/// - `confirmed`: 调用方已确认影子子树删除带来的跨画布连接断开影响。
///
/// # 返回值
/// 成功时返回 `Ok(())`；节点不存在时返回 `ErrorCode::NoNodeWithSuchId`，
/// 影子节点时返回 `ErrorCode::NodeIsShadow`，
/// 删除会使其它画布的节点失去连接且未确认时返回 `ErrorCode::NodeDeleteDisconnectsNodes`，
/// 发生其他错误时返回对应的 `ErrorCode`。
pub fn physical_delete(id: &str, confirmed: bool) -> Result<(), ErrorCode> {
    let connection = state::lock_connection();
    let node = dao::select_by_id(&connection, id)?
        .ok_or_else(|| ErrorCode::NoNodeWithSuchId { id: id.to_string() })?;
    // 影子节点不允许此操作（展示数据从原始节点拉取，生命周期由边管理）。
    if node.shadow_id.is_some() {
        return Err(ErrorCode::NodeIsShadow);
    }
    // 断连检测：收集节点的全部直接影子删除后在下游各级画布中失去连接的节点标题，
    // 仅对不在本画布的受影响节点做确认提示，本画布的边删除是用户可见的预期行为。
    let shadows = dao::select_by_shadow_id(&connection, id)?;
    let mut affected: Vec<String> = Vec::new();
    for shadow in &shadows {
        for title in crate::business::user_database::node::service::collect_shadow_disconnected(
            &connection,
            shadow,
        )? {
            if !affected.contains(&title) {
                affected.push(title);
            }
        }
    }
    if !affected.is_empty() && !confirmed {
        return Err(ErrorCode::NodeDeleteDisconnectsNodes { nodes: affected });
    }
    // 清理附件磁盘文件：先删文件再删节点行，附件元数据由外键随节点行级联删除。
    let attachments = attachment::dao::select_by_node_ids(&connection, &[id.to_string()])?;
    let path = crate::state::path();
    let user_uuid = state::metadata().id;
    for a in &attachments {
        let file = path.user_attachment_file(&user_uuid, &a.id);
        if file_system_util::try_exists(&file)? {
            file_system_util::remove_file(&file)?;
        }
    }
    // 删除节点行：相连的边（edge.source_id/target_id）、节点字段（node_field.node_id）、
    // 附件元数据（attachment.node_id）由外键级联删除；直接影子由 shadow_id 外键级联删除，
    // 嵌套影子与它们相连的边由外键逐层继续级联。
    dao::delete_by_id(&connection, id)?;
    let canvas_ref_id = node.canvas_ref_id.clone();
    log::service::create(id, Action::NodePhysicalDelete { title: node.title })?;
    // 级联物理删除引用的子画布
    if let Some(ref_id) = canvas_ref_id {
        canvas::service::physical_delete(&ref_id)?;
    }
    Ok(())
}
