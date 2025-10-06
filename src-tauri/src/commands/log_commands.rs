//! 前端日志命令
//!
//! 提供前端日志到后端的传输通道,使用统一的 tracing 框架记录。
//! 遵循"日志是思想的表达"原则,每条日志都有意义。

use crate::models::frontend_log::{FrontendLog, LogLevel};

/// 记录单条前端日志事件
///
/// 将前端日志通过 tracing 框架记录到后端日志系统,
/// 与后端日志使用相同的格式和存储机制。
#[tauri::command]
pub async fn log_frontend_event(log: FrontendLog) -> Result<(), String> {
    match log.level {
        LogLevel::Error => {
            tracing::error!(
                来源 = "frontend",
                消息 = %log.message,
                上下文 = ?log.context,
                url = ?log.url,
                user_agent = ?log.user_agent,
                "前端错误"
            );
        }
        LogLevel::Warn => {
            tracing::warn!(
                来源 = "frontend",
                消息 = %log.message,
                上下文 = ?log.context,
                url = ?log.url,
                "前端警告"
            );
        }
        LogLevel::Info => {
            tracing::info!(
                来源 = "frontend",
                消息 = %log.message,
                上下文 = ?log.context,
                "前端信息"
            );
        }
        LogLevel::Debug => {
            tracing::debug!(
                来源 = "frontend",
                消息 = %log.message,
                上下文 = ?log.context,
                "前端调试"
            );
        }
    }

    Ok(())
}

/// 批量记录前端日志
///
/// 性能优化:前端可批量发送日志,减少IPC调用次数。
/// 每条日志独立处理,单条失败不影响其他日志。
#[tauri::command]
pub async fn log_frontend_batch(logs: Vec<FrontendLog>) -> Result<(), String> {
    for log in logs {
        log_frontend_event(log).await?;
    }
    Ok(())
}
