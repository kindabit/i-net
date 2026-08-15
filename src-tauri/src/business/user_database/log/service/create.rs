use crate::business::user_database::entity::{Action, Log};
use crate::business::user_database::log::dao;
use crate::business::user_database::state;
use crate::error_code::ErrorCode;
use crate::security::aes;
use crate::util::time_util;

/// 向日志表插入一条日志：行为序列化为 { variant, data } 结构后，
/// variant 名明文存入 action 列，data JSON 使用当前打开的用户数据库的密钥加密后存入 detail 列。
/// 这就是向日志表执行插入操作的函数，其本身不会产生日志。
///
/// # 参数
/// - `object_id`: 被操作对象的 id。
/// - `action`: 行为（含数据载荷）。
///
/// # 返回值
/// 成功时返回 `Ok(())`；行为序列化失败时返回 `ErrorCode::FailToSerializeAction`，
/// 发生其他错误时返回对应的 `ErrorCode`。
pub fn create(object_id: &str, action: Action) -> Result<(), ErrorCode> {
    let value = serde_json::to_value(&action).map_err(|_| ErrorCode::FailToSerializeAction)?;
    // Action 以内部标签序列化，结果必然存在 variant 字符串字段；取不到属 serde 内部异常。
    let variant = value
        .get("variant")
        .and_then(|variant| variant.as_str())
        .ok_or(ErrorCode::FailToSerializeAction)?
        .to_string();
    // 无 data 的单元变体用 Null 占位（目前所有变体都有载荷，此处为防御）。
    let data = value
        .get("data")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let data = serde_json::to_string(&data).map_err(|_| ErrorCode::FailToSerializeAction)?;
    let detail = aes::encrypt(data.into_bytes(), state::key())?;
    let log = Log {
        id: uuid::Uuid::new_v4().to_string(),
        object_id: object_id.to_string(),
        action: variant,
        time: time_util::now(),
        detail,
    };
    let connection = state::lock_connection();
    dao::insert(&connection, &log)?;
    Ok(())
}
