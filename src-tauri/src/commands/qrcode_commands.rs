use crate::models::{CookiesData, LoginEvent, LoginSession, QrCodeStatus};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tauri::State;
use tokio::time::sleep;

/// 生成二维码响应
///
/// 契约定义: specs/001-cookies/contracts/generate_qrcode.md
/// 每个字段都对应前端的必要展示:
/// - session: 完整会话信息,包含过期时间和状态
/// - qr_image: base64图片,直接用于<img>标签
#[derive(Debug, Serialize, Deserialize)]
pub struct GenerateQrcodeResponse {
    /// 登录会话信息
    pub session: LoginSession,

    /// 二维码图片 (base64编码的PNG)
    pub qr_image: String,
}

/// 生成二维码命令
///
/// 前端调用入口,返回可扫描的二维码。
/// 无参数 - 简约设计,应用配置已包含所需的 app_key。
///
/// # 错误处理哲学
/// 将所有技术性错误转换为用户可理解的字符串,
/// 前端只需展示,无需解析复杂的错误类型。
#[tauri::command]
pub async fn generate_qrcode(
    state: State<'_, AppState>,
) -> Result<GenerateQrcodeResponse, String> {
    tracing::info!("generate_qrcode command called");

    // 调用微博API生成二维码
    let (session, qr_image) = state
        .weibo_api
        .generate_qrcode()
        .await
        .map_err(|e| format!("Failed to generate QR code: {}", e))?;

    tracing::info!(
        qr_id = %session.qr_id,
        expires_in = %session.remaining_seconds(),
        "QR code generated successfully"
    );

    Ok(GenerateQrcodeResponse { session, qr_image })
}

/// 轮询登录状态响应
///
/// 每次轮询返回的状态快照:
/// - event: 当前登录事件 (扫描/确认/过期/错误)
/// - is_final: 是否已达终态,指导前端是否继续轮询
#[derive(Debug, Serialize, Deserialize)]
pub struct PollStatusResponse {
    /// 登录事件
    pub event: LoginEvent,

    /// 是否为终态 (确认成功或过期/拒绝)
    pub is_final: bool,
}

/// 轮询登录状态命令
///
/// 核心登录流程: 持续轮询直到状态变化或超时。
/// 轮询策略: 每3秒一次,最多60次 (3分钟总时长)。
///
/// # 状态机转换
/// Pending -> Scanned -> Confirmed (成功获取cookies)
///     |          |           |
///     +----------+-----------+---> Expired (超时)
///
/// # 自动化处理
/// 获取cookies后自动执行:
/// 1. Playwright验证有效性
/// 2. 保存到Redis
/// 3. 返回验证成功事件
///
/// 这是优雅的体现 - 前端无需关心验证和存储细节,
/// 命令层编排完整流程,返回最终结果。
#[tauri::command]
pub async fn poll_login_status(
    qr_id: String,
    state: State<'_, AppState>,
) -> Result<PollStatusResponse, String> {
    tracing::debug!(qr_id = %qr_id, "poll_login_status command called");

    // 创建会话追踪
    let mut session = LoginSession::new(qr_id.clone(), 180);

    // 轮询最多60次,每次间隔3秒
    for i in 0..60 {
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
                    .map_err(|e| format!("Cookies validation failed: {}", e))?;

                // 确保UID匹配
                if validated_uid != uid {
                    return Err(format!(
                        "UID mismatch: expected {}, got {}",
                        uid, validated_uid
                    ));
                }

                // 保存到Redis
                let mut cookies_data = CookiesData::new(validated_uid.clone(), cookies);
                cookies_data = cookies_data.with_screen_name(screen_name.clone());

                state
                    .redis
                    .save_cookies(&cookies_data)
                    .await
                    .map_err(|e| format!("Failed to save cookies: {}", e))?;

                tracing::info!(
                    qr_id = %qr_id,
                    uid = %validated_uid,
                    screen_name = %screen_name,
                    "Cookies validated and saved successfully"
                );

                // 返回验证成功事件
                let event =
                    LoginEvent::validation_success(qr_id, validated_uid, screen_name);
                return Ok(PollStatusResponse {
                    event,
                    is_final: true,
                });
            }
            Ok(None) => {
                // 状态未变化或仅扫描
                if session.is_expired() {
                    let event = LoginEvent::qr_expired(qr_id);
                    tracing::warn!(qr_id = %session.qr_id, "QR code expired");
                    return Ok(PollStatusResponse {
                        event,
                        is_final: true,
                    });
                }

                // 状态变化为已扫描 (仅在第一次轮询时返回此事件)
                if session.status == QrCodeStatus::Scanned && i == 0 {
                    let event = LoginEvent::qr_scanned(qr_id.clone());
                    tracing::info!(qr_id = %qr_id, "QR code scanned");
                    return Ok(PollStatusResponse {
                        event,
                        is_final: false,
                    });
                }

                // 用户拒绝
                if session.status == QrCodeStatus::Rejected {
                    let event = LoginEvent::error(qr_id, "User rejected login".to_string());
                    tracing::warn!(qr_id = %session.qr_id, "User rejected login");
                    return Ok(PollStatusResponse {
                        event,
                        is_final: true,
                    });
                }
            }
            Err(e) => {
                tracing::error!(qr_id = %qr_id, error = %e, "Poll failed");
                let event = LoginEvent::error(qr_id, e.to_string());
                return Ok(PollStatusResponse {
                    event,
                    is_final: true,
                });
            }
        }

        // 等待3秒后重试
        sleep(Duration::from_secs(3)).await;
    }

    // 超时 - 3分钟仍未完成
    tracing::warn!(qr_id = %qr_id, "Polling timeout after 3 minutes");
    let event = LoginEvent::error(qr_id, "Polling timeout".to_string());
    Ok(PollStatusResponse {
        event,
        is_final: true,
    })
}
