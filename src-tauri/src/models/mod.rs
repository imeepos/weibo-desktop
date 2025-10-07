//! 数据模型模块
//!
//! 包含所有核心数据结构:
//! - errors: 错误类型定义 (API、验证、存储、应用级错误)
//! - login_session: 登录会话管理 (二维码状态追踪)
//! - cookies_data: Cookies数据结构 (凭证存储与验证)
//!
//! # 设计原则
//!
//! 遵循 `.specify/memory/constitution.md` 的所有原则:
//! 1. **存在即合理**: 每个字段都有明确目的,无冗余
//! 2. **优雅即简约**: 类型名自文档化,代码自我阐述
//! 3. **性能即艺术**: 使用引用而非克隆,高效数据结构
//! 4. **错误处理**: 所有验证返回 Result,提供完整上下文
//! 5. **日志安全**: 敏感数据不记录到日志 (如 cookies 值)

pub mod cookies_data;
pub mod dependency;
pub mod errors;
pub mod events;
pub mod frontend_log;
pub mod login_session;
pub mod redis_config;

// PostgreSQL简化架构模型（003-爬取任务）
pub mod postgres;

// 重导出常用类型,简化外部引用
pub use cookies_data::CookiesData;
pub use errors::{ApiError, StorageError, ValidationError};
pub use login_session::{LoginSession, QrCodeStatus};
pub use redis_config::{RedisConfig, RedisConfigError};

/// 解析微博API返回码为二维码状态
///
/// 单一真相来源: 直接映射微博API的retcode
/// 参考: https://passport.weibo.com/sso/v2/qrcode/check
pub fn parse_qr_status(retcode: i32) -> QrCodeStatus {
    match retcode {
        20000000 | 50114001 => QrCodeStatus::Pending,
        50114002 => QrCodeStatus::Scanned,
        50114003 | 50114007 => QrCodeStatus::Rejected,
        50114004..=50114006 => QrCodeStatus::Expired,
        20000001 => QrCodeStatus::Confirmed,
        _ => QrCodeStatus::Expired,
    }
}
