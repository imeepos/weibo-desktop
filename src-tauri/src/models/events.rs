use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::models::{CookiesData, QrCodeStatus};

/// 登录状态更新事件
///
/// 从后台监控任务推送到前端的状态快照
/// 每个字段都承载即时状态,用于UI更新和业务决策
#[derive(Debug, Clone, Serialize)]
pub struct LoginStatusEvent {
    /// 二维码会话ID
    pub qr_id: String,

    /// 当前状态
    pub status: QrCodeStatus,

    /// Cookies数据 (仅在 confirmed 时存在)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cookies: Option<CookiesData>,

    /// 状态更新时间
    pub updated_at: DateTime<Utc>,

    /// 二维码是否已自动刷新
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qr_refreshed: Option<bool>,

    /// 新的二维码图片 (仅在 qr_refreshed 时存在)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qr_image: Option<String>,

    /// 原始Playwright返回的retcode (透传)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retcode: Option<i32>,

    /// 原始Playwright返回的msg (透传)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub msg: Option<String>,

    /// 原始Playwright返回的data (透传)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl LoginStatusEvent {
    /// 创建新的状态事件
    pub fn new(qr_id: String, status: QrCodeStatus, cookies: Option<CookiesData>) -> Self {
        Self {
            qr_id,
            status,
            cookies,
            updated_at: Utc::now(),
            qr_refreshed: None,
            qr_image: None,
            retcode: None,
            msg: None,
            data: None,
        }
    }

    /// 创建带Playwright原始数据的状态事件
    pub fn with_raw_data(
        qr_id: String,
        status: QrCodeStatus,
        cookies: Option<CookiesData>,
        retcode: Option<i32>,
        msg: Option<String>,
        data: Option<serde_json::Value>,
    ) -> Self {
        Self {
            qr_id,
            status,
            cookies,
            updated_at: Utc::now(),
            qr_refreshed: None,
            qr_image: None,
            retcode,
            msg,
            data,
        }
    }
}

/// 登录错误事件
///
/// 监控任务遇到错误时推送到前端
#[derive(Debug, Clone, Serialize)]
pub struct LoginErrorEvent {
    /// 二维码会话ID
    pub qr_id: String,

    /// 错误类型
    pub error_type: String,

    /// 错误消息
    pub message: String,

    /// 错误发生时间
    pub timestamp: DateTime<Utc>,
}

impl LoginErrorEvent {
    pub fn new(qr_id: String, error_type: String, message: String) -> Self {
        Self {
            qr_id,
            error_type,
            message,
            timestamp: Utc::now(),
        }
    }
}
