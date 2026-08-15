use serde::{Deserialize, Serialize};

/// 附件实体类，附件是节点关联的加密文件，元数据存于数据库，密文存于附件目录。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    /// 附件 id（uuid），主键，同时是附件文件的文件名（`<id>.bin`）。
    pub id: String,
    /// 附件所属节点的 id（uuid）。
    pub node_id: String,
    /// 附件的原始文件名。
    pub file_name: String,
    /// 附件明文内容的大小，单位为字节。
    pub size: i64,
    /// 附件的导入时间，毫秒时间戳。
    pub create_time: i64,
    /// 是否逻辑删除。
    pub deleted: bool,
    /// 附件排序序号，同一节点下唯一，按此字段升序排列。
    pub sort_order: i64,
    /// 存储的附件内容是否经过压缩模块压缩。
    pub compressed: bool,
    /// 压缩算法名称和参数（JSON 字符串），由压缩模块生产和消费，其它模块不解释其内容；
    /// compressed 为 false 时为空字符串。
    pub compress_param: String,
}
