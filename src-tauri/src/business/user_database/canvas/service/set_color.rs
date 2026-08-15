use crate::business::user_database::canvas::dao;
use crate::business::user_database::state;
use crate::error_code::ErrorCode;

/// 设置指定画布的颜色。
///
/// # 参数
/// - `id`: 画布 id。
/// - `color`: 前端序列化的自定义颜色，空串表示使用默认色。
///
/// # 返回值
/// 成功时返回 `Ok(())`；画布不存在时返回 `ErrorCode::NoCanvasWithSuchId`，
/// 发生其他错误时返回对应的 `ErrorCode`。
pub fn set_color(id: &str, color: String) -> Result<(), ErrorCode> {
    let connection = state::lock_connection();
    let mut canvas = dao::select_by_id(&connection, id)?
        .ok_or_else(|| ErrorCode::NoCanvasWithSuchId { id: id.to_string() })?;
    canvas.color = color;
    dao::update(&connection, &canvas)?;
    Ok(())
}
