//! 数据模型模块
//!
//! 包含所有核心数据结构:
//! - errors: 错误类型定义 (API、验证、存储、应用级错误)
//! - login_session: 登录会话管理 (二维码状态追踪)
//! - cookies_data: Cookies数据结构 (凭证存储与验证)
//! - login_event: 前端事件通知 (登录流程审计追踪)
//!
//! # 设计原则
//!
//! 遵循 `.specify/memory/constitution.md` 的所有原则:
//! 1. **存在即合理**: 每个字段都有明确目的,无冗余
//! 2. **优雅即简约**: 类型名自文档化,代码自我阐述
//! 3. **性能即艺术**: 使用引用而非克隆,高效数据结构
//! 4. **错误处理**: 所有验证返回 Result,提供完整上下文
//! 5. **日志安全**: 敏感数据不记录到日志 (如 cookies 值)

pub mod errors;
pub mod login_session;
pub mod cookies_data;
pub mod login_event;

// 重导出常用类型,简化外部引用
pub use errors::{ApiError, ValidationError, StorageError, AppError};
pub use login_session::{LoginSession, QrCodeStatus};
pub use cookies_data::CookiesData;
pub use login_event::{LoginEvent, LoginEventType};
