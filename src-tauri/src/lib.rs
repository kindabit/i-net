pub mod argv;
mod business;
mod common;
mod error_code;
mod security;
mod state;
#[cfg(test)]
mod test;
mod util;

/// 初始化应用程序数据目录，并返回相关路径信息。
///
/// # 参数
/// - `data_directory_override`：可选的数据目录覆盖值。如果提供，则使用该目录；
///   否则使用 [`directories::ProjectDirs`] 计算的默认路径。
///
/// # 返回值
/// 返回包含数据目录、日志目录、用户数据库集合目录、偏好数据库文件和元数据数据库文件的路径结构体。
fn initialize_data_directory(data_directory_override: Option<std::path::PathBuf>) -> state::Path {
    let data_directory = data_directory_override.unwrap_or_else(|| {
        directories::ProjectDirs::from("pw.saya", "saya", "i-net")
            .expect("failed to determine project directories")
            .data_dir()
            .to_path_buf()
    });

    let log_directory = data_directory.join("logs");
    let user_database_set_directory = data_directory.join("user_database_set");
    let preference_database_file = data_directory.join("preference.sqlite");
    let metadata_database_file = data_directory.join("metadata.sqlite");

    std::fs::create_dir_all(&data_directory).expect("failed to create data directory");
    std::fs::create_dir_all(&log_directory).expect("failed to create log directory");
    std::fs::create_dir_all(&user_database_set_directory)
        .expect("failed to create user database set directory");

    state::Path {
        data_directory,
        log_directory,
        user_database_set_directory,
        preference_database_file,
        metadata_database_file,
    }
}

/// 初始化日志系统，配置按日滚动的文件日志输出，并在调试模式下同时输出到标准输出。
///
/// 日志文件名只保留日期，格式为 `{yyyy-mm-dd}.log`。
///
/// # 参数
/// - `log_directory`: 日志文件存放目录。
///
/// # 返回值
/// 无。
fn initialize_logging(log_directory: &std::path::Path) {
    let file_appender = tracing_appender::rolling::Builder::new()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_suffix("log")
        .build(log_directory)
        .expect("failed to initialize file appender");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    Box::leak(Box::new(guard));

    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    let file_layer = tracing_subscriber::fmt::layer().with_writer(non_blocking);

    #[cfg(debug_assertions)]
    {
        let stdout_layer = tracing_subscriber::fmt::layer().with_writer(std::io::stdout);
        tracing_subscriber::registry()
            .with(stdout_layer)
            .with(file_layer)
            .with(env_filter)
            .init();
    }
    #[cfg(not(debug_assertions))]
    {
        tracing_subscriber::registry()
            .with(file_layer)
            .with(env_filter)
            .init();
    }

    // 任何线程 panic 时记录日志并立即中止整个进程，避免在状态可能不一致的情况下继续运行。
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        tracing::error!("{}", panic_info);
        default_hook(panic_info);
        std::process::abort();
    }));
}

/// Tauri 应用入口函数，构建并运行 Tauri 应用。
///
/// # 参数
/// - `argv`：解析后的命令行参数。
///
/// # 返回值
/// 无。
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run(argv: argv::ArgV) {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            business::backup::command::backup::backup_backup,
            business::backup::command::restore::backup_restore,
            business::backup::command::restore_probe::backup_restore_probe,
            business::backup::command::data_directory_size::backup_data_directory_size,
            business::reclaim::command::metadata::reclaim_metadata,
            business::reclaim::command::preference::reclaim_preference,
            business::reclaim::command::user_database::reclaim_user_database,
            business::metadata::command::metadata_archive::metadata_archive,
            business::metadata::command::metadata_list::metadata_list,
            business::metadata::command::metadata_physical_delete::metadata_physical_delete,
            business::metadata::command::metadata_register::metadata_register,
            business::metadata::command::metadata_save::metadata_save,
            business::preference::command::preference_get::preference_get,
            business::preference::command::preference_set::preference_set,
            business::preference::command::preference_save::preference_save,
            business::clipboard::command::clipboard_clear,
            business::user_database::attachment::command::user_database_attachment_export::user_database_attachment_export,
            business::user_database::attachment::command::user_database_attachment_import::user_database_attachment_import,
            business::user_database::attachment::command::user_database_attachment_list::user_database_attachment_list,
            business::user_database::attachment::command::user_database_attachment_list_orphan_files::user_database_attachment_list_orphan_files,
            business::user_database::attachment::command::user_database_attachment_load::user_database_attachment_load,
            business::user_database::attachment::command::user_database_attachment_logical_delete::user_database_attachment_logical_delete,
            business::user_database::attachment::command::user_database_attachment_physical_delete::user_database_attachment_physical_delete,
            business::user_database::attachment::command::user_database_attachment_remove_orphan_file::user_database_attachment_remove_orphan_file,
            business::user_database::attachment::command::user_database_attachment_restore::user_database_attachment_restore,
            business::user_database::attachment::command::user_database_attachment_swap_sort_order::user_database_attachment_swap_sort_order,
            business::user_database::attachment::command::user_database_attachment_update_file::user_database_attachment_update_file,
            business::user_database::canvas::command::user_database_canvas_color_list::user_database_canvas_color_list,
            business::user_database::canvas::command::user_database_canvas_create::user_database_canvas_create,
            business::user_database::canvas::command::user_database_canvas_list::user_database_canvas_list,
            business::user_database::canvas::command::user_database_canvas_logical_delete::user_database_canvas_logical_delete,
            business::user_database::canvas::command::user_database_canvas_move_canvas::user_database_canvas_move_canvas,
            business::user_database::canvas::command::user_database_canvas_move_canvases::user_database_canvas_move_canvases,
            business::user_database::canvas::command::user_database_canvas_physical_delete::user_database_canvas_physical_delete,
            business::user_database::canvas::command::user_database_canvas_rename::user_database_canvas_rename,
            business::user_database::canvas::command::user_database_canvas_restore::user_database_canvas_restore,
            business::user_database::canvas::command::user_database_canvas_set_color::user_database_canvas_set_color,
            business::user_database::dictionary::command::user_database_dictionary_list::user_database_dictionary_list,
            business::user_database::dictionary::command::user_database_dictionary_set::user_database_dictionary_set,
            business::user_database::export::command::user_database_export::user_database_export,
            business::user_database::edge::command::user_database_edge_create::user_database_edge_create,
            business::user_database::edge::command::user_database_edge_delete::user_database_edge_delete,
            business::user_database::edge::command::user_database_edge_list::user_database_edge_list,
            business::user_database::edge::command::user_database_edge_update::user_database_edge_update,
            business::user_database::lifecycle::command::user_database_lifecycle_close::user_database_lifecycle_close,
            business::user_database::lifecycle::command::user_database_lifecycle_initialize::user_database_lifecycle_initialize,
            business::user_database::lifecycle::command::user_database_lifecycle_save::user_database_lifecycle_save,
            business::user_database::log::command::user_database_log_list::user_database_log_list,
            business::user_database::node::command::user_database_node_create::user_database_node_create,
            business::user_database::node::command::user_database_node_copy::user_database_node_copy,
            business::user_database::node::command::user_database_node_list::user_database_node_list,
            business::user_database::node::command::user_database_node_logical_delete::user_database_node_logical_delete,
            business::user_database::node::command::user_database_node_modify::user_database_node_modify,
            business::user_database::node::command::user_database_node_move_node::user_database_node_move_node,
            business::user_database::node::command::user_database_node_move_nodes::user_database_node_move_nodes,
            business::user_database::node::command::user_database_node_physical_delete::user_database_node_physical_delete,
            business::user_database::node::command::user_database_node_relocate_nodes::user_database_node_relocate_nodes,
            business::user_database::node::command::user_database_node_restore::user_database_node_restore,
            business::user_database::node::command::user_database_node_color_list::user_database_node_color_list,
            business::user_database::node::command::user_database_node_search::user_database_node_search,
            business::user_database::node::command::user_database_node_set_color::user_database_node_set_color,
            business::user_database::node_field::command::user_database_node_field_get::user_database_node_field_get,
            business::user_database::node_field::command::user_database_node_field_set::user_database_node_field_set,
            business::user_database::template::command::user_database_template_create::user_database_template_create,
            business::user_database::template::command::user_database_template_create_from_node::user_database_template_create_from_node,
            business::user_database::template::command::user_database_template_delete::user_database_template_delete,
            business::user_database::template::command::user_database_template_export::user_database_template_export,
            business::user_database::template::command::user_database_template_get_fields::user_database_template_get_fields,
            business::user_database::template::command::user_database_template_import::user_database_template_import,
            business::user_database::template::command::user_database_template_list::user_database_template_list,
            business::user_database::template::command::user_database_template_rename::user_database_template_rename,
            business::user_database::template::command::user_database_template_set_fields::user_database_template_set_fields,
            business::user_database::registry::command::user_database_registry_get::user_database_registry_get,
            business::user_database::registry::command::user_database_registry_set::user_database_registry_set,
            business::user_database::viewport::command::user_database_viewport_get::user_database_viewport_get,
            business::user_database::viewport::command::user_database_viewport_set::user_database_viewport_set,
        ])
        .setup(move |_app| {
            let path = initialize_data_directory(argv.data_directory);
            initialize_logging(&path.log_directory);
            // tauri 不再托管状态，路径状态与数据库连接状态由各模块手动管理。
            state::set_path(path);
            business::preference::service::initialize().expect("failed to initialize preference database");
            business::metadata::service::initialize().expect("failed to initialize metadata database");
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application");
    app.run(|_app_handle, event| {
        // 应用程序退出时保存两个数据库，让持久化能力对用户保持透明。
        if let tauri::RunEvent::Exit = event {
            if let Err(error) = business::preference::service::save() {
                tracing::error!("failed to save preference database: {:?}", error);
            }
            if let Err(error) = business::metadata::service::save() {
                tracing::error!("failed to save metadata database: {:?}", error);
            }
            // 有打开的用户数据库时一并保存，失败仅记录日志。
            if business::user_database::state::is_open() {
                if let Err(error) = business::user_database::lifecycle::service::save() {
                    tracing::error!("failed to save user database: {:?}", error);
                }
            }
        }
    });
}
