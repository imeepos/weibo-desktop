//! 前端日志模型
//!
//! 定义前端日志的数据结构,用于捕获和传输前端日志到后端统一管理。
//! 遵循"存在即合理"原则,每个字段都有明确用途。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 前端日志数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontendLog {
    /// 日志级别
    pub level: LogLevel,
    /// 日志消息
    pub message: String,
    /// 上下文信息 (JSON格式)
    pub context: serde_json::Value,
    /// 时间戳
    pub timestamp: DateTime<Utc>,
    /// 用户代理字符串
    pub user_agent: Option<String>,
    /// 页面URL
    pub url: Option<String>,
}

/// 日志级别枚举
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
}
