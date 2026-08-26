use rusqlite::Connection;

use crate::business::user_database::entity::Node;
use crate::business::user_database::node;
use crate::error_code::ErrorCode;

/// 找到一条边关联的影子节点：target 是画布节点时，其引用画布内 source 的入向影子；
/// source 是画布节点时，其引用画布内 target 的出向影子；两端都是画布节点时两个影子都会返回。
///
/// 影子只可能由边联动创建并与"原始节点↔画布节点之间的边"同生共死；若画布节点端点存在
/// 但对应影子缺失，属于数据损坏或程序缺陷，返回 DataCorruptionMissingShadow。
///
/// # 参数
/// - `connection`: 数据库连接。
/// - `source`: 边的源节点。
/// - `target`: 边的目标节点。
///
/// # 返回值
/// 返回关联的影子节点列表（0 到 2 个）；影子缺失时返回 `ErrorCode::DataCorruptionMissingShadow`；
/// 发生数据库错误时返回对应的 `ErrorCode`。
pub fn shadows_of_edge(
    connection: &Connection,
    source: &Node,
    target: &Node,
) -> Result<Vec<Node>, ErrorCode> {
    let mut shadows: Vec<Node> = Vec::new();
    if let Some(ref_canvas_id) = &target.canvas_ref_id {
        let shadow = node::dao::select_by_shadow_id_and_canvas_id(
            connection,
            &source.id,
            ref_canvas_id,
        )?
        .ok_or_else(|| ErrorCode::DataCorruptionMissingShadow {
            origin_id: source.id.clone(),
            canvas_id: ref_canvas_id.clone(),
        })?;
        shadows.push(shadow);
    }
    if let Some(ref_canvas_id) = &source.canvas_ref_id {
        let shadow = node::dao::select_by_shadow_id_and_canvas_id(
            connection,
            &target.id,
            ref_canvas_id,
        )?
        .ok_or_else(|| ErrorCode::DataCorruptionMissingShadow {
            origin_id: target.id.clone(),
            canvas_id: ref_canvas_id.clone(),
        })?;
        shadows.push(shadow);
    }
    Ok(shadows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::business::user_database::node::dao;

    /// 构造测试用 Node，各字段可由调用方再修改。
    fn make_node(id: &str, canvas_id: &str) -> Node {
        Node {
            id: id.to_string(),
            canvas_id: canvas_id.to_string(),
            x: 0.0,
            y: 0.0,
            title: id.to_string(),
            sub_title: String::new(),
            canvas_ref_id: None,
            deleted: false,
            color: String::new(),
            shadow_id: None,
        }
    }

    /// 成功路径：target 为画布节点且影子存在时返回该影子；两端皆普通节点时返回空列表。
    /// 同一函数内连续构造两种拓扑以减少代码量。
    #[test]
    fn test_shadows_of_edge_success() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch("PRAGMA foreign_keys = OFF;")
            .unwrap();
        dao::create_table(&connection).unwrap();

        // 拓扑：sub 画布存在且被画布节点 B 引用，shadow 指向 X。
        dao::insert(&connection, &make_node("X", "parent")).unwrap();
        let mut b = make_node("B", "parent");
        b.canvas_ref_id = Some("sub".to_string());
        dao::insert(&connection, &b).unwrap();
        let mut shadow = make_node("S", "sub");
        shadow.title = String::new();
        shadow.sub_title = String::new();
        shadow.color = String::new();
        shadow.shadow_id = Some("X".to_string());
        dao::insert(&connection, &shadow).unwrap();

        // 边 X→B（target 为画布节点）→ 返回 X 的影子。
        let shadows = shadows_of_edge(&connection, &make_node("X", "parent"), &b).unwrap();
        assert_eq!(shadows.len(), 1);
        assert_eq!(shadows[0].id, "S");

        // 边 X→Y（两端皆普通节点）→ 返回空列表。
        dao::insert(&connection, &make_node("Y", "parent")).unwrap();
        let shadows =
            shadows_of_edge(&connection, &make_node("X", "parent"), &make_node("Y", "parent"))
                .unwrap();
        assert!(shadows.is_empty());
    }

    /// 失败路径：target 是画布节点但其引用画布内没有对应影子时报 DataCorruptionMissingShadow。
    #[test]
    fn test_shadows_of_edge_missing_shadow() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch("PRAGMA foreign_keys = OFF;")
            .unwrap();
        dao::create_table(&connection).unwrap();
        dao::insert(&connection, &make_node("X", "parent")).unwrap();
        let mut b = make_node("B", "parent");
        b.canvas_ref_id = Some("sub".to_string());
        dao::insert(&connection, &b).unwrap();
        // 不插入 shadow 即触发缺失路径。
        assert!(matches!(
            shadows_of_edge(&connection, &make_node("X", "parent"), &b),
            Err(ErrorCode::DataCorruptionMissingShadow {
                origin_id,
                canvas_id,
            }) if origin_id == "X" && canvas_id == "sub"
        ));
    }
}