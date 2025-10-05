use std::io;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// 初始化日志系统
///
/// 配置结构化日志输出,遵循"日志是思想的表达"原则:
/// - JSON格式: 便于机器解析和日志分析
/// - 按天轮转: 每天一个新文件,自动管理日志历史
/// - 保留30天: 平衡存储空间与调试需求
/// - 双输出: 控制台(开发) + 文件(生产)
/// - 环境变量控制: RUST_LOG=debug 可调整日志级别
///
/// # 日志级别
/// - ERROR: 严重错误,需要立即关注
/// - WARN: 警告信息,可能导致问题
/// - INFO: 关键业务事件 (默认级别)
/// - DEBUG: 详细调试信息
/// - TRACE: 极详细的跟踪信息
///
/// # 示例日志
/// ```json
/// {
///   "timestamp": "2025-10-05T10:30:45.123Z",
///   "level": "INFO",
///   "target": "weibo_login::services::qr",
///   "fields": {
///     "qr_id": "qr_abc123",
///     "event_type": "QrCodeGenerated"
///   },
///   "message": "二维码生成成功"
/// }
/// ```
pub fn init() -> Result<(), io::Error> {
    // 日志目录: ./logs
    let log_dir = "logs";

    // 按天轮转的文件写入器
    // 文件命名格式: weibo-login.2025-10-05.log
    let file_appender = RollingFileAppender::builder()
        .rotation(Rotation::DAILY) // 每天轮转
        .filename_prefix("weibo-login") // 文件名前缀
        .filename_suffix("log") // 文件扩展名
        .max_log_files(30) // 保留30天
        .build(log_dir)
        .expect("无法创建日志文件");

    // 环境变量过滤器
    // 默认: INFO级别
    // 可通过 RUST_LOG=debug 覆盖
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    // 文件层: JSON格式,便于日志分析工具解析
    let file_layer = fmt::layer()
        .json()
        .with_writer(file_appender)
        .with_target(true) // 包含模块路径
        .with_thread_ids(false) // 不记录线程ID(减少噪音)
        .with_thread_names(false)
        .with_file(false) // 不记录文件名(target已足够)
        .with_line_number(false);

    // 控制台层: 人类可读格式,便于开发调试
    let console_layer = fmt::layer()
        .with_writer(io::stdout)
        .with_target(true)
        .with_level(true)
        .with_ansi(true); // 彩色输出

    // 组合订阅器
    tracing_subscriber::registry()
        .with(env_filter)
        .with(file_layer)
        .with(console_layer)
        .init();

    Ok(())
}

/// 日志宏辅助模块
///
/// 提供结构化日志的便捷宏
pub mod macros {
    /// 记录业务事件
    ///
    /// 使用示例:
    /// ```
    /// log_event!(
    ///     "QrCodeScanned",
    ///     qr_id = "qr_abc123",
    ///     uid = "1234567890"
    /// );
    /// ```
    #[macro_export]
    macro_rules! log_event {
        ($event_type:expr, $($field:tt = $value:expr),* $(,)?) => {
            tracing::info!(
                event_type = $event_type,
                $($field = $value),*
            );
        };
    }

    /// 记录错误事件
    ///
    /// 使用示例:
    /// ```
    /// log_error!(
    ///     "PollingFailed",
    ///     qr_id = "qr_abc123",
    ///     error = %err
    /// );
    /// ```
    #[macro_export]
    macro_rules! log_error {
        ($event_type:expr, $($field:tt = $value:expr),* $(,)?) => {
            tracing::error!(
                event_type = $event_type,
                $($field = $value),*
            );
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing::{error, info, warn};

    #[test]
    fn test_logger_initialization() {
        // 测试日志系统可以正常初始化
        let result = init();
        assert!(result.is_ok());

        // 写入测试日志
        info!("日志系统测试: INFO级别");
        warn!("日志系统测试: WARN级别");
        error!("日志系统测试: ERROR级别");

        // 结构化日志测试
        info!(
            qr_id = "test_qr_123",
            event_type = "TestEvent",
            "结构化日志测试"
        );
    }
}
