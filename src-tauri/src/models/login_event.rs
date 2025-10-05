use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 登录事件 (用于前端通知和日志追踪)
///
/// 记录登录流程中的所有关键事件,提供完整的审计追踪链。
/// 每个字段都不可替代,服务于问题诊断、性能分析和用户体验。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginEvent {
    /// 事件类型
    pub event_type: LoginEventType,

    /// 时间戳
    pub timestamp: DateTime<Utc>,

    /// 会话ID (qr_id)
    pub session_id: String,

    /// 用户ID (可选,仅在获取到cookies后有值)
    pub uid: Option<String>,

    /// 额外详情 (JSON格式,灵活扩展)
    pub details: Value,
}

/// 登录事件类型
///
/// 覆盖登录流程的所有关键节点:
/// - 成功路径: Generated -> Scanned -> ConfirmedSuccess -> ValidationSuccess
/// - 失败路径: Error, QrCodeExpired
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoginEventType {
    /// 二维码生成成功
    QrCodeGenerated,

    /// 二维码已扫描
    QrCodeScanned,

    /// 确认登录成功
    ConfirmedSuccess,

    /// Cookies验证成功
    ValidationSuccess,

    /// 二维码已过期
    QrCodeExpired,

    /// 发生错误 (网络、API、验证等)
    Error,
}

impl LoginEvent {
    /// 创建新的登录事件
    ///
    /// # 参数
    /// - `event_type`: 事件类型
    /// - `session_id`: 会话ID (qr_id)
    /// - `uid`: 用户ID (可选)
    /// - `details`: 额外详情 (JSON格式)
    pub fn new(
        event_type: LoginEventType,
        session_id: String,
        uid: Option<String>,
        details: Value,
    ) -> Self {
        Self {
            event_type,
            timestamp: Utc::now(),
            session_id,
            uid,
            details,
        }
    }

    /// 创建二维码生成事件
    ///
    /// # 参数
    /// - `session_id`: 会话ID
    /// - `expires_in`: 过期时长(秒)
    ///
    /// # 示例
    /// ```
    /// let event = LoginEvent::qr_generated("qr_abc123".to_string(), 180);
    /// ```
    pub fn qr_generated(session_id: String, expires_in: i64) -> Self {
        Self::new(
            LoginEventType::QrCodeGenerated,
            session_id,
            None,
            serde_json::json!({ "expires_in": expires_in }),
        )
    }

    /// 创建扫码事件
    ///
    /// # 参数
    /// - `session_id`: 会话ID
    pub fn qr_scanned(session_id: String) -> Self {
        Self::new(
            LoginEventType::QrCodeScanned,
            session_id,
            None,
            serde_json::json!({}),
        )
    }

    /// 创建确认成功事件
    ///
    /// # 参数
    /// - `session_id`: 会话ID
    /// - `uid`: 用户ID
    pub fn confirmed_success(session_id: String, uid: String) -> Self {
        Self::new(
            LoginEventType::ConfirmedSuccess,
            session_id,
            Some(uid.clone()),
            serde_json::json!({ "uid": uid }),
        )
    }

    /// 创建验证成功事件
    ///
    /// # 参数
    /// - `session_id`: 会话ID
    /// - `uid`: 用户ID
    /// - `screen_name`: 用户昵称
    pub fn validation_success(session_id: String, uid: String, screen_name: String) -> Self {
        Self::new(
            LoginEventType::ValidationSuccess,
            session_id,
            Some(uid.clone()),
            serde_json::json!({
                "uid": uid,
                "screen_name": screen_name
            }),
        )
    }

    /// 创建过期事件
    ///
    /// # 参数
    /// - `session_id`: 会话ID
    pub fn qr_expired(session_id: String) -> Self {
        Self::new(
            LoginEventType::QrCodeExpired,
            session_id,
            None,
            serde_json::json!({}),
        )
    }

    /// 创建错误事件
    ///
    /// # 参数
    /// - `session_id`: 会话ID
    /// - `error_message`: 错误消息
    pub fn error(session_id: String, error_message: String) -> Self {
        Self::new(
            LoginEventType::Error,
            session_id,
            None,
            serde_json::json!({ "error": error_message }),
        )
    }

    /// 创建带详情的错误事件
    ///
    /// # 参数
    /// - `session_id`: 会话ID
    /// - `error_type`: 错误类型
    /// - `error_message`: 错误消息
    /// - `additional_details`: 额外详情
    pub fn error_with_details(
        session_id: String,
        error_type: &str,
        error_message: String,
        additional_details: Value,
    ) -> Self {
        Self::new(
            LoginEventType::Error,
            session_id,
            None,
            serde_json::json!({
                "error_type": error_type,
                "error": error_message,
                "details": additional_details
            }),
        )
    }

    /// 检查是否为成功事件
    pub fn is_success(&self) -> bool {
        matches!(
            self.event_type,
            LoginEventType::QrCodeGenerated
                | LoginEventType::QrCodeScanned
                | LoginEventType::ConfirmedSuccess
                | LoginEventType::ValidationSuccess
        )
    }

    /// 检查是否为失败事件
    pub fn is_failure(&self) -> bool {
        matches!(
            self.event_type,
            LoginEventType::QrCodeExpired | LoginEventType::Error
        )
    }

    /// 获取事件的严重程度级别
    ///
    /// 用于日志输出时确定日志级别:
    /// - Error -> ERROR
    /// - QrCodeExpired -> WARN
    /// - 其他 -> INFO
    pub fn severity(&self) -> &'static str {
        match self.event_type {
            LoginEventType::Error => "ERROR",
            LoginEventType::QrCodeExpired => "WARN",
            _ => "INFO",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qr_generated() {
        let event = LoginEvent::qr_generated("qr_123".to_string(), 180);
        assert_eq!(event.event_type, LoginEventType::QrCodeGenerated);
        assert_eq!(event.session_id, "qr_123");
        assert!(event.uid.is_none());
        assert_eq!(event.details["expires_in"], 180);
        assert!(event.is_success());
        assert!(!event.is_failure());
    }

    #[test]
    fn test_qr_scanned() {
        let event = LoginEvent::qr_scanned("qr_123".to_string());
        assert_eq!(event.event_type, LoginEventType::QrCodeScanned);
        assert_eq!(event.session_id, "qr_123");
        assert!(event.uid.is_none());
        assert!(event.is_success());
    }

    #[test]
    fn test_confirmed_success() {
        let event = LoginEvent::confirmed_success("qr_123".to_string(), "uid_456".to_string());
        assert_eq!(event.event_type, LoginEventType::ConfirmedSuccess);
        assert_eq!(event.session_id, "qr_123");
        assert_eq!(event.uid, Some("uid_456".to_string()));
        assert_eq!(event.details["uid"], "uid_456");
        assert!(event.is_success());
    }

    #[test]
    fn test_validation_success() {
        let event = LoginEvent::validation_success(
            "qr_123".to_string(),
            "uid_456".to_string(),
            "张三".to_string(),
        );
        assert_eq!(event.event_type, LoginEventType::ValidationSuccess);
        assert_eq!(event.uid, Some("uid_456".to_string()));
        assert_eq!(event.details["screen_name"], "张三");
        assert!(event.is_success());
    }

    #[test]
    fn test_qr_expired() {
        let event = LoginEvent::qr_expired("qr_123".to_string());
        assert_eq!(event.event_type, LoginEventType::QrCodeExpired);
        assert_eq!(event.session_id, "qr_123");
        assert!(event.is_failure());
        assert_eq!(event.severity(), "WARN");
    }

    #[test]
    fn test_error() {
        let event = LoginEvent::error("qr_123".to_string(), "网络错误".to_string());
        assert_eq!(event.event_type, LoginEventType::Error);
        assert_eq!(event.session_id, "qr_123");
        assert_eq!(event.details["error"], "网络错误");
        assert!(event.is_failure());
        assert_eq!(event.severity(), "ERROR");
    }

    #[test]
    fn test_error_with_details() {
        let additional = serde_json::json!({ "retry_count": 3, "last_attempt": "2025-10-05T10:30:00Z" });
        let event = LoginEvent::error_with_details(
            "qr_123".to_string(),
            "NetworkError",
            "连接超时".to_string(),
            additional,
        );
        assert_eq!(event.event_type, LoginEventType::Error);
        assert_eq!(event.details["error_type"], "NetworkError");
        assert_eq!(event.details["error"], "连接超时");
        assert_eq!(event.details["details"]["retry_count"], 3);
    }

    #[test]
    fn test_severity_levels() {
        let generated = LoginEvent::qr_generated("qr_123".to_string(), 180);
        let expired = LoginEvent::qr_expired("qr_123".to_string());
        let error = LoginEvent::error("qr_123".to_string(), "错误".to_string());

        assert_eq!(generated.severity(), "INFO");
        assert_eq!(expired.severity(), "WARN");
        assert_eq!(error.severity(), "ERROR");
    }
}
