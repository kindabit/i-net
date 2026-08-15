use sha2::{Digest, Sha256};

use crate::error_code::ErrorCode;

/// 预处理偏好项名称：去除首尾空白字符，并校验名称非空。
///
/// # 参数
/// - `name`: 原始偏好项名称。
///
/// # 返回值
/// 返回清洗后的偏好项名称；名称为空时返回 `ErrorCode::EmptyPreferenceName`。
pub fn preprocess_preference_name(name: String) -> Result<String, ErrorCode> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(ErrorCode::EmptyPreferenceName);
    }
    Ok(name)
}

/// 预处理 registry 变量名称：去除首尾空白字符，并校验名称非空。
///
/// # 参数
/// - `name`: 原始 registry 变量名称。
///
/// # 返回值
/// 返回清洗后的 registry 变量名称；名称为空时返回 `ErrorCode::EmptyRegistryName`。
pub fn preprocess_registry_name(name: String) -> Result<String, ErrorCode> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(ErrorCode::EmptyRegistryName);
    }
    Ok(name)
}

/// 预处理用户数据库名称：去除首尾空白字符，并校验名称非空。
///
/// # 参数
/// - `name`: 原始用户数据库名称。
///
/// # 返回值
/// 返回清洗后的用户数据库名称；名称为空时返回 `ErrorCode::EmptyUserDatabaseName`。
pub fn preprocess_user_database_name(name: String) -> Result<String, ErrorCode> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(ErrorCode::EmptyUserDatabaseName);
    }
    Ok(name)
}

/// 预处理用户数据库 id：去除首尾空白字符，并校验 id 是标准小写连字符格式的 uuid
/// （与 `Uuid::new_v4().to_string()` 产生的格式一致）。
///
/// # 参数
/// - `id`: 原始用户数据库 id。
///
/// # 返回值
/// 返回清洗后的用户数据库 id；格式无效时返回 `ErrorCode::InvalidUserDatabaseId`。
pub fn preprocess_user_database_id(id: String) -> Result<String, ErrorCode> {
    let id = id.trim().to_string();
    // parse_str 接受无连字符、大写、花括号等宽松格式，
    // 而 to_string 永远输出标准格式，因此通过往返比对只接受标准格式的输入。
    match uuid::Uuid::parse_str(&id) {
        Ok(uuid) if uuid.to_string() == id => Ok(id),
        _ => Err(ErrorCode::InvalidUserDatabaseId { id }),
    }
}

/// 预处理画布 id：去除首尾空白字符，并校验 id 是标准小写连字符格式的 uuid
/// （与 `Uuid::new_v4().to_string()` 产生的格式一致）。
///
/// # 参数
/// - `id`: 原始画布 id。
///
/// # 返回值
/// 返回清洗后的画布 id；格式无效时返回 `ErrorCode::InvalidCanvasId`。
pub fn preprocess_canvas_id(id: String) -> Result<String, ErrorCode> {
    let id = id.trim().to_string();
    // parse_str 接受无连字符、大写、花括号等宽松格式，
    // 而 to_string 永远输出标准格式，因此通过往返比对只接受标准格式的输入。
    match uuid::Uuid::parse_str(&id) {
        Ok(uuid) if uuid.to_string() == id => Ok(id),
        _ => Err(ErrorCode::InvalidCanvasId { id }),
    }
}

/// 预处理节点 id：去除首尾空白字符，并校验 id 是标准小写连字符格式的 uuid
/// （与 `Uuid::new_v4().to_string()` 产生的格式一致）。
///
/// # 参数
/// - `id`: 原始节点 id。
///
/// # 返回值
/// 返回清洗后的节点 id；格式无效时返回 `ErrorCode::InvalidNodeId`。
pub fn preprocess_node_id(id: String) -> Result<String, ErrorCode> {
    let id = id.trim().to_string();
    // parse_str 接受无连字符、大写、花括号等宽松格式，
    // 而 to_string 永远输出标准格式，因此通过往返比对只接受标准格式的输入。
    match uuid::Uuid::parse_str(&id) {
        Ok(uuid) if uuid.to_string() == id => Ok(id),
        _ => Err(ErrorCode::InvalidNodeId { id }),
    }
}

/// 预处理附件 id：去除首尾空白字符，并校验 id 是标准小写连字符格式的 uuid
/// （与 `Uuid::new_v4().to_string()` 产生的格式一致）。
///
/// # 参数
/// - `id`: 原始附件 id。
///
/// # 返回值
/// 返回清洗后的附件 id；格式无效时返回 `ErrorCode::InvalidAttachmentId`。
pub fn preprocess_attachment_id(id: String) -> Result<String, ErrorCode> {
    let id = id.trim().to_string();
    // parse_str 接受无连字符、大写、花括号等宽松格式，
    // 而 to_string 永远输出标准格式，因此通过往返比对只接受标准格式的输入。
    match uuid::Uuid::parse_str(&id) {
        Ok(uuid) if uuid.to_string() == id => Ok(id),
        _ => Err(ErrorCode::InvalidAttachmentId { id }),
    }
}

/// 预处理边 id：去除首尾空白字符，并校验 id 是标准小写连字符格式的 uuid
/// （与 `Uuid::new_v4().to_string()` 产生的格式一致）。
///
/// # 参数
/// - `id`: 原始边 id。
///
/// # 返回值
/// 返回清洗后的边 id；格式无效时返回 `ErrorCode::InvalidEdgeId`。
pub fn preprocess_edge_id(id: String) -> Result<String, ErrorCode> {
    let id = id.trim().to_string();
    // parse_str 接受无连字符、大写、花括号等宽松格式，
    // 而 to_string 永远输出标准格式，因此通过往返比对只接受标准格式的输入。
    match uuid::Uuid::parse_str(&id) {
        Ok(uuid) if uuid.to_string() == id => Ok(id),
        _ => Err(ErrorCode::InvalidEdgeId { id }),
    }
}

/// 预处理边标题：去除首尾空白字符。
///
/// # 参数
/// - `title`: 原始边标题。
///
/// # 返回值
/// 返回清洗后的边标题。
pub fn preprocess_edge_title(title: String) -> Result<String, ErrorCode> {
    Ok(title.trim().to_string())
}

/// 预处理边详情：去除首尾空白字符。
///
/// # 参数
/// - `description`: 原始边详情。
///
/// # 返回值
/// 返回清洗后的边详情。
pub fn preprocess_edge_description(description: String) -> Result<String, ErrorCode> {
    Ok(description.trim().to_string())
}

/// 预处理画布名称：去除首尾空白字符，并校验名称非空。
///
/// # 参数
/// - `name`: 原始画布名称。
///
/// # 返回值
/// 返回清洗后的画布名称；名称为空时返回 `ErrorCode::EmptyCanvasName`。
pub fn preprocess_canvas_name(name: String) -> Result<String, ErrorCode> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(ErrorCode::EmptyCanvasName);
    }
    Ok(name)
}

/// 预处理节点连接桩：去除首尾空白字符，并校验连接桩是
/// 上下左右四个连接桩之一（"top" / "right" / "bottom" / "left"）。
///
/// # 参数
/// - `port`: 原始节点连接桩。
///
/// # 返回值
/// 返回清洗后的节点连接桩；连接桩无效时返回 `ErrorCode::InvalidNodePort`。
pub fn preprocess_node_port(port: String) -> Result<String, ErrorCode> {
    let port = port.trim().to_string();
    match port.as_str() {
        "top" | "right" | "bottom" | "left" => Ok(port),
        _ => Err(ErrorCode::InvalidNodePort { port }),
    }
}

/// 预处理文件路径：去除首尾空白字符，并校验路径非空。
///
/// # 参数
/// - `path`: 原始文件路径。
///
/// # 返回值
/// 返回清洗后的文件路径；路径为空时返回 `ErrorCode::EmptyFilePath`。
pub fn preprocess_file_path(path: String) -> Result<String, ErrorCode> {
    let path = path.trim().to_string();
    if path.is_empty() {
        return Err(ErrorCode::EmptyFilePath);
    }
    Ok(path)
}

/// 预处理密码：校验密码非空，然后将其哈希为 32 字节密钥。
///
/// # 参数
/// - `password`: 原始密码。
///
/// # 返回值
/// 返回密码哈希得到的 32 字节密钥；密码为空时返回 `ErrorCode::EmptyPassword`。
pub fn preprocess_password(password: String) -> Result<[u8; 32], ErrorCode> {
    if password.is_empty() {
        return Err(ErrorCode::EmptyPassword);
    }
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    Ok(hasher.finalize().into())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 覆盖 preprocess_util 模块所有预处理函数的成功与失败路径。
    #[test]
    fn test_preprocess_util_all_functions() {
        // preprocess_preference_name 成功路径：去除首尾空白字符。
        assert_eq!(
            preprocess_preference_name("  theme  ".to_string()).unwrap(),
            "theme"
        );

        // preprocess_preference_name 失败路径：空名称或纯空白名称返回 EmptyPreferenceName。
        assert!(matches!(
            preprocess_preference_name("".to_string()),
            Err(ErrorCode::EmptyPreferenceName)
        ));
        assert!(matches!(
            preprocess_preference_name(" \t\n ".to_string()),
            Err(ErrorCode::EmptyPreferenceName)
        ));

        // preprocess_user_database_name 成功路径：去除首尾空白字符。
        assert_eq!(
            preprocess_user_database_name("  my-database  ".to_string()).unwrap(),
            "my-database"
        );

        // preprocess_user_database_name 失败路径：空名称或纯空白名称返回 EmptyUserDatabaseName。
        assert!(matches!(
            preprocess_user_database_name("".to_string()),
            Err(ErrorCode::EmptyUserDatabaseName)
        ));
        assert!(matches!(
            preprocess_user_database_name("  ".to_string()),
            Err(ErrorCode::EmptyUserDatabaseName)
        ));

        // preprocess_user_database_id 成功路径：标准小写连字符格式的 uuid 原样返回，首尾空白被去除。
        let id = uuid::Uuid::new_v4().to_string();
        assert_eq!(
            preprocess_user_database_id(format!("  {id}  ")).unwrap(),
            id
        );

        // preprocess_user_database_id 失败路径：非 uuid 输入返回 InvalidUserDatabaseId，且错误中携带 trim 后的 id。
        match preprocess_user_database_id("  not-a-uuid  ".to_string()) {
            Err(ErrorCode::InvalidUserDatabaseId { id }) => assert_eq!(id, "not-a-uuid"),
            other => panic!("expected InvalidUserDatabaseId, got {other:?}"),
        }

        // preprocess_user_database_id 失败路径：空 id 返回 InvalidUserDatabaseId。
        assert!(matches!(
            preprocess_user_database_id("".to_string()),
            Err(ErrorCode::InvalidUserDatabaseId { .. })
        ));

        // preprocess_user_database_id 失败路径：parse_str 能接受的宽松格式（大写、无连字符、花括号）一律拒绝。
        assert!(matches!(
            preprocess_user_database_id(id.to_uppercase()),
            Err(ErrorCode::InvalidUserDatabaseId { .. })
        ));
        assert!(matches!(
            preprocess_user_database_id(id.replace('-', "")),
            Err(ErrorCode::InvalidUserDatabaseId { .. })
        ));
        assert!(matches!(
            preprocess_user_database_id(format!("{{{id}}}")),
            Err(ErrorCode::InvalidUserDatabaseId { .. })
        ));

        // preprocess_password 失败路径：空密码返回 EmptyPassword。
        assert!(matches!(
            preprocess_password("".to_string()),
            Err(ErrorCode::EmptyPassword)
        ));

        // preprocess_password 成功路径：非空密码哈希为其 sha256 值（"abc" 的标准测试向量）。
        let key = preprocess_password("abc".to_string()).unwrap();
        let hex: String = key.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(
            hex,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );

        // preprocess_password 属性：哈希结果确定（同一密码两次哈希一致），且不同密码哈希不同。
        assert_eq!(
            preprocess_password("same".to_string()).unwrap(),
            preprocess_password("same".to_string()).unwrap()
        );
        assert_ne!(
            preprocess_password("one".to_string()).unwrap(),
            preprocess_password("two".to_string()).unwrap()
        );

        // preprocess_password 成功路径：纯空白密码视为非空（不做 trim），正常返回哈希。
        assert!(preprocess_password(" ".to_string()).is_ok());

        // preprocess_canvas_id 成功路径：标准小写连字符格式的 uuid 原样返回，首尾空白被去除。
        let id = uuid::Uuid::new_v4().to_string();
        assert_eq!(preprocess_canvas_id(format!("  {id}  ")).unwrap(), id);

        // preprocess_canvas_id 失败路径：非 uuid 输入与宽松格式一律返回 InvalidCanvasId。
        assert!(matches!(
            preprocess_canvas_id("not-a-uuid".to_string()),
            Err(ErrorCode::InvalidCanvasId { .. })
        ));
        assert!(matches!(
            preprocess_canvas_id(id.to_uppercase()),
            Err(ErrorCode::InvalidCanvasId { .. })
        ));
        assert!(matches!(
            preprocess_canvas_id(id.replace('-', "")),
            Err(ErrorCode::InvalidCanvasId { .. })
        ));

        // preprocess_node_id 成功路径：标准小写连字符格式的 uuid 原样返回，首尾空白被去除。
        assert_eq!(preprocess_node_id(format!("  {id}  ")).unwrap(), id);

        // preprocess_node_id 失败路径：非 uuid 输入与宽松格式一律返回 InvalidNodeId。
        assert!(matches!(
            preprocess_node_id("not-a-uuid".to_string()),
            Err(ErrorCode::InvalidNodeId { .. })
        ));
        assert!(matches!(
            preprocess_node_id(format!("{{{id}}}")),
            Err(ErrorCode::InvalidNodeId { .. })
        ));

        // preprocess_edge_id 成功路径：标准小写连字符格式的 uuid 原样返回，首尾空白被去除。
        assert_eq!(preprocess_edge_id(format!("  {id}  ")).unwrap(), id);

        // preprocess_edge_id 失败路径：非 uuid 输入与宽松格式一律返回 InvalidEdgeId。
        assert!(matches!(
            preprocess_edge_id("not-a-uuid".to_string()),
            Err(ErrorCode::InvalidEdgeId { .. })
        ));
        assert!(matches!(
            preprocess_edge_id(id.to_uppercase()),
            Err(ErrorCode::InvalidEdgeId { .. })
        ));

        // preprocess_edge_title 成功路径：去除首尾空白字符。
        assert_eq!(
            preprocess_edge_title("  test title  ".to_string()).unwrap(),
            "test title"
        );

        // preprocess_edge_title 成功路径：空字符串返回空字符串。
        assert_eq!(
            preprocess_edge_title("".to_string()).unwrap(),
            ""
        );

        // preprocess_edge_description 成功路径：去除首尾空白字符。
        assert_eq!(
            preprocess_edge_description("  test description  ".to_string()).unwrap(),
            "test description"
        );

        // preprocess_edge_description 成功路径：空字符串返回空字符串。
        assert_eq!(
            preprocess_edge_description("".to_string()).unwrap(),
            ""
        );

        // preprocess_canvas_name 成功路径：去除首尾空白字符。
        assert_eq!(
            preprocess_canvas_name("  my-canvas  ".to_string()).unwrap(),
            "my-canvas"
        );

        // preprocess_canvas_name 失败路径：空名称或纯空白名称返回 EmptyCanvasName。
        assert!(matches!(
            preprocess_canvas_name("".to_string()),
            Err(ErrorCode::EmptyCanvasName)
        ));
        assert!(matches!(
            preprocess_canvas_name("  ".to_string()),
            Err(ErrorCode::EmptyCanvasName)
        ));

        // preprocess_node_port 成功路径：上下左右四个连接桩均被接受，首尾空白被去除。
        for port in ["top", "right", "bottom", "left"] {
            assert_eq!(preprocess_node_port(format!("  {port}  ")).unwrap(), port);
        }

        // preprocess_node_port 失败路径：其它字符串返回 InvalidNodePort，且错误中携带 trim 后的连接桩。
        match preprocess_node_port("  middle  ".to_string()) {
            Err(ErrorCode::InvalidNodePort { port }) => assert_eq!(port, "middle"),
            other => panic!("expected InvalidNodePort, got {other:?}"),
        }
        assert!(matches!(
            preprocess_node_port("".to_string()),
            Err(ErrorCode::InvalidNodePort { .. })
        ));

        // preprocess_attachment_id 成功路径：标准小写连字符格式的 uuid 原样返回，首尾空白被去除。
        let id = uuid::Uuid::new_v4().to_string();
        assert_eq!(
            preprocess_attachment_id(format!("  {id}  ")).unwrap(),
            id
        );

        // preprocess_attachment_id 失败路径：非 uuid 输入返回 InvalidAttachmentId，且错误中携带 trim 后的 id。
        match preprocess_attachment_id("  not-a-uuid  ".to_string()) {
            Err(ErrorCode::InvalidAttachmentId { id }) => assert_eq!(id, "not-a-uuid"),
            other => panic!("expected InvalidAttachmentId, got {other:?}"),
        }

        // preprocess_attachment_id 失败路径：空 id 返回 InvalidAttachmentId。
        assert!(matches!(
            preprocess_attachment_id("".to_string()),
            Err(ErrorCode::InvalidAttachmentId { .. })
        ));

        // preprocess_attachment_id 失败路径：parse_str 能接受的宽松格式（大写、无连字符、花括号）一律拒绝。
        assert!(matches!(
            preprocess_attachment_id(id.to_uppercase()),
            Err(ErrorCode::InvalidAttachmentId { .. })
        ));
        assert!(matches!(
            preprocess_attachment_id(id.replace('-', "")),
            Err(ErrorCode::InvalidAttachmentId { .. })
        ));
        assert!(matches!(
            preprocess_attachment_id(format!("{{{id}}}")),
            Err(ErrorCode::InvalidAttachmentId { .. })
        ));

        // preprocess_file_path 成功路径：去除首尾空白字符。
        assert_eq!(
            preprocess_file_path("  C:\\data\\a.pdf  ".to_string()).unwrap(),
            "C:\\data\\a.pdf"
        );

        // preprocess_file_path 失败路径：空路径或纯空白路径返回 EmptyFilePath。
        assert!(matches!(
            preprocess_file_path("".to_string()),
            Err(ErrorCode::EmptyFilePath)
        ));
        assert!(matches!(
            preprocess_file_path(" \t\n ".to_string()),
            Err(ErrorCode::EmptyFilePath)
        ));
    }
}
