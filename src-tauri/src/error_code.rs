use serde::{Deserialize, Serialize};

use crate::common::data_version::entity::DataVersion;

/// 应用内部错误码枚举，用于向前端返回可识别的错误信息。
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "variant", content = "data")]
pub enum ErrorCode {
    /// 附件大小超过上限，包含上限（单位 MB）。
    AttachmentTooLarge { max: u64 },
    /// 两个附件不属于同一节点，无法交换排序位置。
    AttachmentNotInSameNode,
    /// 附件已逻辑删除，无法进行操作。
    AttachmentDeleted,
    /// 画布名称已经存在，包含重复的画布名称。
    CanvasNameAlreadyExists { name: String },
    /// 数据库操作失败，包含详细错误信息，仅限 dao 层使用。
    DatabaseError { detail: String },
    /// 数据库名称已经存在，包含重复的数据库名称。
    DatabaseNameAlreadyExists { name: String },
    /// 数据库必须先归档才能删除。
    DatabaseMustBeArchivedBeforeDelete,
    /// 数据版本不匹配，包含期望的版本和实际的版本。
    DataVersionMismatch {
        expected: DataVersion,
        actual: DataVersion,
    },
    /// 字典条目 id 重复，包含重复的字典条目 id。
    DuplicateDictionaryId { id: String },
    /// 字段名称重复，包含重复的字段名称。
    DuplicateNodeFieldName { name: String },
    /// 相同源节点和目标节点的边已经存在。
    EdgeAlreadyExists,
    /// 删除该边会物理删除影子节点并使所列节点失去连接，包含受影响节点的标题列表。
    EdgeDeleteDisconnectsNodes { nodes: Vec<String> },
    /// 新建该边会在画布内形成环。
    EdgeWouldFormCycle,
    /// 画布名称为空。
    EmptyCanvasName,
    /// 文件路径为空。
    EmptyFilePath,
    /// 字典条目值为空。
    EmptyDictionaryValue,
    /// 节点字段名称为空。
    EmptyNodeFieldName,
    /// 偏好项名称为空。
    EmptyPreferenceName,
    /// registry 变量名称为空。
    EmptyRegistryName,
    /// 模板名称为空。
    EmptyTemplateName,
    /// 密码为空。
    EmptyPassword,
    /// 用户数据库名称为空。
    EmptyUserDatabaseName,
    /// 反序列化日志行为失败。
    FailToDeserializeAction,
    /// 反序列化数据库失败，包含详细错误信息，仅限 connection 业务模块使用。
    FailToDeserializeDatabase { detail: String },
    /// 反序列化节点字段数据失败，包含详细错误信息。
    FailToDeserializeNodeFieldValue { detail: String },
    /// 打开数据库连接失败，包含详细错误信息，仅限 connection 业务模块使用。
    FailToOpenConnection { detail: String },
    /// 序列化日志行为失败。
    FailToSerializeAction,
    /// 序列化数据库失败，包含详细错误信息，仅限 connection 业务模块使用。
    FailToSerializeDatabase { detail: String },
    /// 压缩失败，包含详细错误信息，仅限压缩模块使用。
    FailToCompress { detail: String },
    /// 创建目录失败，包含目标路径和详细错误信息。
    FailToCreateDirectory { path: String, detail: String },
    /// 加密失败，包含详细错误信息。
    FailToEncrypt { detail: String },
    /// 解密失败，包含详细错误信息。
    FailToDecrypt { detail: String },
    /// 解压缩失败，包含详细错误信息，仅限压缩模块使用。
    FailToDecompress { detail: String },
    /// 读取目录失败，包含目标路径和详细错误信息。
    FailToReadDirectory { path: String, detail: String },
    /// 读取文件失败，包含目标路径和详细错误信息。
    FailToReadFile { path: String, detail: String },
    /// 删除文件失败，包含目标路径和详细错误信息。
    FailToRemoveFile { path: String, detail: String },
    /// 删除目录失败，包含目标路径和详细错误信息。
    FailToRemoveDirectory { path: String, detail: String },
    /// 判断文件或目录是否存在失败，包含目标路径和详细错误信息。
    FailToTryExists { path: String, detail: String },
    /// 写入文件失败，包含目标路径和详细错误信息。
    FailToWriteFile { path: String, detail: String },
    /// 该字段类型不支持引用字典，包含字段类型。
    FieldTypeNotSupportDictionary { field_type: String },
    /// 附件 id 无效，包含附件 id。
    InvalidAttachmentId { id: String },
    /// 密文无效。
    InvalidCiphertext,
    /// 画布 id 无效，包含画布 id。
    InvalidCanvasId { id: String },
    /// 字典条目 id 无效，包含字典条目 id。
    InvalidDictionaryId { id: String },
    /// 边 id 无效，包含边 id。
    InvalidEdgeId { id: String },
    /// 导出目标路径无效，包含目标路径。
    InvalidExportTargetPath { path: String },
    /// 导出模式字符串无效，包含原始模式字符串。
    InvalidExportMode { mode: String },
    /// 节点 id 无效，包含节点 id。
    InvalidNodeId { id: String },
    /// 节点字段类型不存在，包含字段类型。
    InvalidNodeFieldType { field_type: String },
    /// 节点字段类型配置无效，包含字段类型和详细错误信息。
    InvalidNodeFieldTypeConfig { field_type: String, detail: String },
    /// 节点连接桩无效，包含连接桩。
    InvalidNodePort { port: String },
    /// 用户在系统对话框中选择的路径无法转换为本地文件系统路径，包含详细错误信息。
    InvalidPath { detail: String },
    /// 影子节点的连线方向不合法或影子节点不允许与画布节点相连。
    InvalidShadowEdge,
    /// 模板 id 无效，包含模板 id。
    InvalidTemplateId { id: String },
    /// 用户数据库 id 无效，包含数据库 id。
    InvalidUserDatabaseId { id: String },
    /// 数据版本表内存在多行数据。
    MultipleDataVersion,
    /// 不存在指定 id 的附件，包含附件 id。
    NoAttachmentWithSuchId { id: String },
    /// 不存在指定 id 的画布，包含画布 id。
    NoCanvasWithSuchId { id: String },
    /// 不存在指定 id 的数据库，包含数据库 id。
    NoDatabaseWithSuchId { id: String },
    /// 数据版本表内没有数据。
    NoDataVersion,
    /// 不存在指定 id 的字典条目，包含字典条目 id。
    NoDictionaryEntryWithSuchId { id: String },
    /// 不存在指定 id 的边，包含边 id。
    NoEdgeWithSuchId { id: String },
    /// 不存在指定 id 的节点，包含节点 id。
    NoNodeWithSuchId { id: String },
    /// 不存在指定 id 的模板，包含模板 id。
    NoTemplateWithSuchId { id: String },
    /// 节点字段值的底层数据类型与字段类型声明的不匹配，包含字段类型、期望的底层数据类型和实际的底层数据类型。
    NodeFieldValueKindMismatch { field_type: String, expected: String, actual: String },
    /// 节点字段值未通过校验，包含字段名称。
    NodeFieldValueValidationFailed { name: String },
    /// 该操作不允许作用于影子节点。
    NodeIsShadow,
    /// 根画布不可删除。
    RootCanvasCannotBeDeleted,
    /// 模板名称已经存在，包含重复的模板名称。
    TemplateNameAlreadyExists { name: String },
    /// 用户数据库未打开。
    UserDatabaseNotOpen,
    /// 剪贴板操作失败，包含详细错误信息。
    ClipboardError { detail: String },
    /// 备份文件格式无效（magic 不匹配、版本不支持、Header 损坏等），包含详细错误信息。
    InvalidBackupFile { detail: String },
    /// 备份文件版本不兼容，包含读取到的版本号。
    UnsupportedBackupVersion { version: u16 },
    /// 备份文件中损坏的 shard 数超过可恢复上限，包含丢失数与可恢复上限。
    BackupTooManyShardsLost { lost: usize, recoverable: usize },
    /// 备份打包失败，包含详细错误信息。
    FailToPackBackup { detail: String },
    /// 备份解包失败，包含详细错误信息。
    FailToUnpackBackup { detail: String },
}
