use crate::business::metadata;
use crate::business::metadata::entity::Metadata;
use crate::business::user_database::{attachment, canvas, dictionary, edge, log, node, node_field, registry, state, template, viewport};
use crate::common::connection;
use crate::error_code::ErrorCode;
use crate::util::{file_system_util, time_util};

/// 初始化（打开）一个用户数据库：如果用户数据库文件不存在，则创建数据库目录、
/// 附件目录和加密的数据库文件，并调用各子业务模块的 initialize 函数建表和填充
/// 初始数据；如果已存在，则用密钥解密打开。打开成功后先更新元信息的最后打开时间
/// 并写回元数据库，再将连接、更新后的元信息和密钥写入 user_database 的 state。
///
/// # 参数
/// - `id`: 数据库 id。
/// - `key`: 32 字节的密钥（密码哈希得到）。
///
/// # 返回值
/// 返回更新过最后打开时间的元数据；id 不存在时返回 `ErrorCode::NoDatabaseWithSuchId`，
/// 密钥无法正确解密时返回 `ErrorCode::FailToDecrypt`，
/// 发生其他错误时返回对应的 `ErrorCode`。
pub fn initialize(id: &str, key: [u8; 32]) -> Result<Metadata, ErrorCode> {
    let metadata_connection = metadata::state::lock_connection();
    let mut metadata = metadata::dao::select_by_id(&metadata_connection, id)?
        .ok_or_else(|| ErrorCode::NoDatabaseWithSuchId { id: id.to_string() })?;
    let path = crate::state::path();
    let database_file = path.user_database_file(id);
    let is_new = !file_system_util::try_exists(&database_file)?;
    if is_new {
        // 用户数据库不存在：创建目录，准备初始化一个全新的加密数据库。
        file_system_util::create_dir_all(&path.user_database_directory(id))?;
        file_system_util::create_dir_all(&path.user_attachment_directory(id))?;
    }
    // 用密钥解密打开（不存在时新建）；错误密钥产生的 FailToDecrypt 自然传播，
    // 此时不更新最后打开时间，也不写入 state。
    let connection = connection::service::open_file_encrypt(&database_file, key)?;
    // 打开成功后先更新最后打开时间并写回元数据库，再把更新后的元信息写入 state，
    // 保证 state 中保存的和返回值都是更新后的元信息。
    metadata.last_open_time = time_util::now();
    metadata::dao::update(&metadata_connection, &metadata)?;
    // 各子业务模块的 initialize 函数通过 state 获取连接，因此先写入连接和密钥。
    state::set_connection(connection);
    state::set_key(key);
    state::set_metadata(metadata.clone());
    if is_new {
        canvas::service::initialize()?;
        viewport::service::initialize()?;
        registry::service::initialize()?;
        node::service::initialize()?;
        attachment::service::initialize()?;
        node_field::service::initialize()?;
        template::service::initialize()?;
        dictionary::service::initialize()?;
        edge::service::initialize()?;
        log::service::initialize()?;
        connection::service::save_file_encrypt(
            &database_file,
            &state::lock_connection(),
            state::key(),
        )?;
    }
    Ok(metadata)
}
