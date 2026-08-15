use crate::business::user_database::canvas::dao;
use crate::business::user_database::entity::{Action, Canvas};
use crate::business::user_database::{log, state};
use crate::error_code::ErrorCode;

/// 在指定父画布下新建一个子画布：先检测名称是否重复，
/// 然后通过简易 layout 引擎在父画布附近选择一个合适的位置。
///
/// 产生 CanvasCreate 日志，载荷内记录画布名称。
///
/// # 参数
/// - `parent_id`: 父画布 id。
/// - `name`: 新画布的名称。
///
/// # 返回值
/// 返回新建的画布；名称重复时返回 `ErrorCode::CanvasNameAlreadyExists`，
/// 父画布不存在或已逻辑删除时返回 `ErrorCode::NoCanvasWithSuchId`，
/// 发生其他错误时返回对应的 `ErrorCode`。
pub fn create(parent_id: &str, name: String) -> Result<Canvas, ErrorCode> {
    let connection = state::lock_connection();
    if dao::select_by_name(&connection, &name)?.is_some() {
        return Err(ErrorCode::CanvasNameAlreadyExists { name });
    }
    let parent = dao::select_by_id(&connection, parent_id)?
        .filter(|parent| !parent.deleted)
        .ok_or_else(|| ErrorCode::NoCanvasWithSuchId {
            id: parent_id.to_string(),
        })?;
    let all = dao::select_all(&connection)?;
    let (x, y) = layout(&parent, &all);
    let canvas = Canvas {
        id: uuid::Uuid::new_v4().to_string(),
        parent_id: Some(parent_id.to_string()),
        name,
        x,
        y,
        deleted: false,
        color: String::new(),
    };
    dao::insert(&connection, &canvas)?;
    log::service::create(
        &canvas.id,
        Action::CanvasCreate {
            name: canvas.name.clone(),
        },
    )?;
    Ok(canvas)
}

/// 简易 layout 引擎：以父画布坐标为圆心逐圈向外搜索候选位置，
/// 返回第一个与所有现存画布（含已逻辑删除的）中心的欧氏距离都不小于 200 的候选点。
///
/// 半径从 240 起每圈增加 120，每圈取 8 个等分角度（起点角度为 0），
/// 最多搜索 32 圈；都找不到时兜底返回父画布坐标向右偏移 240 的位置。
///
/// # 参数
/// - `parent`: 父画布。
/// - `all`: 全部现存画布。
///
/// # 返回值
/// 返回新画布的坐标。
fn layout(parent: &Canvas, all: &[Canvas]) -> (f64, f64) {
    let is_free = |x: f64, y: f64| {
        all.iter().all(|canvas| {
            let dx = canvas.x - x;
            let dy = canvas.y - y;
            (dx * dx + dy * dy).sqrt() >= 200.0
        })
    };
    for ring in 0..32 {
        let radius = 240.0 + ring as f64 * 120.0;
        for index in 0..8 {
            let angle = index as f64 * std::f64::consts::TAU / 8.0;
            let x = parent.x + radius * angle.cos();
            let y = parent.y + radius * angle.sin();
            if is_free(x, y) {
                return (x, y);
            }
        }
    }
    (parent.x + 240.0, parent.y)
}
