// 禁用Windows控制台窗口
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod models;
mod services;
mod state;
mod utils;

use services::ConfigService;
use state::AppState;

fn main() {
    // 初始化日志系统
    utils::logger::init().expect("日志系统初始化失败");

    tracing::info!("应用程序启动 (WebSocket模式)...");

    // 读取 Redis 配置 (从 .env 文件)
    let redis_config = ConfigService::load_redis_config()
        .expect("无法加载 Redis 配置");
    let redis_url = redis_config.to_connection_url();

    tracing::info!(
        redis_config = %redis_config.summary_for_logging(),
        "已加载 Redis 配置"
    );

    let playwright_server_url = std::env::var("PLAYWRIGHT_SERVER_URL")
        .unwrap_or_else(|_| "ws://localhost:9223".to_string());
    let playwright_validation_script = std::env::var("PLAYWRIGHT_VALIDATION_SCRIPT")
        .unwrap_or_else(|_| {
            "/home/ubuntu/worktrees/desktop/playwright/dist/validate-cookies.js".to_string()
        });

    tracing::info!(
        playwright_server = %playwright_server_url,
        validation_script = %playwright_validation_script,
        "Playwright server 由外部脚本管理 (scripts/start-playwright-server.sh)"
    );

    // 初始化全局状态
    let app_state = AppState::new(
        &redis_url,
        &playwright_server_url,
        &playwright_validation_script,
    )
    .expect("Failed to initialize AppState");

    // 启动Tauri应用
    tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            commands::qrcode_commands::generate_qrcode,
            commands::cookies_commands::save_cookies,
            commands::cookies_commands::query_cookies,
            commands::cookies_commands::delete_cookies,
            commands::cookies_commands::list_all_uids,
            commands::dependency_commands::check_dependencies,
            commands::dependency_commands::install_dependency,
            commands::dependency_commands::query_dependency_status,
            commands::dependency_commands::trigger_manual_check,
            commands::log_commands::log_frontend_event,
            commands::log_commands::log_frontend_batch,
            commands::playwright_commands::start_playwright_server,
            commands::playwright_commands::stop_playwright_server,
            commands::playwright_commands::check_playwright_server,
            commands::playwright_commands::get_playwright_logs,
            commands::redis_commands::test_redis_connection,
            commands::redis_commands::save_redis_config,
            commands::redis_commands::load_redis_config,
        ])
        .setup(move |_app| {
            // 浏览器后端选择
            let backend = std::env::var("BROWSER_BACKEND")
                .unwrap_or_else(|_| "playwright".to_string());

            match backend.as_str() {
                #[cfg(feature = "rust-browser-poc")]
                "rust-poc" => {
                    tracing::info!("使用 Rust 浏览器 POC (实验性)");
                    tracing::info!("正在启动内置 WebSocket 服务器 (端口: 9223)...");

                    tokio::spawn(async {
                        if let Err(e) = services::WebSocketServerPoc::start().await {
                            tracing::error!("WebSocket 服务器启动失败: {}", e);
                        } else {
                            tracing::info!("WebSocket 服务器已停止");
                        }
                    });
                }
                _ => {
                    tracing::info!("使用 Playwright Server (稳定,默认)");
                    tracing::info!("Playwright server 由外部脚本管理: {}", playwright_server_url);
                }
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("启动Tauri应用时发生错误");

    tracing::info!("应用程序已停止");
}
