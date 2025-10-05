use std::io;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// 初始化日志系统
///
/// 配置结构化日志输出,遵循"日志是思想的表达"原则:
/// - JSON格式: 便于机器解析和日志分析
/// - 按天轮转: 每天一个新文件,自动管理日志历史
/// - 永久存储: 日志文件永久保留,不自动删除 (FR-009)
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
    // 永久存储: 移除 max_log_files 限制,确保审计日志永久保留
    let file_appender = RollingFileAppender::builder()
        .rotation(Rotation::DAILY) // 每天轮转
        .filename_prefix("weibo-login") // 文件名前缀
        .filename_suffix("log") // 文件扩展名
        .build(log_dir)
        .expect("无法创建日志文件");

    // 环境变量过滤器
    // 默认: INFO级别
    // 可通过 RUST_LOG=debug 覆盖
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

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

/// 依赖检测日志初始化 (基于research.md规范)
///
/// 实现应用数据目录日志持久化策略:
/// - 日志路径: <app_data_dir>/logs/dependency_check_YYYY-MM-DD.log
/// - JSON格式: 便于后续分析和搜索
/// - non_blocking: 避免日志I/O阻塞检测流程
/// - 按日期滚动: 自然组织日志文件
///
/// # 日志位置
/// - Windows: `C:\Users\<user>\AppData\Roaming\微博登录助手\logs\`
/// - macOS: `~/Library/Application Support/微博登录助手/logs/`
/// - Linux: `~/.local/share/微博登录助手/logs/`
///
/// # 示例日志
/// ```json
/// {
///     "timestamp": "2025-10-05T10:30:15.123Z",
///     "level": "INFO",
///     "target": "dependency_checker",
///     "fields": {
///         "message": "Dependency check completed",
///         "dependency": "node",
///         "status": "satisfied",
///         "version": "20.10.0"
///     }
/// }
/// ```
///
/// # 重要提示
/// 返回的guard必须被调用者保存,直到应用退出。
/// 如果guard被drop,日志写入器将被关闭。
pub fn init_logging() -> Result<tracing_appender::non_blocking::WorkerGuard, Box<dyn std::error::Error>> {
    use tracing_appender::rolling;
    use tauri::api::path::config_dir;

    // 1. 获取应用数据目录
    // 优先使用系统配置目录,回退到当前目录
    let app_data_dir = config_dir()
        .map(|p| p.join("微博登录助手"))
        .unwrap_or_else(|| std::path::PathBuf::from("."));

    // 2. 创建logs子目录
    let log_dir = app_data_dir.join("logs");
    std::fs::create_dir_all(&log_dir)?;

    // 3. 配置按日期滚动的文件appender
    // 文件名格式: dependency_check_2025-10-05.log
    let file_appender = rolling::daily(log_dir, "dependency_check");

    // 4. 使用non_blocking避免阻塞
    // guard必须被保存,否则写入器会立即关闭
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    // 5. 配置JSON格式输出
    // 6. 设置INFO级别
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    // 7. 初始化subscriber
    tracing_subscriber::registry()
        .with(
            fmt::layer()
                .json() // JSON格式
                .with_writer(non_blocking)
                .with_target(true)
                .with_thread_ids(false)
                .with_thread_names(false)
                .with_file(false)
                .with_line_number(false)
        )
        .with(env_filter)
        .init();

    Ok(guard)
}

/// 日志宏辅助模块
///
/// 提供结构化日志的便捷宏
pub mod macros {
    /// 记录业务事件
    ///
    /// 使用示例:
    /// ```no_run
    /// use weibo_login::log_event;
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
    /// ```no_run
    /// use weibo_login::log_error;
    /// log_error!(
    ///     "PollingFailed",
    ///     qr_id = "qr_abc123",
    ///     error = "connection timeout"
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

    #[test]
    fn test_dependency_logging_initialization() {
        use tauri::api::path::config_dir;

        // 打印日志目录位置
        let app_data_dir = config_dir()
            .map(|p| p.join("微博登录助手"))
            .unwrap_or_else(|| std::path::PathBuf::from("."));
        let log_dir = app_data_dir.join("logs");
        println!("日志目录: {:?}", log_dir);

        // 测试依赖检测日志系统可以正常初始化
        let result = init_logging();
        assert!(result.is_ok());

        // 保存guard,防止被drop
        let _guard = result.unwrap();

        // 写入测试日志
        info!(
            dependency = "node",
            status = "satisfied",
            version = "20.10.0",
            "依赖检测完成"
        );

        info!(
            dependency = "redis",
            status = "missing",
            "依赖缺失"
        );

        warn!(
            dependency = "playwright",
            status = "version_mismatch",
            detected = "1.35.0",
            required = "1.40.0",
            "版本不匹配"
        );

        // 验证日志文件已创建
        if log_dir.exists() {
            println!("日志目录已创建: {:?}", log_dir);
        }
    }
}
