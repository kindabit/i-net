use crate::business::user_database::entity::Node;
use crate::business::user_database::node::service;
use crate::error_code::ErrorCode;
use crate::util::preprocess_util;

/// 在指定画布内新建一个节点。
///
/// 有两种模式：
/// - `create_canvas == false`（普通节点）：可选地基于模板 id 复制模板字段结构。
/// - `create_canvas == true`（画布节点）：以 title 作为画布名称创建子画布，并在宿主画布内创建引用节点；
///   标题经 `preprocess_canvas_name` 校验非空。
///
/// # 参数
/// - `canvas_id`: 画布 id。
/// - `title`: 节点标题；`create_canvas == true` 时也作为画布名称的基础名。
/// - `sub_title`: 节点副标题。
/// - `x`: 节点在画布中的 x 坐标。
/// - `y`: 节点在画布中的 y 坐标。
/// - `template_id`: 可选的模板 id。
/// - `create_canvas`: 是否创建画布节点。
///
/// # 返回值
/// 返回新建的节点；若发生错误则返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_node_create(
    canvas_id: String,
    title: String,
    sub_title: String,
    x: f64,
    y: f64,
    template_id: Option<String>,
    create_canvas: bool,
) -> Result<Node, ErrorCode> {
    preprocess(canvas_id, title, sub_title, x, y, template_id, create_canvas)
}

/// `user_database_node_create` 的 preprocess 函数：校验参数后接入 service 层的 create 函数。
///
/// `create_canvas == true` 时，title 经 `preprocess_canvas_name` 校验（trim + 非空）；
/// `create_canvas == false` 时，title 仅裁剪首尾空白，允许为空。
/// sub_title 两种模式下均仅裁剪首尾空白，允许为空。
/// template_id 在 Some 时校验 uuid 格式。
pub fn preprocess(
    canvas_id: String,
    title: String,
    sub_title: String,
    x: f64,
    y: f64,
    template_id: Option<String>,
    create_canvas: bool,
) -> Result<Node, ErrorCode> {
    let canvas_id = preprocess_util::preprocess_canvas_id(canvas_id)?;
    let template_id = match template_id {
        Some(tid) => {
            let tid = tid.trim().to_string();
            match uuid::Uuid::parse_str(&tid) {
                Ok(uuid) if uuid.to_string() == tid => Some(tid),
                _ => return Err(ErrorCode::InvalidTemplateId { id: tid }),
            }
        }
        None => None,
    };
    let title = if create_canvas {
        preprocess_util::preprocess_canvas_name(title)?
    } else {
        title.trim().to_string()
    };
    let sub_title = sub_title.trim().to_string();
    service::create(
        &canvas_id,
        title,
        sub_title,
        x,
        y,
        template_id,
        create_canvas,
    )
}
