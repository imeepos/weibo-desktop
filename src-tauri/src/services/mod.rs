//! 服务层模块
//!
//! 包含所有业务逻辑服务:
//! - `redis_service`: Redis存储服务,管理cookies持久化
//! - `weibo_api`: 微博API客户端,生成二维码和轮询状态
//! - `validation_service`: Cookies验证服务,调用Playwright验证有效性
//!
//! # 设计原则
//!
//! 遵循 `.specify/memory/constitution.md` 的所有原则:
//! 1. **存在即合理**: 每个服务都有单一职责,互不重叠
//! 2. **优雅即简约**: 方法签名清晰,易于理解和使用
//! 3. **性能即艺术**: 使用连接池、异步操作和批处理
//! 4. **错误处理**: 所有外部调用都有完整错误处理和日志
//! 5. **日志安全**: 记录关键操作,不记录敏感数据(如cookies值)
//!
//! # 服务架构
//!
//! ```text
//! ┌─────────────────┐
//! │  Tauri Commands │  (Phase 5)
//! └────────┬────────┘
//!          │
//!          ▼
//! ┌──────────────────────────────────────┐
//! │         Services Layer               │
//! │  ┌──────────────┐  ┌──────────────┐ │
//! │  │ WeiboApiClient│  │ValidationSvc │ │
//! │  └──────┬───────┘  └──────┬───────┘ │
//! │         │                 │         │
//! │  ┌──────▼─────────────────▼───────┐ │
//! │  │       RedisService              │ │
//! │  └────────────────────────────────┘ │
//! └──────────────────────────────────────┘
//!          │                 │
//!          ▼                 ▼
//!    Weibo API         Playwright
//! ```
//!
//! # 使用示例
//!
//! ```no_run
//! use weibo_login::services::{RedisService, WeiboApiClient, ValidationService};
//! use weibo_login::models::CookiesData;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // 初始化服务
//! let redis = RedisService::new("redis://localhost:6379")?;
//! let weibo_api = WeiboApiClient::new("/path/to/playwright_script.js".to_string());
//! let validator = ValidationService::new("/path/to/validation_script.js".to_string());
//!
//! // 生成二维码
//! let (mut session, qr_image) = weibo_api.generate_qrcode().await?;
//!
//! // 检查登录状态(单次)
//! if let Some((uid, cookies)) = weibo_api.check_qrcode_status(&mut session).await? {
//!     // 验证cookies
//!     let (uid, screen_name) = validator.validate_cookies(&cookies).await?;
//!
//!     // 保存到Redis
//!     let cookies_data = CookiesData::new(uid, cookies)
//!         .with_screen_name(screen_name);
//!     redis.save_cookies(&cookies_data).await?;
//! }
//! # Ok(())
//! # }
//! ```

// Rust 浏览器 POC (实验性功能,仅在启用 rust-browser-poc feature 时编译)
#[cfg(feature = "rust-browser-poc")]
pub mod browser_service_poc;
#[cfg(feature = "rust-browser-poc")]
pub mod weibo_login_service_poc;
#[cfg(feature = "rust-browser-poc")]
pub mod websocket_server_poc;

pub mod config_service;
pub mod dependency_checker;
pub mod installer_service;
pub mod redis_service;
pub mod session_manager;
pub mod validation_service;
pub mod weibo_api;

// 重导出常用类型,简化外部引用
#[cfg(feature = "rust-browser-poc")]
pub use browser_service_poc::BrowserService as BrowserServicePoc;
#[cfg(feature = "rust-browser-poc")]
pub use weibo_login_service_poc::WeiboLoginService as WeiboLoginServicePoc;
#[cfg(feature = "rust-browser-poc")]
pub use websocket_server_poc::WebSocketServer as WebSocketServerPoc;

pub use config_service::ConfigService;
pub use dependency_checker::DependencyChecker;
pub use installer_service::InstallerService;
pub use redis_service::RedisService;
pub use session_manager::SessionManager;
pub use validation_service::ValidationService;
pub use weibo_api::WeiboApiClient;
