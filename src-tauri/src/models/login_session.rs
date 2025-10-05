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
/// Pending -> Scanned -> ConfirmedSuccess
///     |          |              |
///     +----------+--------------+---> Expired (任何状态超时)
///                |
///                +---> Rejected (用户拒绝)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QrCodeStatus {
    /// 等待扫码
    Pending,

    /// 已扫码,等待确认
    Scanned,

    /// 确认成功
    ConfirmedSuccess,

    /// 已过期
    Expired,

    /// 用户拒绝
    Rejected,
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

    /// 检查是否已过期
    ///
    /// 判断依据:
    /// 1. 当前时间超过 `expires_at`
    /// 2. 状态已标记为 `Expired`
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at || self.status == QrCodeStatus::Expired
    }

    /// 检查是否为终态
    ///
    /// 终态包括:
    /// - `ConfirmedSuccess`: 登录成功
    /// - `Expired`: 二维码过期
    /// - `Rejected`: 用户拒绝
    ///
    /// 一旦进入终态,不应再进行状态轮询。
    pub fn is_final_status(&self) -> bool {
        matches!(
            self.status,
            QrCodeStatus::ConfirmedSuccess | QrCodeStatus::Expired | QrCodeStatus::Rejected
        )
    }

    /// 更新状态为已扫码
    ///
    /// 记录用户扫码的时间点,用于性能分析和用户体验反馈。
    pub fn mark_scanned(&mut self) {
        self.status = QrCodeStatus::Scanned;
        self.scanned_at = Some(Utc::now());
    }

    /// 更新状态为确认成功
    ///
    /// 记录用户确认登录的时间点,标志登录流程成功完成。
    pub fn mark_confirmed(&mut self) {
        self.status = QrCodeStatus::ConfirmedSuccess;
        self.confirmed_at = Some(Utc::now());
    }

    /// 更新状态为已过期
    ///
    /// 当检测到超时或API返回过期状态时调用。
    pub fn mark_expired(&mut self) {
        self.status = QrCodeStatus::Expired;
    }

    /// 更新状态为用户拒绝
    ///
    /// 当API返回用户在手机端拒绝登录时调用。
    pub fn mark_rejected(&mut self) {
        self.status = QrCodeStatus::Rejected;
    }

    /// 获取会话持续时长(秒)
    ///
    /// 从创建到当前时刻的秒数,用于性能监控和SLA统计。
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
    use std::thread::sleep;
    use std::time::Duration;

    #[test]
    fn test_new_session_initial_state() {
        let session = LoginSession::new("test_qr_123".to_string(), 180);
        assert_eq!(session.qr_id, "test_qr_123");
        assert_eq!(session.status, QrCodeStatus::Pending);
        assert!(session.scanned_at.is_none());
        assert!(session.confirmed_at.is_none());
        assert!(!session.is_expired());
        assert!(!session.is_final_status());
    }

    #[test]
    fn test_mark_scanned() {
        let mut session = LoginSession::new("test_qr_123".to_string(), 180);
        session.mark_scanned();
        assert_eq!(session.status, QrCodeStatus::Scanned);
        assert!(session.scanned_at.is_some());
        assert!(!session.is_final_status());
    }

    #[test]
    fn test_mark_confirmed() {
        let mut session = LoginSession::new("test_qr_123".to_string(), 180);
        session.mark_confirmed();
        assert_eq!(session.status, QrCodeStatus::ConfirmedSuccess);
        assert!(session.confirmed_at.is_some());
        assert!(session.is_final_status());
    }

    #[test]
    fn test_mark_expired() {
        let mut session = LoginSession::new("test_qr_123".to_string(), 180);
        session.mark_expired();
        assert_eq!(session.status, QrCodeStatus::Expired);
        assert!(session.is_expired());
        assert!(session.is_final_status());
    }

    #[test]
    fn test_expiry_check() {
        let session = LoginSession::new("test_qr_123".to_string(), 1);
        assert!(!session.is_expired());
        sleep(Duration::from_secs(2));
        assert!(session.is_expired());
    }

    #[test]
    fn test_remaining_seconds() {
        let session = LoginSession::new("test_qr_123".to_string(), 180);
        let remaining = session.remaining_seconds();
        assert!(remaining > 175 && remaining <= 180);
    }
}
