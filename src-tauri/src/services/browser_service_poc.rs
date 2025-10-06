//! 浏览器服务 - Chromium 生命周期管理
//!
//! 职责:
//! - 启动和管理全局 Chromium 实例
//! - 提供浏览器配置 (headless, no-sandbox)
//! - 处理浏览器连接状态

use chromiumoxide::browser::{Browser, BrowserConfig};
use futures_util::StreamExt;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn};
use crate::models::errors::ApiError;

/// 全局浏览器实例 (使用 Arc 共享)
static GLOBAL_BROWSER: once_cell::sync::Lazy<Arc<Mutex<Option<Arc<Browser>>>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(None)));

/// 浏览器服务
pub struct BrowserService;

impl BrowserService {
    /// 获取或创建浏览器实例
    pub async fn get_browser() -> Result<Arc<Browser>, ApiError> {
        let mut browser_guard = GLOBAL_BROWSER.lock().await;

        // 检查现有实例是否仍然连接
        if let Some(browser) = browser_guard.as_ref() {
            // chromiumoxide 没有直接的 is_connected 方法
            // 我们尝试创建新页面来验证连接
            if browser.new_page("about:blank").await.is_ok() {
                info!("复用现有浏览器实例");
                return Ok(Arc::clone(browser));
            } else {
                warn!("现有浏览器实例已断开连接,将重新创建");
                *browser_guard = None;
            }
        }

        // 创建新浏览器实例
        info!("启动新 Chromium 实例");

        let (browser, mut handler) = Browser::launch(
            BrowserConfig::builder()
                .with_head() // 先用有头模式调试,后续改为 without_head()
                .args(vec![
                    "--no-sandbox",
                    "--disable-setuid-sandbox",
                    "--disable-dev-shm-usage",
                ])
                .build()
                .map_err(|e| ApiError::BrowserError(format!("浏览器配置失败: {}", e)))?
        )
        .await
        .map_err(|e| ApiError::BrowserError(format!("浏览器启动失败: {}", e)))?;

        // 启动后台任务处理浏览器事件
        tokio::spawn(async move {
            while let Some(_event) = handler.next().await {
                // 处理浏览器事件 (可选)
                // 这里暂时不需要特殊处理
            }
            warn!("浏览器事件处理器已退出");
        });

        info!("Chromium 实例启动成功");
        let browser_arc = Arc::new(browser);
        *browser_guard = Some(Arc::clone(&browser_arc));
        Ok(browser_arc)
    }

    /// 关闭浏览器实例
    pub async fn close_browser() -> Result<(), ApiError> {
        let mut browser_guard = GLOBAL_BROWSER.lock().await;

        if let Some(browser) = browser_guard.take() {
            info!("正在关闭浏览器实例");
            // chromiumoxide 会在 drop 时自动关闭
            drop(browser);
            info!("浏览器实例已关闭");
        }

        Ok(())
    }
}
