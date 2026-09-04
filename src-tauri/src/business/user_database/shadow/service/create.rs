use rusqlite::Connection;

use crate::business::user_database::entity::{Edge, Node};
use crate::business::user_database::node;
use crate::business::user_database::shadow::service::{resolve_root, shadow_direction};
use crate::business::user_database::shadow::vo::ShadowDirection;
use crate::error_code::ErrorCode;

/// 按连接规则为新建的边联动创建影子节点（不产生影子的连接直接返回）：
/// - 源端是画布节点（画布节点→画布节点 / 画布节点→出向影子）：在源画布节点引用的子画布内
///   创建目标端根本体（必为画布节点）的出向影子；
/// - 目标端是画布节点（普通节点/入向影子→画布节点）：在目标画布节点引用的子画布内创建
///   源端根本体（必为普通节点）的入向影子；
/// - 目标端是出向影子（普通节点→出向影子）：向上查找目标影子的根本体画布节点，
///   在其引用的子画布内创建源端根本体（必为普通节点）的入向影子；
/// - 其余连接（普通→普通、入向影子→普通）不产生影子。
/// 影子的 shadow_id 指向产生它的边，生命周期完全由边控制。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `edge`: 刚插入的新边。
/// - `source`: 边的源节点。
/// - `target`: 边的目标节点。
///
/// # 返回值
/// 成功时返回 `Ok(())`；根本体类型与预期矛盾时返回 `ErrorCode::DataCorruptionShadowRootTypeMismatch`；
/// 产生边链解析失败时返回对应的 `DataCorruption*` 错误；数据库错误返回对应的 `ErrorCode`。
pub fn create_shadow_for_edge(
    connection: &Connection,
    edge: &Edge,
    source: &Node,
    target: &Node,
) -> Result<(), ErrorCode> {
    if let Some(ref_canvas_id) = &source.canvas_ref_id {
        // 出向影子：本体链在目标端，根本体必须是画布节点。
        let root = resolve_root(connection, target)?;
        if root.canvas_ref_id.is_none() {
            return Err(ErrorCode::DataCorruptionShadowRootTypeMismatch {
                shadow_id: target.id.clone(),
                root_id: root.id.clone(),
            });
        }
        return create_shadow(connection, ref_canvas_id, &edge.id, ShadowDirection::Outflow);
    }
    if target.canvas_ref_id.is_none() && target.shadow_id.is_none() {
        // 普通→普通、入向影子→普通：不产生影子。
        return Ok(());
    }
    // 入向影子：本体链在源端，根本体必须是普通节点。
    let root = resolve_root(connection, source)?;
    if root.canvas_ref_id.is_some() {
        return Err(ErrorCode::DataCorruptionShadowRootTypeMismatch {
            shadow_id: source.id.clone(),
            root_id: root.id.clone(),
        });
    }
    // 落点画布：target 是画布节点时取其 canvas_ref_id；target 是出向影子时沿其本体链
    // 找到根本体画布节点，取其 canvas_ref_id。
    let shadow_canvas_id = match &target.canvas_ref_id {
        Some(ref_canvas_id) => ref_canvas_id.clone(),
        None => {
            let target_root = resolve_root(connection, target)?;
            target_root.canvas_ref_id.clone().ok_or_else(|| {
                ErrorCode::DataCorruptionShadowRootTypeMismatch {
                    shadow_id: target.id.clone(),
                    root_id: target_root.id.clone(),
                }
            })?
        }
    };
    create_shadow(connection, &shadow_canvas_id, &edge.id, ShadowDirection::Inflow)
}

/// 在指定画布内创建由指定产生边产生的影子节点：只有位置与 shadow_id 是影子自己的数据，
/// shadow_id 指向产生该影子的边；title / sub_title / color 以满足非空约束的空串落库。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `canvas_id`: 影子所在画布（被画布节点引用的子画布）的 id。
/// - `edge_id`: 产生该影子的边的 id，写入 shadow_id。
/// - `direction`: 影子方向（入向偏左车道，出向偏右车道）。
///
/// # 返回值
/// 成功时返回 `Ok(())`；发生错误时返回对应的 `ErrorCode`。
fn create_shadow(
    connection: &Connection,
    canvas_id: &str,
    edge_id: &str,
    direction: ShadowDirection,
) -> Result<(), ErrorCode> {
    let (x, y) = shadow_position(connection, canvas_id, direction)?;
    let shadow = Node {
        id: uuid::Uuid::new_v4().to_string(),
        canvas_id: canvas_id.to_string(),
        x,
        y,
        title: String::new(),
        sub_title: String::new(),
        canvas_ref_id: None,
        deleted: false,
        color: String::new(),
        shadow_id: Some(edge_id.to_string()),
    };
    node::dao::insert(connection, &shadow)
}

/// 计算新建影子节点的初始位置（不做网格吸附；吸附由前端在坐标写入后端前完成）：
/// 入向影子放在画布现有非影子内容左侧车道（最小 x - 400），
/// 出向影子放在右侧车道（最大 x + 400）；画布内还没有非影子节点时分别取 0 / 400。
/// 同一画布内已有同方向影子时垂直堆叠在其下方（最大 y + 120），避免影子互相重叠。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `canvas_id`: 影子所在画布的 id。
/// - `direction`: 影子方向。
///
/// # 返回值
/// 返回 (x, y) 坐标；发生数据库错误时返回对应的 `ErrorCode`。
fn shadow_position(
    connection: &Connection,
    canvas_id: &str,
    direction: ShadowDirection,
) -> Result<(f64, f64), ErrorCode> {
    let nodes = node::dao::select_by_canvas_id_and_deleted(connection, canvas_id, false)?;
    let content: Vec<&Node> = nodes.iter().filter(|n| n.shadow_id.is_none()).collect();
    let lane_x = match direction {
        ShadowDirection::Inflow => content
            .iter()
            .map(|n| n.x)
            .reduce(f64::min)
            .map(|min_x| min_x - 400.0)
            .unwrap_or(0.0),
        ShadowDirection::Outflow => content
            .iter()
            .map(|n| n.x)
            .reduce(f64::max)
            .map(|max_x| max_x + 400.0)
            .unwrap_or(400.0),
    };
    // 同方向已有影子的最大 y：逐个推导方向，与新建影子同方向的参与堆叠。
    let mut stack_y: Option<f64> = None;
    for existing in nodes.iter().filter(|n| n.shadow_id.is_some()) {
        if shadow_direction(connection, existing)? != direction {
            continue;
        }
        stack_y = Some(match stack_y {
            Some(max_y) => f64::max(max_y, existing.y),
            None => existing.y,
        });
    }
    let y = stack_y.map(|max_y| max_y + 120.0).unwrap_or(0.0);
    Ok((lane_x, y))
}
