use crate::business::user_database::node::service;
use crate::business::user_database::node::vo::MoveNodeVO;
use crate::error_code::ErrorCode;
use crate::util::preprocess_util;

/// 批量跨画布迁移节点。
///
/// # 参数
/// - `items`: 要迁移的节点列表（含最终坐标）。
/// - `target_canvas_id`: 目标画布 id。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_node_relocate_nodes(
    items: Vec<MoveNodeVO>,
    target_canvas_id: String,
) -> Result<(), ErrorCode> {
    preprocess(items, target_canvas_id)
}

/// `user_database_node_relocate_nodes` 的 preprocess 函数：校验每个 item 的 id 与目标画布 id 后接入 service 层的 relocate_nodes 函数。
pub fn preprocess(items: Vec<MoveNodeVO>, target_canvas_id: String) -> Result<(), ErrorCode> {
    let validated: Vec<MoveNodeVO> = items
        .into_iter()
        .map(|item| {
            let id = preprocess_util::preprocess_node_id(item.id)?;
            Ok(MoveNodeVO {
                id,
                x: item.x,
                y: item.y,
            })
        })
        .collect::<Result<Vec<_>, ErrorCode>>()?;
    let target_canvas_id = preprocess_util::preprocess_canvas_id(target_canvas_id)?;
    service::relocate_nodes(&validated, &target_canvas_id)
}