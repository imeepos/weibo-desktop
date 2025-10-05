// 禁用Windows控制台窗口
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use weibo_login::utils::logger;

fn main() {
    // 初始化日志系统
    logger::init().expect("日志系统初始化失败");

    // 启动Tauri应用
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("启动Tauri应用时发生错误");
}
