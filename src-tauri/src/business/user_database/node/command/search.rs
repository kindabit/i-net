use crate::business::user_database::node::response::NodeSearchResponse;
use crate::business::user_database::node::service;
use crate::error_code::ErrorCode;

/// 按原始查询字符串在所有画布中搜索节点。
///
/// # 参数
/// - `query`: 用户输入的原始查询字符串，可包含空格、中英文逗号、顿号、句号、分号等分隔符。
///
/// # 返回值
/// 返回搜索结果列表；查询串中不含任何有效关键词时返回空列表；若发生错误则返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_node_search(query: String) -> Result<Vec<NodeSearchResponse>, ErrorCode> {
    preprocess(query)
}

/// `user_database_node_search` 的 preprocess 函数：把原始查询串拆分为关键词列表后接入 service 层。
///
/// 拆分规则：以空白字符以及 `,` `，` `、` `。` `.` `;` `；` 为分隔符，
/// 每段去除首尾空白后非空才保留；拆分后无有效关键词时直接返回空列表，不访问数据库。
pub fn preprocess(query: String) -> Result<Vec<NodeSearchResponse>, ErrorCode> {
    let keywords = split_keywords(&query);
    if keywords.is_empty() {
        return Ok(Vec::new());
    }
    service::search(&keywords)
}

/// 把原始查询串拆分为关键词列表。
///
/// 以空白字符以及 `,` `，` `、` `。` `.` `;` `；` 为分隔符，
/// 每段去除首尾空白后非空才保留。
fn split_keywords(query: &str) -> Vec<String> {
    query
        .split(|c: char| c.is_whitespace() || matches!(c, ',' | '，' | '、' | '。' | '.' | ';' | '；'))
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 覆盖 user_database_node_search command 层 preprocess 与 split_keywords 的所有分支。
    #[test]
    fn test_user_database_node_search_all_branches() {
        // split_keywords 空字符串 → 返回空列表。
        assert!(split_keywords("").is_empty());

        // split_keywords 纯分隔符输入 → 各段 trim 后均为空，返回空列表。
        assert!(split_keywords(" , ，、。 .;；").is_empty());

        // split_keywords 各分隔符单独拆分。
        assert_eq!(split_keywords("a,b"), vec!["a", "b"]);
        assert_eq!(split_keywords("a，b"), vec!["a", "b"]);
        assert_eq!(split_keywords("a、b"), vec!["a", "b"]);
        assert_eq!(split_keywords("a。b"), vec!["a", "b"]);
        assert_eq!(split_keywords("a.b"), vec!["a", "b"]);
        assert_eq!(split_keywords("a;b"), vec!["a", "b"]);
        assert_eq!(split_keywords("a；b"), vec!["a", "b"]);
        assert_eq!(split_keywords("a b"), vec!["a", "b"]);

        // split_keywords 连续分隔符 → 空段被丢弃。
        assert_eq!(split_keywords("a,,b"), vec!["a", "b"]);
        assert_eq!(split_keywords("a,，b"), vec!["a", "b"]);
        assert_eq!(split_keywords("a  b"), vec!["a", "b"]);

        // split_keywords 首尾空白 trim。
        assert_eq!(split_keywords("  a b  "), vec!["a", "b"]);
        assert_eq!(split_keywords(" a , b "), vec!["a", "b"]);

        // split_keywords 混合分隔符拆分后关键词提取正确。
        assert_eq!(
            split_keywords("hello,world，foo、bar。baz.qux;quux；corge"),
            vec!["hello", "world", "foo", "bar", "baz", "qux", "quux", "corge"]
        );

        // preprocess 空字符串 → 返回空列表（不触达 service）。
        assert!(preprocess(String::new()).unwrap().is_empty());

        // preprocess 纯分隔符 → 返回空列表（不触达 service）。
        assert!(preprocess(" , ，、。 .;；".to_string()).unwrap().is_empty());
    }
}
