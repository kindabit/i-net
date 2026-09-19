use serde::Serialize;

/// 目录内的单个条目。
#[derive(Serialize)]
pub struct DirectoryEntryVO {
    /// 条目名称
    pub name: String,
    /// 条目的完整路径
    pub path: String,
    /// 是否为目录
    pub is_directory: bool,
}

/// 一次目录读取的结果。
#[derive(Serialize)]
pub struct DirectoryListingVO {
    /// 当前目录的完整路径
    pub path: String,
    /// 父目录的完整路径；当前目录为根目录时为 None
    pub parent: Option<String>,
    /// 目录内的条目列表（目录优先、名称升序）
    pub entries: Vec<DirectoryEntryVO>,
}
