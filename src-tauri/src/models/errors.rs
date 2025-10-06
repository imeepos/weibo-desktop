use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// API调用相关错误
///
/// 处理与微博API交互时的各种失败场景。
/// 每个错误都包含足够的上下文信息,帮助调试和恢复。
#[derive(Debug, Error, Serialize, Deserialize)]
#[serde(tag = "error", content = "details")]
pub enum ApiError {
    /// 网络请求失败
    ///
    /// 可能原因:
    /// - 网络连接中断
    /// - 微博服务器不可达
    /// - DNS解析失败
    #[error("网络请求失败: {0}")]
    NetworkFailed(String),

    /// 二维码未找到
    ///
    /// 提供的 qr_id 在微博API中不存在或已被清理
    #[error("二维码未找到: {qr_id}")]
    QrCodeNotFound { qr_id: String },

    /// 二维码已过期
    ///
    /// 二维码超过有效期(通常为180秒)
    #[error("二维码已过期")]
    QrCodeExpired {
        generated_at: DateTime<Utc>,
        expired_at: DateTime<Utc>,
    },

    /// 响应格式无效
    ///
    /// 微博API返回的数据格式不符合预期
    #[error("响应格式无效: {0}")]
    InvalidResponse(String),

    /// 二维码生成失败
    ///
    /// 微博API返回了非预期的响应或拒绝了请求
    #[error("二维码生成失败: {0}")]
    QrCodeGenerationFailed(String),

    /// 轮询状态检查失败
    ///
    /// 检查二维码扫描状态时出错
    #[error("轮询失败: {0}")]
    PollingFailed(String),

    /// 触发速率限制
    ///
    /// 微博API返回429或类似的限流响应
    #[error("请求过于频繁,已被限流")]
    RateLimitExceeded { retry_after: Option<u64> },

    /// JSON解析失败
    ///
    /// 微博API返回的数据格式不符合预期
    #[error("响应数据解析失败: {0}")]
    JsonParseFailed(String),

    /// HTTP状态码错误
    ///
    /// 微博API返回了非200状态码
    #[error("HTTP错误 {status}: {message}")]
    HttpStatusError { status: u16, message: String },

    /// 依赖安装错误
    ///
    /// 依赖安装过程中出现的错误
    #[error("依赖安装错误: {:?} - {}", error_type, details)]
    InstallError {
        /// 安装错误类型
        error_type: InstallErrorType,
        /// 详细错误信息
        details: String,
    },

    /// 浏览器操作错误
    ///
    /// Chromium 浏览器启动、操作或通信失败
    #[error("浏览器错误: {0}")]
    BrowserError(String),
}

/// Cookies验证相关错误
///
/// 处理Cookies有效性验证过程中的失败场景
#[derive(Debug, Error, Serialize, Deserialize)]
#[serde(tag = "error", content = "details")]
pub enum ValidationError {
    /// 调用个人资料API失败
    ///
    /// 使用cookies访问/api/profile端点失败
    #[error("个人资料API调用失败 (状态码 {status}): {message}")]
    ProfileApiFailed { status: u16, message: String },

    /// 缺少必需的cookie字段
    ///
    /// Cookies中缺少关键字段,无法通过验证
    #[error("缺少必需的cookie字段: {0}")]
    MissingCookie(String),

    /// Playwright执行失败
    ///
    /// 浏览器自动化脚本执行出错
    #[error("Playwright脚本执行失败: {0}")]
    PlaywrightFailed(String),

    /// Cookies格式无效
    ///
    /// Cookies字符串格式不符合预期
    #[error("Cookies格式无效: {0}")]
    InvalidFormat(String),

    /// UID提取失败
    ///
    /// 无法从个人资料API响应中提取用户UID
    #[error("无法提取用户UID: {0}")]
    UidExtractionFailed(String),
}

/// Redis存储相关错误
///
/// 处理与Redis交互时的失败场景
#[derive(Debug, Error, Serialize, Deserialize)]
#[serde(tag = "error", content = "details")]
pub enum StorageError {
    /// Redis连接失败
    ///
    /// 无法建立或维持与Redis服务器的连接
    #[error("Redis连接失败: {0}")]
    RedisConnectionFailed(String),

    /// 指定UID的Cookies未找到
    ///
    /// Redis中不存在该用户的cookies记录
    #[error("未找到UID {0} 的Cookies")]
    NotFound(String),

    /// 序列化/反序列化失败
    ///
    /// 将数据转换为JSON或从JSON解析失败
    #[error("数据序列化失败: {0}")]
    SerializationError(String),

    /// Redis操作超时
    ///
    /// Redis命令执行超过了预设的超时时间
    #[error("Redis操作超时: {0}")]
    OperationTimeout(String),

    /// Redis命令执行失败
    ///
    /// 具体的Redis命令(GET/SET/DEL等)执行出错
    #[error("Redis命令执行失败: {0}")]
    CommandFailed(String),
}

/// 实现从reqwest::Error到ApiError的转换
impl From<reqwest::Error> for ApiError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            ApiError::NetworkFailed("请求超时".to_string())
        } else if err.is_connect() {
            ApiError::NetworkFailed("无法连接到服务器".to_string())
        } else {
            ApiError::NetworkFailed(err.to_string())
        }
    }
}

/// 实现从redis::RedisError到StorageError的转换
impl From<redis::RedisError> for StorageError {
    fn from(err: redis::RedisError) -> Self {
        if err.is_connection_refusal() {
            StorageError::RedisConnectionFailed("连接被拒绝".to_string())
        } else if err.is_timeout() {
            StorageError::OperationTimeout(err.to_string())
        } else {
            StorageError::CommandFailed(err.to_string())
        }
    }
}

/// 实现从serde_json::Error到相关错误的转换
impl From<serde_json::Error> for ApiError {
    fn from(err: serde_json::Error) -> Self {
        ApiError::JsonParseFailed(err.to_string())
    }
}

impl From<serde_json::Error> for StorageError {
    fn from(err: serde_json::Error) -> Self {
        StorageError::SerializationError(err.to_string())
    }
}

/// 依赖项管理相关错误
///
/// 处理依赖检查、安装过程中的失败场景
#[derive(Debug, Error, Serialize, Deserialize)]
#[serde(tag = "error", content = "details")]
pub enum DependencyError {
    /// 依赖检查失败
    ///
    /// 检查依赖项时出现的错误，包含具体的失败原因
    #[error("Dependency check failed: {0}")]
    CheckFailed(String),

    /// 依赖项不支持自动安装
    ///
    /// 尝试安装 auto_installable=false 的依赖
    #[error("Dependency '{0}' cannot be auto-installed. Please install manually.")]
    NotAutoInstallable(String),

    /// 安装失败
    ///
    /// 安装过程中出现的各种错误，分类为不同的错误类型
    #[error("Installation failed: {0}")]
    InstallFailed(InstallErrorType),

    /// 依赖已满足
    ///
    /// 依赖已满足且 force=false 时返回
    #[error("Dependency '{0}' is already satisfied (version {1})")]
    AlreadySatisfied(String, String),

    /// 依赖项未找到
    ///
    /// 请求的 dependency_id 不存在
    #[error("Dependency '{0}' not found")]
    NotFound(String),
}

/// 安装错误类型
///
/// 细分安装失败的具体原因，便于前端针对性处理
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallErrorType {
    /// 网络错误
    ///
    /// 下载依赖包时网络超时、连接失败、DNS解析错误
    NetworkError,

    /// 权限错误
    ///
    /// 安装目录需要管理员权限 (如写入 /usr/local/bin)
    PermissionDenied,

    /// 磁盘空间不足
    ///
    /// 磁盘剩余空间不足
    DiskSpaceError,

    /// 版本冲突
    ///
    /// 系统已存在不兼容的版本且无法覆盖
    VersionConflict,

    /// 未知错误
    ///
    /// 未分类的安装失败 (脚本执行异常、依赖关系解析失败)
    UnknownError,

    /// 命令执行失败
    ///
    /// 安装命令返回非零退出码
    CommandFailed,

    /// 安装超时
    ///
    /// 安装过程超过预设的超时时间
    TimeoutExpired,

    /// 不支持的操作
    ///
    /// 尝试执行不支持的操作 (如安装不支持自动安装的依赖)
    UnsupportedOperation,

    /// 无效输入
    ///
    /// 提供的参数或配置无效
    InvalidInput,
}

impl std::fmt::Display for InstallErrorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InstallErrorType::NetworkError => write!(f, "Network error"),
            InstallErrorType::PermissionDenied => write!(f, "Permission denied"),
            InstallErrorType::DiskSpaceError => write!(f, "Disk space error"),
            InstallErrorType::VersionConflict => write!(f, "Version conflict"),
            InstallErrorType::UnknownError => write!(f, "Unknown error"),
            InstallErrorType::CommandFailed => write!(f, "Command failed"),
            InstallErrorType::TimeoutExpired => write!(f, "Timeout expired"),
            InstallErrorType::UnsupportedOperation => write!(f, "Unsupported operation"),
            InstallErrorType::InvalidInput => write!(f, "Invalid input"),
        }
    }
}

/// 实现从std::io::Error到DependencyError的转换
impl From<std::io::Error> for DependencyError {
    fn from(err: std::io::Error) -> Self {
        match err.kind() {
            std::io::ErrorKind::PermissionDenied => {
                DependencyError::CheckFailed(format!("Permission denied: {}", err))
            }
            std::io::ErrorKind::NotFound => {
                DependencyError::CheckFailed(format!("File or directory not found: {}", err))
            }
            std::io::ErrorKind::ConnectionRefused => {
                DependencyError::CheckFailed(format!("Connection refused: {}", err))
            }
            std::io::ErrorKind::TimedOut => {
                DependencyError::CheckFailed(format!("Connection timed out: {}", err))
            }
            std::io::ErrorKind::UnexpectedEof => {
                DependencyError::CheckFailed(format!("Unexpected end of file: {}", err))
            }
            _ => {
                DependencyError::CheckFailed(format!("I/O error: {}", err))
            }
        }
    }
}
