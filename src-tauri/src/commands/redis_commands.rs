use crate::models::redis_config::{RedisConfig, RedisConfigError};
use crate::services::ConfigService;
use serde::{Deserialize, Serialize};
use std::time::Instant;
use thiserror::Error;

/// Redis连接测试错误
///
/// 捕获连接测试过程中的失败场景。
/// 每个错误提供清晰的诊断信息,指导问题解决。
#[derive(Debug, Error, Serialize, Deserialize)]
#[serde(tag = "error")]
pub enum RedisTestError {
    #[error("连接失败: {message}")]
    ConnectionFailed { message: String },

    #[error("认证失败: {message}")]
    AuthenticationFailed { message: String },

    #[error("PING命令执行失败: {message}")]
    PingFailed { message: String },

    #[error("超时: 连接耗时超过{timeout_ms}ms")]
    Timeout { timeout_ms: u64 },

    #[error("无效的配置: {message}")]
    InvalidConfig { message: String },
}

/// Redis连接测试结果
///
/// 完整描述测试结果的数据结构。
/// 每个字段服务于不同层面的诊断:
/// - success: 快速判断测试通过与否
/// - latency_ms: 性能基准,评估网络和Redis响应速度
/// - message: 人类可读的结果描述
/// - error: 失败时的具体错误信息
#[derive(Debug, Serialize, Deserialize)]
pub struct RedisConnectionTestResult {
    /// 测试是否成功
    pub success: bool,

    /// 连接延迟 (毫秒)
    ///
    /// 测量从发起连接到收到PONG响应的总耗时。
    /// 包含: 网络RTT + Redis处理时间 + 认证时间
    pub latency_ms: Option<u64>,

    /// 结果描述消息
    pub message: String,

    /// 错误详情 (仅在失败时)
    pub error: Option<String>,
}

impl RedisConnectionTestResult {
    /// 创建成功结果
    fn success(latency_ms: u64) -> Self {
        Self {
            success: true,
            latency_ms: Some(latency_ms),
            message: format!("连接成功 (延迟: {}ms)", latency_ms),
            error: None,
        }
    }

    /// 创建失败结果
    fn failure(error: RedisTestError) -> Self {
        Self {
            success: false,
            latency_ms: None,
            message: "连接测试失败".to_string(),
            error: Some(error.to_string()),
        }
    }
}

/// 测试Redis连接
///
/// # 功能
/// 1. 建立连接: 根据提供的配置创建Redis客户端
/// 2. 执行PING: 发送PING命令验证连接可用性
/// 3. 测量延迟: 记录从连接到响应的总耗时
///
/// # 验证层次
/// - 网络连通性: 能否到达Redis服务器
/// - 认证正确性: 密码是否正确 (如配置了密码)
/// - 服务可用性: Redis是否正常响应命令
///
/// # 超时保护
/// - 总超时: 5秒 (防止长时间阻塞UI)
/// - 包含: 连接建立 + 认证 + PING执行
///
/// # 使用场景
/// - 配置验证: 用户填写配置后立即测试
/// - 健康检查: 定期验证现有连接是否正常
/// - 故障诊断: 连接失败时提供详细错误信息
///
/// # 示例
/// ```rust
/// let config = RedisConfig::new("localhost".to_string(), 6379)
///     .with_password("mypass".to_string());
/// let result = test_redis_connection(config).await;
/// ```
#[tauri::command]
pub async fn test_redis_connection(
    config: RedisConfig,
) -> Result<RedisConnectionTestResult, RedisTestError> {
    tracing::info!(
        config = %config.summary_for_logging(),
        "开始Redis连接测试"
    );

    let start = Instant::now();

    // 建立连接
    let client = redis::Client::open(config.to_connection_url()).map_err(|e| {
        tracing::error!(error = %e, "创建Redis客户端失败");
        RedisTestError::InvalidConfig {
            message: e.to_string(),
        }
    })?;

    // 获取异步连接
    let mut conn = match tokio::time::timeout(
        std::time::Duration::from_secs(5),
        client.get_multiplexed_async_connection(),
    )
    .await
    {
        Ok(Ok(conn)) => conn,
        Ok(Err(e)) => {
            tracing::error!(error = %e, "连接Redis失败");
            return Err(classify_connection_error(e));
        }
        Err(_) => {
            tracing::error!("连接Redis超时 (5秒)");
            return Err(RedisTestError::Timeout { timeout_ms: 5000 });
        }
    };

    // 执行PING命令
    let pong: String = redis::cmd("PING")
        .query_async(&mut conn)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "PING命令失败");
            RedisTestError::PingFailed {
                message: e.to_string(),
            }
        })?;

    let latency = start.elapsed().as_millis() as u64;

    // 验证响应
    if pong != "PONG" {
        tracing::error!(response = %pong, "PING响应异常");
        return Err(RedisTestError::PingFailed {
            message: format!("期望 'PONG', 收到 '{}'", pong),
        });
    }

    tracing::info!(
        latency_ms = %latency,
        "Redis连接测试成功"
    );

    Ok(RedisConnectionTestResult::success(latency))
}

/// 分类连接错误
///
/// 将Redis底层错误映射到业务语义错误。
/// 帮助用户快速定位问题根因:
/// - 认证失败 → 检查密码配置
/// - 连接拒绝 → 检查服务器状态和防火墙
/// - 超时 → 检查网络连通性
fn classify_connection_error(err: redis::RedisError) -> RedisTestError {
    let err_msg = err.to_string().to_lowercase();

    // 认证错误
    if err_msg.contains("auth") || err_msg.contains("noauth") || err_msg.contains("wrongpass") {
        return RedisTestError::AuthenticationFailed {
            message: "密码错误或未提供密码".to_string(),
        };
    }

    // 连接错误
    if err_msg.contains("connection refused")
        || err_msg.contains("could not connect")
        || err_msg.contains("no connection")
    {
        return RedisTestError::ConnectionFailed {
            message: "无法连接到Redis服务器,请检查地址、端口和服务状态".to_string(),
        };
    }

    // 超时错误
    if err_msg.contains("timeout") || err_msg.contains("timed out") {
        return RedisTestError::Timeout { timeout_ms: 5000 };
    }

    // 默认连接失败
    RedisTestError::ConnectionFailed {
        message: err.to_string(),
    }
}

/// 保存Redis配置到 .env 文件
///
/// # 功能
/// 1. 更新 .env 文件中的 Redis 配置项
/// 2. 保持其他配置项不变
/// 3. 密码字段在日志中不显示明文
///
/// # 参数
/// - `config`: 待保存的 Redis 配置
///
/// # 错误处理
/// - 文件无法写入时返回 IoError
/// - 配置验证失败时返回 InvalidConfig
///
/// # 使用场景
/// - 用户在UI中配置Redis连接信息后保存
/// - 应用首次启动时初始化默认配置
///
/// # 示例
/// ```rust
/// let config = RedisConfig::new("localhost".to_string(), 6379)
///     .with_password("secret".to_string());
/// save_redis_config(config).await?;
/// ```
#[tauri::command]
pub async fn save_redis_config(config: RedisConfig) -> Result<(), RedisConfigError> {
    tracing::info!(
        config = %config.summary_for_logging(),
        "保存 Redis 配置到 .env 文件"
    );

    ConfigService::save_redis_config(&config)?;

    tracing::info!("Redis 配置已保存");
    Ok(())
}

/// 从 .env 文件加载Redis配置
///
/// # 功能
/// 1. 读取 .env 文件中的 Redis 配置项
/// 2. 解析并验证配置参数
/// 3. 文件不存在时返回默认配置
///
/// # 环境变量
/// - REDIS_HOST: Redis服务器地址 (默认: localhost)
/// - REDIS_PORT: Redis端口 (默认: 6379)
/// - REDIS_PASSWORD: 认证密码 (可选)
/// - REDIS_DATABASE: 数据库索引 (可选, 0-15)
///
/// # 错误处理
/// - 文件读取失败时返回 IoError
/// - 端口/数据库索引格式错误时返回 InvalidUrl
///
/// # 使用场景
/// - 应用启动时加载配置
/// - 用户在UI中查看当前配置
/// - 配置页面初始化默认值
///
/// # 示例
/// ```rust
/// let config = load_redis_config().await?;
/// println!("当前配置: {}", config.summary_for_logging());
/// ```
#[tauri::command]
pub async fn load_redis_config() -> Result<RedisConfig, RedisConfigError> {
    tracing::info!("从 .env 文件加载 Redis 配置");

    let config = ConfigService::load_redis_config()?;

    tracing::info!(
        config = %config.summary_for_logging(),
        "Redis 配置已加载"
    );

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_result_success() {
        let result = RedisConnectionTestResult::success(42);
        assert!(result.success);
        assert_eq!(result.latency_ms, Some(42));
        assert!(result.message.contains("42ms"));
        assert!(result.error.is_none());
    }

    #[test]
    fn test_connection_result_failure() {
        let error = RedisTestError::ConnectionFailed {
            message: "连接被拒绝".to_string(),
        };
        let result = RedisConnectionTestResult::failure(error);
        assert!(!result.success);
        assert!(result.latency_ms.is_none());
        assert!(result.error.is_some());
    }

    #[test]
    fn test_classify_auth_error() {
        let redis_err = redis::RedisError::from((
            redis::ErrorKind::ResponseError,
            "NOAUTH Authentication required",
        ));
        let classified = classify_connection_error(redis_err);
        assert!(matches!(
            classified,
            RedisTestError::AuthenticationFailed { .. }
        ));
    }

    #[test]
    fn test_classify_connection_refused() {
        let redis_err =
            redis::RedisError::from((redis::ErrorKind::IoError, "Connection refused"));
        let classified = classify_connection_error(redis_err);
        assert!(matches!(
            classified,
            RedisTestError::ConnectionFailed { .. }
        ));
    }
}
