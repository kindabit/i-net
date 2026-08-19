use crate::business::user_database::entity::{Action, Node};
use crate::business::user_database::entity::NodeField;
use crate::business::user_database::node::dao;
use crate::business::user_database::{canvas, log, node_field, state, template};
use crate::error_code::ErrorCode;

/// 在指定画布内新建一个节点。
///
/// 有两种模式：
/// - `create_canvas == false`（普通节点）：宿主画布仅校验存在。可选地基于模板 id 复制模板字段结构。
/// - `create_canvas == true`（画布节点）：宿主画布须存在且未逻辑删除；以 title 为基础名去重
///   （重复时追加 " 2"、" 3"…）；创建子画布后在宿主画布内创建引用它的节点
///   （`canvas_ref_id = Some(canvas.id)`）；insert 失败时补偿物理删除画布。
///   可选地同样基于模板 id 复制模板字段结构。
///
/// 产生 NodeCreate 日志，载荷为节点的标题和副标题。
///
/// # 参数
/// - `canvas_id`: 画布 id。
/// - `title`: 节点标题；`create_canvas == true` 时也作为画布名称的基础名。
/// - `sub_title`: 节点副标题。
/// - `x`: 节点在画布中的 x 坐标。
/// - `y`: 节点在画布中的 y 坐标。
/// - `template_id`: 可选的模板 id，用于从模板复制字段结构。
/// - `create_canvas`: 是否创建画布节点。
///
/// # 返回值
/// 返回新建的节点；画布不存在时返回 `ErrorCode::NoCanvasWithSuchId`，
/// 模板不存在时返回 `ErrorCode::NoTemplateWithSuchId`，
/// 发生其他错误时返回对应的 `ErrorCode`。
pub fn create(
    canvas_id: &str,
    title: String,
    sub_title: String,
    x: f64,
    y: f64,
    template_id: Option<String>,
    create_canvas: bool,
) -> Result<Node, ErrorCode> {
    let connection = state::lock_connection();

    let host = canvas::dao::select_by_id(&connection, canvas_id)?
        .filter(|c| !create_canvas || !c.deleted)
        .ok_or_else(|| ErrorCode::NoCanvasWithSuchId {
            id: canvas_id.to_string(),
        })?;
    if let Some(ref tid) = template_id {
        template::dao::select_by_id(&connection, tid)?
            .ok_or_else(|| ErrorCode::NoTemplateWithSuchId {
                id: tid.clone(),
            })?;
    }

    if create_canvas {
        let mut final_name = title.clone();
        let mut suffix = 2u32;
        while canvas::dao::select_by_name(&connection, &final_name)?.is_some() {
            final_name = format!("{title} {suffix}");
            suffix += 1;
        }

        let canvas = canvas::service::create(&host.id, final_name.clone())?;

        let node = Node {
            id: uuid::Uuid::new_v4().to_string(),
            canvas_id: host.id.clone(),
            x,
            y,
            title: final_name,
            sub_title,
            canvas_ref_id: Some(canvas.id.clone()),
            deleted: false,
            color: String::new(),
            shadow_id: None,
        };

        if let Err(e) = dao::insert(&connection, &node) {
            let _ = canvas::service::physical_delete(&canvas.id);
            return Err(e);
        }
        if let Some(ref tid) = template_id {
            copy_template_fields(&connection, &node.id, tid)?;
        }

        log::service::create(
            &node.id,
            Action::NodeCreate {
                title: node.title.clone(),
                sub_title: node.sub_title.clone(),
            },
        )?;
        return Ok(node);
    }

    let node = Node {
        id: uuid::Uuid::new_v4().to_string(),
        canvas_id: canvas_id.to_string(),
        x,
        y,
        title,
        sub_title,
        canvas_ref_id: None,
        deleted: false,
        color: String::new(),
        shadow_id: None,
    };
    dao::insert(&connection, &node)?;
    if let Some(ref tid) = template_id {
        copy_template_fields(&connection, &node.id, tid)?;
    }
    log::service::create(
        &node.id,
        Action::NodeCreate {
            title: node.title.clone(),
            sub_title: node.sub_title.clone(),
        },
    )?;
    Ok(node)
}

/// 将指定模板的字段结构复制为指定节点的节点字段（field_value 为 None）。
fn copy_template_fields(
    connection: &rusqlite::Connection,
    node_id: &str,
    template_id: &str,
) -> Result<(), ErrorCode> {
    let tpl_fields =
        template::dao::select_fields_by_template_id(connection, template_id)?;
    for tpl_field in tpl_fields {
        let node_field = NodeField {
            node_id: node_id.to_string(),
            name: tpl_field.name,
            field_type: tpl_field.field_type,
            type_config: tpl_field.type_config,
            field_value: None,
            order: tpl_field.order,
            dictionary_id: tpl_field.dictionary_id,
        };
        node_field::dao::insert(connection, &node_field)?;
    }
    Ok(())
}
