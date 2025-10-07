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
#[allow(dead_code)]
pub fn init_logging(
) -> Result<tracing_appender::non_blocking::WorkerGuard, Box<dyn std::error::Error>> {
    use tracing_appender::rolling;

    // 1. 获取应用数据目录
    // Tauri 2.x: 使用 std::env 或 dirs crate 替代 tauri::api::path
    // 优先使用系统配置目录,回退到当前目录
    let app_data_dir = dirs::config_dir()
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
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

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
                .with_line_number(false),
        )
        .with(env_filter)
        .init();

    Ok(guard)
}

/// 日志宏辅助模块
///
/// 提供优雅的中文结构化日志宏，遵循"日志是思想的表达"原则
pub mod macros {
    /// 记录二维码相关业务事件
    ///
    /// 使用示例:
    /// ```no_run
    /// use weibo_login::log_qr_event;
    /// log_qr_event!(
    ///     "二维码生成",
    ///     qr_id = "qr_abc123",
    ///     有效期秒数 = 180
    /// );
    /// ```
    #[macro_export]
    macro_rules! log_qr_event {
        ($message:expr, $($field:tt = $value:expr),* $(,)?) => {
            tracing::info!(
                事件类型 = "二维码操作",
                $($field = $value),*,
                message = $message
            );
        };
    }

    /// 记录登录相关业务事件
    ///
    /// 使用示例:
    /// ```no_run
    /// use weibo_login::log_login_event;
    /// log_login_event!(
    ///     "登录成功",
    ///     用户ID = "1234567890",
    ///     用户名 = "example_user"
    /// );
    /// ```
    #[macro_export]
    macro_rules! log_login_event {
        ($message:expr, $($field:tt = $value:expr),* $(,)?) => {
            tracing::info!(
                事件类型 = "登录流程",
                $($field = $value),*,
                message = $message
            );
        };
    }

    /// 记录存储相关业务事件
    ///
    /// 使用示例:
    /// ```no_run
    /// use weibo_login::log_storage_event;
    /// log_storage_event!(
    ///     "数据保存成功",
    ///     数据类型 = "Cookies",
    ///     用户ID = "1234567890"
    /// );
    /// ```
    #[macro_export]
    macro_rules! log_storage_event {
        ($message:expr, $($field:tt = $value:expr),* $(,)?) => {
            tracing::info!(
                事件类型 = "数据存储",
                $($field = $value),*,
                message = $message
            );
        };
    }

    /// 记录网络请求相关事件
    ///
    /// 使用示例:
    /// ```no_run
    /// use weibo_login::log_network_event;
    /// log_network_event!(
    ///     "API调用成功",
    ///     端点 = "/generate_qrcode",
    ///     耗时毫秒 = 1500
    /// );
    /// ```
    #[macro_export]
    macro_rules! log_network_event {
        ($message:expr, $($field:tt = $value:expr),* $(,)?) => {
            tracing::info!(
                事件类型 = "网络请求",
                $($field = $value),*,
                消息 = $message
            );
        };
    }

    /// 记录验证相关业务事件
    ///
    /// 使用示例:
    /// ```no_run
    /// use weibo_login::log_validation_event;
    /// log_validation_event!(
    ///     "验证通过",
    ///     验证类型 = "Cookies有效性",
    ///     用户ID = "1234567890"
    /// );
    /// ```
    #[macro_export]
    macro_rules! log_validation_event {
        ($message:expr, $($field:tt = $value:expr),* $(,)?) => {
            tracing::info!(
                事件类型 = "数据验证",
                $($field = $value),*,
                message = $message
            );
        };
    }

    /// 记录系统级事件
    ///
    /// 使用示例:
    /// ```no_run
    /// use weibo_login::log_system_event;
    /// log_system_event!(
    ///     "应用启动完成",
    ///     组件 = "Redis连接池",
    ///     状态 = "健康"
    /// );
    /// ```
    #[macro_export]
    macro_rules! log_system_event {
        ($message:expr, $($field:tt = $value:expr),* $(,)?) => {
            tracing::info!(
                事件类型 = "系统事件",
                $($field = $value),*,
                message = $message
            );
        };
    }

    /// 记录错误事件，包含错误上下文
    ///
    /// 使用示例:
    /// ```no_run
    /// use weibo_login::log_error;
    /// log_error!(
    ///     "网络连接失败",
    ///     二维码ID = "qr_abc123",
    ///     错误详情 = "连接超时"
    /// );
    /// ```
    #[macro_export]
    macro_rules! log_error {
        ($message:expr, $($field:tt = $value:expr),* $(,)?) => {
            tracing::error!(
                错误类型 = "业务错误",
                $($field = $value),*,
                message = $message
            );
        };
    }

    /// 记录警告事件
    ///
    /// 使用示例:
    /// ```no_run
    /// use weibo_login::log_warn;
    /// log_warn!(
    ///     "二维码即将过期",
    ///     二维码ID = "qr_abc123",
    ///     剩余秒数 = 30
    /// );
    /// ```
    #[macro_export]
    macro_rules! log_warn {
        ($message:expr, $($field:tt = $value:expr),* $(,)?) => {
            tracing::warn!(
                警告类型 = "业务警告",
                $($field = $value),*,
                message = $message
            );
        };
    }

    /// 记录调试信息
    ///
    /// 使用示例:
    /// ```no_run
    /// use weibo_login::log_debug;
    /// log_debug!(
    ///     "状态轮询中",
    ///     二维码ID = "qr_abc123",
    ///     轮询次数 = 5
    /// );
    /// ```
    #[macro_export]
    macro_rules! log_debug {
        ($message:expr, $($field:tt = $value:expr),* $(,)?) => {
            tracing::debug!(
                调试类型 = "状态跟踪",
                $($field = $value),*,
                message = $message
            );
        };
    }

    // 保持向后兼容的旧宏
    #[macro_export]
    macro_rules! log_event {
        ($event_type:expr, $($field:tt = $value:expr),* $(,)?) => {
            tracing::info!(
                event_type = $event_type,
                $($field = $value),*
            );
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logger_initialization() {
        // 测试日志系统可以正常初始化
        let result = init();
        assert!(result.is_ok());

        // 写入测试日志 - 使用新的中文日志宏
        crate::log_system_event!("日志系统初始化完成", 级别 = "INFO");
        crate::log_warn!("这是一个警告测试", 测试场景 = "系统警告");
        crate::log_error!("这是一个错误测试", 测试场景 = "系统错误");

        // 使用新的领域特定日志宏
        crate::log_qr_event!("测试二维码生成", 二维码ID = "test_qr_123", 有效期秒数 = 180);
        crate::log_login_event!("测试登录事件", 用户ID = "123456789", 状态 = "成功");
        crate::log_storage_event!("测试数据存储", 数据类型 = "Cookies", 存储位置 = "Redis");
        crate::log_network_event!("测试网络请求", 端点 = "/api/test", 耗时毫秒 = 150);
        crate::log_validation_event!("测试数据验证", 验证类型 = "格式检查", 结果 = "通过");
    }

    #[test]
    fn test_dependency_logging_initialization() {
        // 打印日志目录位置
        // Tauri 2.x: 使用 dirs crate 替代 tauri::api::path
        let app_data_dir = dirs::config_dir()
            .map(|p| p.join("微博登录助手"))
            .unwrap_or_else(|| std::path::PathBuf::from("."));
        let log_dir = app_data_dir.join("logs");
        println!("日志目录: {:?}", log_dir);

        // 测试依赖检测日志系统可以正常初始化
        let result = init_logging();
        assert!(result.is_ok());

        // 保存guard,防止被drop
        let _guard = result.unwrap();

        // 写入测试日志 - 使用新的中文日志宏
        crate::log_validation_event!(
            "依赖检测完成",
            依赖名称 = "node",
            状态 = "满足",
            版本 = "20.10.0"
        );
        crate::log_error!("依赖缺失", 依赖名称 = "redis", 期望状态 = "已安装");
        crate::log_warn!(
            "版本不匹配",
            依赖名称 = "playwright",
            检测版本 = "1.35.0",
            要求版本 = "1.40.0"
        );

        // 验证日志文件已创建
        if log_dir.exists() {
            println!("日志目录已创建: {:?}", log_dir);
        }
    }
}
