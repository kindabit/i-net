mod create;
mod export;
// get 当前仅被单元测试使用（生产代码无调用者），只在测试构建中编译，避免未使用告警。
#[cfg(test)]
mod get;
mod import;
mod initialize;
mod list;
mod list_orphan_files;
mod load;
mod logical_delete;
mod physical_delete;
mod rename;
mod remove_orphan_file;
mod restore;
mod swap_sort_order;
mod update_file;

pub use create::create;
pub use export::export;
#[cfg(test)]
pub use get::get;
pub use import::import;
pub use initialize::initialize;
pub use list::list;
pub use list_orphan_files::list_orphan_files;
pub use load::load;
pub use logical_delete::logical_delete;
pub use physical_delete::physical_delete;
pub use rename::rename;
pub use remove_orphan_file::remove_orphan_file;
pub use restore::restore;
pub use swap_sort_order::swap_sort_order;
pub use update_file::update_file;

/// 单个附件的明文大小上限（单位 MB），与前端 error-code 文案中的数值对应。
pub const MAX_ATTACHMENT_SIZE_MB: u64 = 100;
