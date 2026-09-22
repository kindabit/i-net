use crate::business::user_database::entity::Action;
use crate::business::user_database::node::dao as node_dao;
use crate::business::user_database::node_tag::dao;
use crate::business::user_database::{log, state};
use crate::error_code::ErrorCode;

/// 全量设置指定节点的标签集合。
///
/// 入参标签逐个 trim 后丢弃空串并去重（保持首次出现顺序）；与旧标签集合完全相同时
/// 直接返回（不写库不写日志）；否则先删除旧标签再逐条插入新标签，
/// 产生 NodeTagsModify 日志（added / removed 为按升序排序的标签名列表）。
///
/// # 参数
/// - `node_id`: 节点 id。
/// - `tags`: 新的标签名称列表，允许包含空白与重复项。
///
/// # 返回值
/// 成功时返回 `Ok(())`；节点不存在时返回 `ErrorCode::NoNodeWithSuchId`，
/// 影子节点时返回 `ErrorCode::NodeIsShadow`，发生其他错误时返回对应的 `ErrorCode`。
pub fn set_for_node(node_id: &str, tags: Vec<String>) -> Result<(), ErrorCode> {
    let connection = state::lock_connection();
    let node = node_dao::select_by_id(&connection, node_id)?.ok_or_else(|| {
        ErrorCode::NoNodeWithSuchId {
            id: node_id.to_string(),
        }
    })?;
    // 影子节点不允许此操作（展示数据从本体节点拉取，生命周期由边管理）。
    if node.shadow_producing_edge_id.is_some() {
        return Err(ErrorCode::NodeIsShadow);
    }
    // 规整入参：trim 后丢弃空串，按首次出现顺序去重。
    let mut new_tags: Vec<String> = Vec::new();
    for tag in tags {
        let tag = tag.trim().to_string();
        if tag.is_empty() || new_tags.contains(&tag) {
            continue;
        }
        new_tags.push(tag);
    }
    // 旧标签已按名称升序，排序后的新标签集合与旧标签集合同序，便于直接比较与求差集。
    let old_tags = dao::select_tags_by_node_id(&connection, node_id)?;
    let mut sorted_new_tags = new_tags.clone();
    sorted_new_tags.sort();
    if sorted_new_tags == old_tags {
        return Ok(());
    }
    let added: Vec<String> = sorted_new_tags
        .iter()
        .filter(|tag| !old_tags.contains(tag))
        .cloned()
        .collect();
    let removed: Vec<String> = old_tags
        .iter()
        .filter(|tag| !sorted_new_tags.contains(tag))
        .cloned()
        .collect();
    dao::delete_by_node_id(&connection, node_id)?;
    for tag in &new_tags {
        dao::insert(&connection, node_id, tag)?;
    }
    log::service::create(Action::NodeTagsModify {
        node_title: node.title.clone(),
        added,
        removed,
    })?;
    Ok(())
}
