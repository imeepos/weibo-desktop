/// Tauri命令模块
///
/// 包含所有前端可调用的命令:
/// - qrcode_commands: 二维码生成和轮询
/// - cookies_commands: Cookies保存/查询/删除
/// - dependency_commands: 依赖检测和安装
/// - playwright_commands: Playwright服务管理

pub mod cookies_commands;
pub mod dependency_commands;
pub mod log_commands;
pub mod playwright_commands;
pub mod qrcode_commands;
