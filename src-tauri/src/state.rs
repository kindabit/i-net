use std::path::{Component, Path as StdPath, PathBuf};
use std::sync::{LazyLock, Mutex};

use crate::error_code::ErrorCode;

/// 全局路径状态对象。
///
/// tauri 不再托管状态，路径状态由本静态变量手动管理：
/// 程序启动时通过 [`set_path`] 写入，此后各处通过 [`path()`] 读取。
static PATH: LazyLock<Mutex<Option<Path>>> = LazyLock::new(|| Mutex::new(None));

/// 将路径状态对象写入全局状态。
///
/// 程序启动时调用一次；测试可以用它切换到各自的数据目录。
///
/// # 参数
/// - `path`: 要存储的路径状态对象。
///
/// # 返回值
/// 无。
pub fn set_path(path: Path) {
    *PATH.lock().expect("path state lock is poisoned") = Some(path);
}

/// 获取全局路径状态对象的克隆。
///
/// # 返回值
/// 返回路径状态对象；若尚未初始化则 panic。
pub fn path() -> Path {
    PATH.lock()
        .expect("path state lock is poisoned")
        .clone()
        .expect("path state is not initialized")
}

/// 应用程序相关的路径集合。
#[derive(Debug, Clone)]
pub struct Path {
    /// 应用程序数据根目录。
    pub data_directory: PathBuf,
    /// 日志文件存放目录。
    pub log_directory: PathBuf,
    /// 用户数据库集合目录，用于存放全部用户数据库。
    pub user_database_set_directory: PathBuf,
    /// 偏好数据库文件路径。
    pub preference_database_file: PathBuf,
    /// 元数据数据库文件路径。
    pub metadata_database_file: PathBuf,
}

impl Path {
    /// 获取指定用户数据库的目录路径。
    ///
    /// # 参数
    /// - `user_uuid`: 用户数据库 UUID。
    ///
    /// # 返回值
    /// 返回该用户数据库对应的目录路径。
    pub fn user_database_directory(&self, user_uuid: &str) -> PathBuf {
        self.user_database_set_directory.join(user_uuid)
    }

    /// 获取指定用户数据库文件路径。
    ///
    /// # 参数
    /// - `user_uuid`: 用户数据库 UUID。
    ///
    /// # 返回值
    /// 返回该用户数据库对应的 SQLite 文件路径。
    pub fn user_database_file(&self, user_uuid: &str) -> PathBuf {
        self.user_database_directory(user_uuid)
            .join("user_database.sqlite")
    }

    /// 获取指定用户数据库的附件目录路径。
    ///
    /// # 参数
    /// - `user_uuid`: 用户数据库 UUID。
    ///
    /// # 返回值
    /// 返回该用户数据库对应的附件目录路径。
    pub fn user_attachment_directory(&self, user_uuid: &str) -> PathBuf {
        self.user_database_directory(user_uuid).join("attachment")
    }

    /// 获取指定用户数据库中某个附件的文件路径。
    ///
    /// # 参数
    /// - `user_uuid`: 用户数据库 UUID。
    /// - `attachment_uuid`: 附件 UUID。
    ///
    /// # 返回值
    /// 返回该附件对应的二进制文件路径。
    pub fn user_attachment_file(&self, user_uuid: &str, attachment_uuid: &str) -> PathBuf {
        self.user_attachment_directory(user_uuid)
            .join(format!("{}.bin", attachment_uuid))
    }

    /// 校验目标路径不在应用数据目录内，防止导出等写操作覆盖用户数据库文件或附件文件。
    /// 比较前对两端路径做词法规范化（不访问文件系统），再做组件级前缀比较。
    ///
    /// # 参数
    /// - `target_path`: 待校验的目标文件路径。
    ///
    /// # 返回值
    /// 目标路径在数据目录外时返回 `Ok(())`；
    /// 等于数据目录或位于其内部时返回 `ErrorCode::InvalidExportTargetPath`。
    pub fn ensure_outside_data_directory(&self, target_path: &str) -> Result<(), ErrorCode> {
        let target = Self::normalize_components(StdPath::new(target_path));
        let data_directory = Self::normalize_components(&self.data_directory);
        if target.len() >= data_directory.len() && target[..data_directory.len()] == data_directory[..]
        {
            return Err(ErrorCode::InvalidExportTargetPath {
                path: target_path.to_string(),
            });
        }
        Ok(())
    }

    /// 对路径做词法规范化，返回统一小写的组件序列（小写化兼容 Windows 大小写不敏感的文件系统）：
    /// 跳过 `.` 组件，`..` 弹出栈顶普通组件（栈顶为根目录或盘符前缀时 `..` 无法上穿，直接丢弃），
    /// 其余组件入栈。不调用 `canonicalize`，因此目标文件不存在时同样适用。
    ///
    /// # 参数
    /// - `path`: 待规范化的路径。
    ///
    /// # 返回值
    /// 返回规范化后的路径组件序列（每个组件已转小写）。
    fn normalize_components(path: &StdPath) -> Vec<String> {
        let mut stack: Vec<String> = Vec::new();
        // 与 stack 一一对应，标记每个入栈组件是否为可被 `..` 弹出的普通组件。
        let mut is_normal: Vec<bool> = Vec::new();
        for component in path.components() {
            match component {
                Component::Prefix(_) | Component::RootDir => {
                    stack.push(component.as_os_str().to_string_lossy().to_lowercase());
                    is_normal.push(false);
                }
                Component::CurDir => {}
                Component::ParentDir => {
                    if is_normal.last() == Some(&true) {
                        stack.pop();
                        is_normal.pop();
                    }
                }
                Component::Normal(part) => {
                    stack.push(part.to_string_lossy().to_lowercase());
                    is_normal.push(true);
                }
            }
        }
        stack
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test;

    /// 覆盖 normalize_components 的全部词法规范化规则（纯函数，不依赖全局状态）。
    #[test]
    fn test_normalize_components_all_rules() {
        // `.` 组件被跳过。
        assert_eq!(
            Path::normalize_components(StdPath::new("a/./b")),
            vec!["a".to_string(), "b".to_string()]
        );
        // `..` 弹出栈顶普通组件。
        assert_eq!(
            Path::normalize_components(StdPath::new("a/b/../c")),
            vec!["a".to_string(), "c".to_string()]
        );
        // 栈顶为根目录时 `..` 无法上穿，与无 `..` 形式等价。
        assert_eq!(
            Path::normalize_components(StdPath::new("/../a")),
            Path::normalize_components(StdPath::new("/a"))
        );
        // 连续 `..` 超出栈深时多余部分被丢弃。
        assert_eq!(
            Path::normalize_components(StdPath::new("a/../../b")),
            vec!["b".to_string()]
        );
        // 组件统一转小写以兼容大小写不敏感的文件系统。
        assert_eq!(
            Path::normalize_components(StdPath::new("Ab/Cd")),
            vec!["ab".to_string(), "cd".to_string()]
        );
    }

    /// 覆盖 ensure_outside_data_directory 对各类目标路径形态的放行/拒绝判定。
    #[test]
    fn test_ensure_outside_data_directory_all_paths() {
        let _guard = test::acquire_test_lock();
        let path = test::create_test_path();
        crate::state::set_path(path.clone());
        let data_dir = path.data_directory.clone();
        let dir_name = data_dir.file_name().unwrap().to_string_lossy().into_owned();
        let parent = data_dir.parent().unwrap().to_path_buf();
        let state = crate::state::path();

        // 失败路径：目标就是数据目录本身。
        assert!(matches!(
            state.ensure_outside_data_directory(&data_dir.to_string_lossy()),
            Err(ErrorCode::InvalidExportTargetPath { .. })
        ));
        // 失败路径：数据目录内的数据库文件。
        assert!(matches!(
            state.ensure_outside_data_directory(
                &data_dir.join("user_database.sqlite").to_string_lossy()
            ),
            Err(ErrorCode::InvalidExportTargetPath { .. })
        ));
        // 失败路径：数据目录内更深层的附件文件。
        assert!(matches!(
            state.ensure_outside_data_directory(
                &data_dir.join("attachment").join("a.bin").to_string_lossy()
            ),
            Err(ErrorCode::InvalidExportTargetPath { .. })
        ));
        // 失败路径：数据目录内的导出文件。
        assert!(matches!(
            state.ensure_outside_data_directory(&data_dir.join("export.md").to_string_lossy()),
            Err(ErrorCode::InvalidExportTargetPath { .. })
        ));
        // 失败路径：用 `..` 绕出数据目录后再绕回，规范化后仍在目录内。
        let roundabout = data_dir
            .join("sub")
            .join("..")
            .join("..")
            .join(&dir_name)
            .join("user_database.sqlite");
        assert!(matches!(
            state.ensure_outside_data_directory(&roundabout.to_string_lossy()),
            Err(ErrorCode::InvalidExportTargetPath { .. })
        ));
        // 失败路径：路径整体大写化的大小写变体。
        let upper = data_dir
            .join("export.md")
            .to_string_lossy()
            .to_uppercase();
        assert!(matches!(
            state.ensure_outside_data_directory(&upper),
            Err(ErrorCode::InvalidExportTargetPath { .. })
        ));
        // 成功路径：数据目录的兄弟目录（经 `..` 规范化后落在数据目录之外）。
        let sibling = data_dir
            .join("..")
            .join("not-the-data-directory")
            .join("file.md");
        assert!(state.ensure_outside_data_directory(&sibling.to_string_lossy()).is_ok());
        // 成功路径：名字前缀相似但并非数据目录的目录（组件级比较不会误判）。
        let lookalike = parent.join(format!("{dir_name}2")).join("file.md");
        assert!(state.ensure_outside_data_directory(&lookalike.to_string_lossy()).is_ok());
        // 成功路径：相对路径（首组件与数据目录的根/盘符不同，天然在目录外）。
        assert!(state.ensure_outside_data_directory("some/relative/file.md").is_ok());

        test::cleanup(&path);
    }
}
