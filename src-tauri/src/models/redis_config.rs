use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Redis配置错误
///
/// 处理Redis连接配置过程中的失败场景。
/// 每个错误都表达清晰的失败原因,便于诊断和恢复。
#[derive(Debug, Error, Serialize, Deserialize)]
#[serde(tag = "error", content = "details")]
pub enum RedisConfigError {
    /// 连接失败
    ///
    /// 无法建立与Redis服务器的连接
    /// 可能原因: 服务器未启动、网络不可达、防火墙阻断
    #[error("Redis连接失败: {0}")]
    ConnectionFailed(String),

    /// 无效的连接URL
    ///
    /// 生成的Redis URL格式不正确或包含非法字符
    #[error("无效的Redis URL: {0}")]
    InvalidUrl(String),

    /// 配置未找到
    ///
    /// 无法加载或读取Redis配置文件
    #[error("Redis配置未找到: {0}")]
    ConfigNotFound(String),

    /// I/O错误
    ///
    /// 读取或写入配置文件时的文件系统错误
    #[error("I/O错误: {0}")]
    IoError(String),
}

/// Redis连接配置
///
/// 封装Redis连接所需的全部参数。
/// 每个字段都服务于连接建立、安全认证和数据隔离。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    /// Redis服务器主机地址
    ///
    /// 示例: "localhost", "127.0.0.1", "redis.example.com"
    pub host: String,

    /// Redis服务器端口
    ///
    /// 默认: 6379
    pub port: u16,

    /// 认证密码 (可选)
    ///
    /// 如果Redis配置了 `requirepass`,此字段必需
    pub password: Option<String>,

    /// 数据库索引 (可选)
    ///
    /// Redis支持0-15共16个数据库,默认使用0
    /// 用于逻辑隔离不同环境的数据 (如开发/测试/生产)
    pub database: Option<u8>,
}

impl RedisConfig {
    /// 创建新的Redis配置
    ///
    /// # 参数
    /// - `host`: 服务器地址
    /// - `port`: 服务器端口
    ///
    /// # 示例
    /// ```
    /// use weibo_login::models::RedisConfig;
    ///
    /// let config = RedisConfig::new("localhost".to_string(), 6379);
    /// ```
    pub fn new(host: String, port: u16) -> Self {
        Self {
            host,
            port,
            password: None,
            database: None,
        }
    }

    /// 设置密码 (构建器模式)
    ///
    /// # 示例
    /// ```
    /// use weibo_login::models::RedisConfig;
    ///
    /// let config = RedisConfig::new("localhost".to_string(), 6379)
    ///     .with_password("secret123".to_string());
    /// ```
    pub fn with_password(mut self, password: String) -> Self {
        self.password = Some(password);
        self
    }

    /// 设置数据库索引 (构建器模式)
    ///
    /// # 参数
    /// - `database`: 数据库索引 (0-15)
    ///
    /// # 示例
    /// ```
    /// use weibo_login::models::RedisConfig;
    ///
    /// let config = RedisConfig::new("localhost".to_string(), 6379)
    ///     .with_database(1);
    /// ```
    pub fn with_database(mut self, database: u8) -> Self {
        self.database = Some(database);
        self
    }

    /// 生成Redis连接URL
    ///
    /// 根据配置生成标准的Redis连接字符串。
    ///
    /// # URL格式
    /// - 无密码: `redis://{host}:{port}/{db}`
    /// - 有密码: `redis://:{password}@{host}:{port}/{db}`
    ///
    /// # 示例
    /// ```
    /// use weibo_login::models::RedisConfig;
    ///
    /// let config = RedisConfig::new("localhost".to_string(), 6379)
    ///     .with_password("secret".to_string())
    ///     .with_database(1);
    ///
    /// let url = config.to_connection_url();
    /// assert_eq!(url, "redis://:secret@localhost:6379/1");
    /// ```
    pub fn to_connection_url(&self) -> String {
        let auth = match &self.password {
            Some(pwd) => format!(":{}@", pwd),
            None => String::new(),
        };

        let db = self.database.unwrap_or(0);

        format!(
            "redis://{}{}{}/{}",
            auth,
            self.host,
            format_port(self.port),
            db
        )
    }

    /// 获取配置摘要 (用于日志,不记录密码)
    ///
    /// 遵循安全日志原则: 不泄露敏感信息。
    ///
    /// # 示例
    /// ```
    /// use weibo_login::models::RedisConfig;
    ///
    /// let config = RedisConfig::new("localhost".to_string(), 6379)
    ///     .with_password("secret".to_string());
    ///
    /// let summary = config.summary_for_logging();
    /// // 输出: "localhost:6379/0 (authenticated)"
    /// assert!(!summary.contains("secret"));
    /// ```
    pub fn summary_for_logging(&self) -> String {
        let auth_hint = if self.password.is_some() {
            " (authenticated)"
        } else {
            ""
        };
        format!(
            "{}:{}/{}{}",
            self.host,
            self.port,
            self.database.unwrap_or(0),
            auth_hint
        )
    }
}

/// 格式化端口号
///
/// 如果是默认端口6379,则省略端口号(符合Redis URL惯例)
/// 否则返回 `:port` 格式
fn format_port(port: u16) -> String {
    if port == 6379 {
        String::new()
    } else {
        format!(":{}", port)
    }
}

impl Default for RedisConfig {
    /// 默认配置: localhost:6379, 无密码, 数据库0
    fn default() -> Self {
        Self::new("localhost".to_string(), 6379)
    }
}

impl From<std::io::Error> for RedisConfigError {
    fn from(err: std::io::Error) -> Self {
        use std::io::ErrorKind;
        match err.kind() {
            ErrorKind::NotFound => RedisConfigError::ConfigNotFound(err.to_string()),
            ErrorKind::PermissionDenied => RedisConfigError::IoError(format!("权限不足: {}", err)),
            _ => RedisConfigError::IoError(err.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_redis_config() {
        let config = RedisConfig::new("127.0.0.1".to_string(), 6379);
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 6379);
        assert!(config.password.is_none());
        assert!(config.database.is_none());
    }

    #[test]
    fn test_default_redis_config() {
        let config = RedisConfig::default();
        assert_eq!(config.host, "localhost");
        assert_eq!(config.port, 6379);
        assert!(config.password.is_none());
        assert!(config.database.is_none());
    }

    #[test]
    fn test_builder_pattern() {
        let config = RedisConfig::new("redis.local".to_string(), 6380)
            .with_password("mypass".to_string())
            .with_database(2);

        assert_eq!(config.host, "redis.local");
        assert_eq!(config.port, 6380);
        assert_eq!(config.password, Some("mypass".to_string()));
        assert_eq!(config.database, Some(2));
    }

    #[test]
    fn test_to_connection_url_without_password() {
        let config = RedisConfig::new("localhost".to_string(), 6379);
        assert_eq!(config.to_connection_url(), "redis://localhost/0");
    }

    #[test]
    fn test_to_connection_url_with_password() {
        let config =
            RedisConfig::new("localhost".to_string(), 6379).with_password("secret123".to_string());
        assert_eq!(config.to_connection_url(), "redis://:secret123@localhost/0");
    }

    #[test]
    fn test_to_connection_url_with_custom_port() {
        let config = RedisConfig::new("redis.example.com".to_string(), 6380);
        assert_eq!(
            config.to_connection_url(),
            "redis://redis.example.com:6380/0"
        );
    }

    #[test]
    fn test_to_connection_url_full_config() {
        let config = RedisConfig::new("redis.prod".to_string(), 6380)
            .with_password("prod_pass".to_string())
            .with_database(3);
        assert_eq!(
            config.to_connection_url(),
            "redis://:prod_pass@redis.prod:6380/3"
        );
    }

    #[test]
    fn test_summary_for_logging_without_password() {
        let config = RedisConfig::new("localhost".to_string(), 6379);
        let summary = config.summary_for_logging();
        assert_eq!(summary, "localhost:6379/0");
    }

    #[test]
    fn test_summary_for_logging_with_password() {
        let config =
            RedisConfig::new("localhost".to_string(), 6379).with_password("secret".to_string());
        let summary = config.summary_for_logging();
        assert_eq!(summary, "localhost:6379/0 (authenticated)");
        assert!(!summary.contains("secret"));
    }

    #[test]
    fn test_summary_for_logging_full_config() {
        let config = RedisConfig::new("redis.local".to_string(), 6380)
            .with_password("mypass".to_string())
            .with_database(5);
        let summary = config.summary_for_logging();
        assert_eq!(summary, "redis.local:6380/5 (authenticated)");
        assert!(!summary.contains("mypass"));
    }

    #[test]
    fn test_format_port_default() {
        assert_eq!(format_port(6379), "");
    }

    #[test]
    fn test_format_port_custom() {
        assert_eq!(format_port(6380), ":6380");
        assert_eq!(format_port(7000), ":7000");
    }
}
