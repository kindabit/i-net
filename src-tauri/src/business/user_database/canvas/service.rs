mod color_list;
mod create;
mod initialize;
mod list;
mod logical_delete;
mod move_canvas;
mod move_canvases;
mod physical_delete;
mod rename;
mod restore;
mod set_color;

pub use color_list::color_list;
pub use create::create;
pub use initialize::initialize;
pub use list::list;
pub use logical_delete::logical_delete;
pub use move_canvas::move_canvas;
pub use move_canvases::move_canvases;
pub use physical_delete::physical_delete;
pub use rename::rename;
pub use restore::restore;
pub use set_color::set_color;

use crate::business::user_database::entity::Canvas;

/// 收集以指定画布为根的子树内全部画布的 id（含根画布自身）。
///
/// # 参数
/// - `all`: 全部画布。
/// - `root`: 子树根画布的 id。
///
/// # 返回值
/// 返回子树内全部画布的 id。
fn collect_subtree_ids(all: &[Canvas], root: &str) -> Vec<String> {
    let mut result = vec![root.to_string()];
    let mut index = 0;
    while index < result.len() {
        let current = result[index].clone();
        for canvas in all
            .iter()
            .filter(|canvas| canvas.parent_id.as_deref() == Some(current.as_str()))
        {
            if !result.contains(&canvas.id) {
                result.push(canvas.id.clone());
            }
        }
        index += 1;
    }
    result
}

/// 收集指定画布的全部祖先画布的 id（从父画布开始向根方向排列，不含自身）。
///
/// # 参数
/// - `all`: 全部画布。
/// - `id`: 画布 id。
///
/// # 返回值
/// 返回全部祖先画布的 id。
fn collect_ancestor_ids(all: &[Canvas], id: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = all
        .iter()
        .find(|canvas| canvas.id == id)
        .and_then(|canvas| canvas.parent_id.clone());
    while let Some(parent_id) = current {
        // 防御：parent_id 构成环时终止向上遍历。
        if result.contains(&parent_id) {
            break;
        }
        result.push(parent_id.clone());
        current = all
            .iter()
            .find(|canvas| canvas.id == parent_id)
            .and_then(|canvas| canvas.parent_id.clone());
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造测试用 Canvas：id 与 parent_id 由调用方指定，其余字段与这些测试无关。
    fn canvas(id: &str, parent_id: Option<&str>) -> Canvas {
        Canvas {
            id: id.to_string(),
            parent_id: parent_id.map(str::to_string),
            name: id.to_string(),
            x: 0.0,
            y: 0.0,
            deleted: false,
            color: String::new(),
        }
    }

    /// 覆盖 collect_subtree_ids 与 collect_ancestor_ids 的成功与防御路径。
    #[test]
    fn test_canvas_service_helpers() {
        // collect_subtree_ids：只有根画布时，子树只含根画布自身。
        let all = vec![canvas("root", None)];
        assert_eq!(collect_subtree_ids(&all, "root"), vec!["root".to_string()]);

        // collect_ancestor_ids：根画布没有父画布，祖先列表为空。
        assert!(collect_ancestor_ids(&all, "root").is_empty());

        // 多层嵌套：root 下有 child 和 other，child 下有 grandchild。
        let all = vec![
            canvas("root", None),
            canvas("child", Some("root")),
            canvas("grandchild", Some("child")),
            canvas("other", Some("root")),
        ];
        // collect_subtree_ids：收集 child 的子树只含 child 与 grandchild，不含 root 与 other。
        let mut ids = collect_subtree_ids(&all, "child");
        ids.sort();
        assert_eq!(ids, vec!["child".to_string(), "grandchild".to_string()]);
        // collect_subtree_ids：从根画布收集时包含全部画布。
        let mut ids = collect_subtree_ids(&all, "root");
        ids.sort();
        assert_eq!(
            ids,
            vec![
                "child".to_string(),
                "grandchild".to_string(),
                "other".to_string(),
                "root".to_string()
            ]
        );
        // collect_ancestor_ids：从孙画布向根方向依次为 child、root，不含自身。
        assert_eq!(
            collect_ancestor_ids(&all, "grandchild"),
            vec!["child".to_string(), "root".to_string()]
        );

        // collect_ancestor_ids：父画布在集合中不存在时，向上遍历记录该 id 后终止。
        let all = vec![canvas("orphan", Some("missing"))];
        assert_eq!(
            collect_ancestor_ids(&all, "orphan"),
            vec!["missing".to_string()]
        );

        // parent_id 成环（a <-> b）时两个函数都防御性终止：
        // collect_subtree_ids 不会死循环，结果为环内全部画布；
        // collect_ancestor_ids 不会死循环，也不会重复记录。
        let all = vec![canvas("a", Some("b")), canvas("b", Some("a"))];
        let mut ids = collect_subtree_ids(&all, "a");
        ids.sort();
        assert_eq!(ids, vec!["a".to_string(), "b".to_string()]);
        assert_eq!(
            collect_ancestor_ids(&all, "a"),
            vec!["b".to_string(), "a".to_string()]
        );
    }
}
