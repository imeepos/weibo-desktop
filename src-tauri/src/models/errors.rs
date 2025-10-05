use thiserror::Error;

/// API调用相关错误
///
/// 处理与微博API交互时的各种失败场景。
/// 每个错误都包含足够的上下文信息,帮助调试和恢复。
#[derive(Debug, Error)]
pub enum ApiError {
    /// 网络请求失败
    ///
    /// 可能原因:
    /// - 网络连接中断
    /// - 微博服务器不可达
    /// - DNS解析失败
    #[error("网络请求失败: {0}")]
    NetworkFailed(String),

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
    #[error("请求过于频繁,已被限流: {0}")]
    RateLimited(String),

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
}

/// Cookies验证相关错误
///
/// 处理Cookies有效性验证过程中的失败场景
#[derive(Debug, Error)]
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
#[derive(Debug, Error)]
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

/// 应用程序整体错误
///
/// 聚合所有子系统的错误,提供统一的错误处理接口
#[derive(Debug, Error)]
pub enum AppError {
    #[error(transparent)]
    Api(#[from] ApiError),

    #[error(transparent)]
    Validation(#[from] ValidationError),

    #[error(transparent)]
    Storage(#[from] StorageError),

    /// 配置错误
    ///
    /// 应用配置缺失或格式错误
    #[error("配置错误: {0}")]
    ConfigError(String),

    /// 内部错误
    ///
    /// 未预期的运行时错误
    #[error("内部错误: {0}")]
    InternalError(String),
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
