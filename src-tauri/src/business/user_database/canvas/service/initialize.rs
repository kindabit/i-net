use crate::business::user_database::canvas::dao;
use crate::business::user_database::entity::{Canvas, ROOT_CANVAS_NAME};
use crate::business::user_database::state;
use crate::error_code::ErrorCode;

/// 初始化 canvas 子业务模块：新建 canvas 表，并插入根画布。
/// 根画布名称为常量 "root"，没有父画布，不产生日志。
///
/// # 返回值
/// 成功时返回 `Ok(())`；若发生错误则返回对应的 `ErrorCode`。
pub fn initialize() -> Result<(), ErrorCode> {
    let connection = state::lock_connection();
    dao::create_table(&connection)?;
    let root = Canvas {
        id: uuid::Uuid::new_v4().to_string(),
        parent_id: None,
        name: ROOT_CANVAS_NAME.to_string(),
        x: 0.0,
        y: 0.0,
        deleted: false,
        color: String::new(),
    };
    dao::insert(&connection, &root)?;
    Ok(())
}
