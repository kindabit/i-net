use crate::business::user_database::attachment::service;
use crate::business::user_database::attachment::vo::AttachmentVO;
use crate::error_code::ErrorCode;
use crate::util::preprocess_util;

/// 新建文本附件：以指定文件名在指定节点下创建一个内容为空的附件，供用户随后写入文本内容。
///
/// # 参数
/// - `node_id`: 附件所属节点的 id。
/// - `file_name`: 附件文件名。
///
/// # 返回值
/// 返回新建的附件值对象；发生错误时返回对应的 `ErrorCode`。
#[tauri::command]
pub fn user_database_attachment_create(
    node_id: String,
    file_name: String,
) -> Result<AttachmentVO, ErrorCode> {
    preprocess(node_id, file_name)
}

/// `user_database_attachment_create` 的 preprocess 函数：校验 node_id 与 file_name 后接入 service 层的 create 函数。
pub fn preprocess(node_id: String, file_name: String) -> Result<AttachmentVO, ErrorCode> {
    let node_id = preprocess_util::preprocess_node_id(node_id)?;
    let file_name = preprocess_util::preprocess_file_name(file_name)?;
    service::create(&node_id, &file_name)
}
