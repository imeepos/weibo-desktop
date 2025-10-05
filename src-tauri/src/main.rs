// 禁用Windows控制台窗口
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod models;
mod services;
mod state;
mod utils;

use state::AppState;

fn main() {
    // 初始化日志系统
    utils::logger::init().expect("日志系统初始化失败");

    tracing::info!("Application starting (Playwright mode)...");

    // 读取配置 - 环境变量提供核心参数
    let redis_url = std::env::var("REDIS_URL")
        .unwrap_or_else(|_| "redis://localhost:6379".to_string());
    let playwright_login_script = std::env::var("PLAYWRIGHT_LOGIN_SCRIPT")
        .unwrap_or_else(|_| "./playwright/dist/weibo-login.js".to_string());
    let playwright_validation_script = std::env::var("PLAYWRIGHT_VALIDATION_SCRIPT")
        .unwrap_or_else(|_| "./playwright/dist/validate-cookies.js".to_string());

    // 初始化全局状态
    let app_state = AppState::new(
        &redis_url,
        &playwright_login_script,
        &playwright_validation_script,
    )
    .expect("Failed to initialize AppState");

    // 启动Tauri应用
    tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            commands::qrcode_commands::generate_qrcode,
            commands::qrcode_commands::poll_login_status,
            commands::cookies_commands::save_cookies,
            commands::cookies_commands::query_cookies,
            commands::cookies_commands::delete_cookies,
            commands::cookies_commands::list_all_uids,
        ])
        .run(tauri::generate_context!())
        .expect("启动Tauri应用时发生错误");

    tracing::info!("Application stopped");
}
