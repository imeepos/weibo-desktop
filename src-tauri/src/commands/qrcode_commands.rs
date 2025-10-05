use crate::models::{ApiError, CookiesData, LoginSession, QrCodeStatus};
use crate::state::AppState;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tauri::State;

/// 生成二维码响应
///
/// 契约定义: specs/001-cookies/contracts/generate_qrcode.md:40
/// 四个顶层字段,直接对应前端需求:
/// - qr_id: 唯一标识,用于后续轮询
/// - qr_image: base64图片,直接用于<img>标签
/// - expires_at: ISO 8601时间戳,过期时刻
/// - expires_in: 秒数,用于倒计时
#[derive(Debug, Serialize, Deserialize)]
pub struct QrCodeResponse {
    /// 二维码唯一标识,用于后续轮询
    pub qr_id: String,

    /// Base64编码的二维码图片 (PNG格式)
    pub qr_image: String,

    /// 二维码过期时间 (ISO 8601格式)
    pub expires_at: DateTime<Utc>,

    /// 有效期秒数 (通常为180秒)
    pub expires_in: u64,
}

/// 生成二维码命令
///
/// 前端调用入口,返回可扫描的二维码。
/// 无参数 - 简约设计,应用配置已包含所需的 app_key。
///
/// 返回结构化数据:
/// - 成功: QrCodeResponse (qr_id, qr_image, expires_at, expires_in)
/// - 失败: ApiError 枚举 (NetworkFailed, InvalidResponse, RateLimitExceeded)
#[tauri::command]
pub async fn generate_qrcode(state: State<'_, AppState>) -> Result<QrCodeResponse, ApiError> {
    tracing::info!("generate_qrcode command called");

    // 调用微博API生成二维码
    let (session, qr_image) = state.weibo_api.generate_qrcode().await?;

    let expires_in = session.remaining_seconds().max(0) as u64;

    tracing::info!(
        qr_id = %session.qr_id,
        expires_in = %expires_in,
        "QR code generated successfully"
    );

    Ok(QrCodeResponse {
        qr_id: session.qr_id,
        qr_image,
        expires_at: session.expires_at,
        expires_in,
    })
}

/// 轮询登录状态响应
///
/// 契约定义: specs/001-cookies/contracts/poll_login_status.md:47
/// 三个字段,完整描述当前状态:
/// - status: 当前状态 (pending, scanned, confirmed, expired)
/// - cookies: Cookies数据 (仅在 confirmed 时存在)
/// - updated_at: 状态更新时间
#[derive(Debug, Serialize, Deserialize)]
pub struct LoginStatusResponse {
    /// 当前状态
    pub status: QrCodeStatus,

    /// Cookies数据 (仅在 status === 'confirmed' 时存在)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cookies: Option<CookiesData>,

    /// 状态更新时间
    pub updated_at: DateTime<Utc>,
}

/// 轮询登录状态命令
///
/// 单次轮询调用,检查二维码状态并返回当前状态快照。
/// 前端负责轮询策略 (间隔2-5秒,exponential backoff)。
///
/// 状态转换:
/// Pending -> Scanned -> Confirmed
///     |          |              |
///     +----------+--------------+---> Expired
///
/// 获取cookies后自动执行验证和存储:
/// 1. Playwright验证有效性
/// 2. 保存到Redis
/// 3. 返回confirmed状态和完整cookies
///
/// 返回:
/// - 成功: LoginStatusResponse (status, cookies?, updated_at)
/// - 失败: ApiError (QrCodeNotFound, QrCodeExpired, NetworkFailed)
#[tauri::command]
pub async fn poll_login_status(
    qr_id: String,
    state: State<'_, AppState>,
) -> Result<LoginStatusResponse, ApiError> {
    tracing::debug!(qr_id = %qr_id, "poll_login_status command called");

    // 创建会话追踪
    let mut session = LoginSession::new(qr_id.clone(), 180);

    // 检查过期
    if session.is_expired() {
        tracing::warn!(qr_id = %qr_id, "QR code expired");
        return Err(ApiError::QrCodeExpired {
            generated_at: session.created_at,
            expired_at: session.expires_at,
        });
    }

    // 检查状态
    match state.weibo_api.check_qrcode_status(&mut session).await {
        Ok(Some((uid, cookies))) => {
            // 确认成功,获取到cookies
            tracing::info!(qr_id = %qr_id, uid = %uid, "Login confirmed");

            // 验证cookies
            let (validated_uid, screen_name) = state
                .validator
                .validate_cookies(&cookies)
                .await
                .map_err(|e| ApiError::InvalidResponse(format!("Validation failed: {}", e)))?;

            // 确保UID匹配
            if validated_uid != uid {
                return Err(ApiError::InvalidResponse(format!(
                    "UID mismatch: expected {}, got {}",
                    uid, validated_uid
                )));
            }

            // 保存到Redis
            let mut cookies_data = CookiesData::new(validated_uid.clone(), cookies);
            cookies_data = cookies_data.with_screen_name(screen_name.clone());

            // 验证CookiesData结构
            cookies_data
                .validate()
                .map_err(|e| ApiError::InvalidResponse(format!("Validation failed: {}", e)))?;

            state
                .redis
                .save_cookies(&cookies_data)
                .await
                .map_err(|e| ApiError::InvalidResponse(format!("Storage failed: {}", e)))?;

            tracing::info!(
                qr_id = %qr_id,
                uid = %validated_uid,
                screen_name = %screen_name,
                "Cookies validated and saved successfully"
            );

            // 返回confirmed状态和完整cookies
            Ok(LoginStatusResponse {
                status: QrCodeStatus::Confirmed,
                cookies: Some(cookies_data),
                updated_at: Utc::now(),
            })
        }
        Ok(None) => {
            // 状态未变化或仅扫描
            let current_status = session.status;

            // 状态变化时记录日志
            if current_status == QrCodeStatus::Scanned {
                tracing::info!(qr_id = %qr_id, "QR code scanned");
            }

            Ok(LoginStatusResponse {
                status: current_status,
                cookies: None,
                updated_at: Utc::now(),
            })
        }
        Err(e) => {
            tracing::error!(qr_id = %qr_id, error = ?e, "Poll failed");
            Err(e)
        }
    }
}
