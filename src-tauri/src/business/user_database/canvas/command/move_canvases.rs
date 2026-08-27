use crate::business::user_database::canvas::service;
use crate::business::user_database::canvas::vo::MoveNodeVO;
use crate::error_code::ErrorCode;
use crate::util::preprocess_util;

/// 批量移动画布坐标。
///
/// # 参数
/// - `items`: 要移动的画布列表。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_canvas_move_canvases(items: Vec<MoveNodeVO>) -> Result<(), ErrorCode> {
    preprocess(items)
}

/// `user_database_canvas_move_canvases` 的 preprocess 函数：校验每个 item 的 id 后接入 service 层的 move_canvases 函数。
pub fn preprocess(items: Vec<MoveNodeVO>) -> Result<(), ErrorCode> {
    let validated: Vec<MoveNodeVO> = items
        .into_iter()
        .map(|item| {
            let id = preprocess_util::preprocess_canvas_id(item.id)?;
            Ok(MoveNodeVO {
                id,
                x: item.x,
                y: item.y,
            })
        })
        .collect::<Result<Vec<_>, ErrorCode>>()?;
    service::move_canvases(&validated)
}
