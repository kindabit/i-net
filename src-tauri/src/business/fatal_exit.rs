/// 前端检测到数据损坏（DataCorruption* 系列错误）后请求受控崩溃：
/// 记录英文错误日志并立即退出进程。直接 std::process::exit 终止，
/// 不会触发 RunEvent::Exit 的数据库保存逻辑，避免把损坏的数据写盘。
/// 该路径按设计不可单元测试（会终止测试进程），由代码审查保证。
///
/// # 参数
/// - `detail`: 数据损坏的详细上下文（前端透传的 DataCorruption* 错误内容）。
#[tauri::command]
pub fn fatal_exit(detail: String) {
    tracing::error!("fatal exit requested by frontend: {detail}");
    std::process::exit(1);
}