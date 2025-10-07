use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 二维码登录会话
///
/// 追踪从二维码生成到确认完成的完整登录流程。
/// 每个字段都不可替代,服务于状态追踪、时序分析和过期判断。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginSession {
    /// 二维码唯一ID (微博API返回)
    pub qr_id: String,

    /// 当前状态
    pub status: QrCodeStatus,

    /// 创建时间
    pub created_at: DateTime<Utc>,

    /// 扫码时间 (可选)
    pub scanned_at: Option<DateTime<Utc>>,

    /// 确认登录时间 (可选)
    pub confirmed_at: Option<DateTime<Utc>>,

    /// 过期时间 (通常为创建后180秒)
    pub expires_at: DateTime<Utc>,
}

/// 二维码状态
///
/// 状态转换流程:
/// Pending -> Scanned -> Confirmed (成功路径)
///     |          |          |
///     |          +---> Rejected (用户拒绝)
///     |
///     +---> Expired (超时/过期)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QrCodeStatus {
    /// 等待扫码
    Pending,

    /// 已扫码,等待确认
    Scanned,

    /// 确认成功
    Confirmed,

    /// 用户拒绝登录
    Rejected,

    /// 已过期
    Expired,
}

impl LoginSession {
    /// 创建新的登录会话
    ///
    /// # 参数
    /// - `qr_id`: 微博API返回的二维码唯一标识
    /// - `expires_in_seconds`: 过期时长(秒),通常为180秒
    ///
    /// # 示例
    /// ```
    /// use weibo_login::models::{LoginSession, QrCodeStatus};
    /// let session = LoginSession::new("qr_abc123".to_string(), 180);
    /// assert_eq!(session.status, QrCodeStatus::Pending);
    /// ```
    pub fn new(qr_id: String, expires_in_seconds: i64) -> Self {
        let now = Utc::now();
        Self {
            qr_id,
            status: QrCodeStatus::Pending,
            created_at: now,
            scanned_at: None,
            confirmed_at: None,
            expires_at: now + chrono::Duration::seconds(expires_in_seconds),
        }
    }

    /// 从绝对时间戳创建登录会话
    ///
    /// # 参数
    /// - `qr_id`: 二维码唯一标识
    /// - `expires_at_millis`: 绝对过期时间戳(毫秒)
    ///
    /// 用于Playwright返回的绝对时间戳,避免时间重复计算导致的偏差
    pub fn from_timestamp(qr_id: String, expires_at_millis: i64) -> Self {
        let now = Utc::now();
        let expires_at = chrono::DateTime::from_timestamp_millis(expires_at_millis)
            .unwrap_or_else(|| now + chrono::Duration::seconds(180));

        Self {
            qr_id,
            status: QrCodeStatus::Pending,
            created_at: now,
            scanned_at: None,
            confirmed_at: None,
            expires_at,
        }
    }

    /// 更新状态为已扫码
    #[allow(dead_code)]
    pub fn mark_scanned(&mut self) {
        self.status = QrCodeStatus::Scanned;
        self.scanned_at = Some(Utc::now());
    }

    /// 更新状态为确认成功
    #[allow(dead_code)]
    pub fn mark_confirmed(&mut self) {
        self.status = QrCodeStatus::Confirmed;
        self.confirmed_at = Some(Utc::now());
    }

    /// 更新状态为已过期
    #[allow(dead_code)]
    pub fn mark_expired(&mut self) {
        self.status = QrCodeStatus::Expired;
    }

    /// 更新状态为已拒绝
    #[allow(dead_code)]
    pub fn mark_rejected(&mut self) {
        self.status = QrCodeStatus::Rejected;
    }

    /// 获取会话持续时长(秒)
    #[allow(dead_code)]
    pub fn duration_seconds(&self) -> i64 {
        (Utc::now() - self.created_at).num_seconds()
    }

    /// 获取距离过期的剩余秒数
    ///
    /// 返回负数表示已过期。用于前端倒计时显示。
    pub fn remaining_seconds(&self) -> i64 {
        (self.expires_at - Utc::now()).num_seconds()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_session_initial_state() {
        let session = LoginSession::new("test_qr_123".to_string(), 180);
        assert_eq!(session.qr_id, "test_qr_123");
        assert_eq!(session.status, QrCodeStatus::Pending);
        assert!(session.scanned_at.is_none());
        assert!(session.confirmed_at.is_none());
    }

    #[test]
    fn test_remaining_seconds() {
        let session = LoginSession::new("test_qr_123".to_string(), 180);
        let remaining = session.remaining_seconds();
        assert!(remaining > 175 && remaining <= 180);
    }
}
