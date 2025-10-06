use crate::models::{ApiError, QrCodeStatus, CookiesData, parse_qr_status};
use crate::models::events::{LoginErrorEvent, LoginStatusEvent};
use crate::state::AppState;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};
use futures_util::StreamExt;

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

    /// 有效期秒数 (60秒超时)
    pub expires_in: u64,
}

/// 生成二维码并启动监控
///
/// 一次调用完成:
/// 1. 生成二维码
/// 2. 返回二维码数据
/// 3. 后台监控登录状态并推送Event到前端
///
/// 返回:
/// - QrCodeResponse: 二维码图片和会话信息
///
/// 副作用:
/// - 启动后台WebSocket监控任务
/// - 状态变化通过Tauri Event推送: login_status_update, login_error
#[tauri::command]
pub async fn generate_qrcode(
    app: AppHandle,
    state: State<'_, AppState>
) -> Result<QrCodeResponse, ApiError> {
    tracing::info!("生成二维码并启动监控");

    // 调用微博API生成二维码 (返回WebSocket连接)
    let (session, qr_image, ws_stream) = state.weibo_api.generate_qrcode().await?;

    let expires_in = session.remaining_seconds().max(0) as u64;
    let qr_id = session.qr_id.clone();

    tracing::info!(
        二维码ID = %qr_id,
        有效期秒数 = %expires_in,
        "二维码生成成功,启动后台监控"
    );

    // 克隆services用于后台任务 (Arc已在内部,无需重复包装)
    let redis = state.redis.clone();
    let validator = state.validator.clone();
    let session_manager = state.session_manager.clone();

    // 克隆qr_id用于后续操作
    let qr_id_for_task = qr_id.clone();
    let qr_id_for_manager = qr_id.clone();

    // 启动后台监控任务 (可取消)
    let monitor_task = tokio::spawn(async move {
        monitor_login(qr_id_for_task, ws_stream, app, redis, validator).await;
    });

    // 注册到会话管理器 (自动取消旧任务)
    let abort_handle = monitor_task.abort_handle();
    session_manager.set_current_session(qr_id_for_manager, abort_handle).await;

    Ok(QrCodeResponse {
        qr_id: session.qr_id,
        qr_image,
        expires_at: session.expires_at,
        expires_in,
    })
}

/// 监控登录状态 (后台任务)
///
/// 监听WebSocket消息流,处理状态变化并推送Event到前端
/// 支持任务取消 - 当SessionManager取消旧会话时,此任务会自动终止
async fn monitor_login(
    qr_id: String,
    ws_stream: crate::services::weibo_api::WsStream,
    app: AppHandle,
    redis: Arc<crate::services::RedisService>,
    validator: Arc<crate::services::ValidationService>,
) {
    use crate::services::weibo_api::WsEvent;
    use tokio_tungstenite::tungstenite::Message;

    tracing::info!(二维码ID = %qr_id, "登录监控已启动");

    // 监控任务被取消时的清理逻辑
    let cleanup_guard = CleanupGuard::new(qr_id.clone());

    let stream = ws_stream.filter_map(|msg_result| async move {
        match msg_result {
            Ok(Message::Text(text)) => {
                match serde_json::from_str::<WsEvent>(&text) {
                    Ok(WsEvent::QrcodeGenerated { .. }) => None,
                    Ok(WsEvent::StatusUpdate { retcode, msg, data, .. }) => {
                        Some(Ok((parse_qr_status(retcode), None, None, None, Some(retcode), Some(msg), data)))
                    }
                    Ok(WsEvent::LoginConfirmed { cookies, uid, screen_name, .. }) => {
                        Some(Ok((QrCodeStatus::Confirmed, Some(uid), Some(cookies), Some(screen_name), None, None, None)))
                    }
                    Ok(WsEvent::Error { error_type, message, .. }) => {
                        Some(Err(ApiError::QrCodeGenerationFailed(format!("{}: {}", error_type, message))))
                    }
                    Err(e) => Some(Err(ApiError::JsonParseFailed(e.to_string())))
                }
            }
            Ok(Message::Close(_)) => None,
            Err(e) => Some(Err(ApiError::NetworkFailed(format!("WebSocket error: {}", e)))),
            _ => None,
        }
    });

    tokio::pin!(stream);

    tracing::debug!(二维码ID = %qr_id, "WebSocket消息流已就绪,开始等待消息");

    while let Some(result) = stream.next().await {
        tracing::debug!(二维码ID = %qr_id, "收到WebSocket消息: {:?}", result);

        match result {
            Ok((status, uid_opt, cookies_opt, screen_name_opt, retcode, msg, data)) => {
                tracing::info!(二维码ID = %qr_id, 状态 = ?status, retcode = ?retcode, msg = ?msg, "状态更新");

                match status {
                    QrCodeStatus::Confirmed => {
                        tracing::debug!(二维码ID = %qr_id, "处理Confirmed状态");
                        if let (Some(uid), Some(cookies), Some(screen_name)) = (uid_opt, cookies_opt, screen_name_opt) {
                            // 验证cookies
                            match validator.validate_cookies(&cookies).await {
                                Ok((validated_uid, _validated_screen_name)) => {
                                    if validated_uid != uid {
                                        emit_error(&app, &qr_id, "ValidationError", format!("UID不匹配: 期望 {}, 实际 {}", uid, validated_uid));
                                        break;
                                    }

                                    // 构建并保存CookiesData
                                    let cookies_data = CookiesData::new(validated_uid.clone(), cookies)
                                        .with_screen_name(screen_name);

                                    if let Err(e) = redis.save_cookies(&cookies_data).await {
                                        tracing::error!(二维码ID = %qr_id, 错误 = ?e, "保存cookies失败");
                                        emit_error(&app, &qr_id, "StorageError", format!("保存Cookies失败: {}", e));
                                        break;
                                    }

                                    tracing::info!(二维码ID = %qr_id, uid = %validated_uid, "Cookies已保存");

                                    // 推送confirmed事件
                                    let event = LoginStatusEvent::new(qr_id.clone(), QrCodeStatus::Confirmed, Some(cookies_data));
                                    let _ = app.emit_all("login_status_update", event);
                                    tracing::debug!(二维码ID = %qr_id, "Confirmed事件已发送至前端");
                                }
                                Err(e) => {
                                    tracing::error!(二维码ID = %qr_id, 错误 = ?e, "Cookies验证失败");
                                    emit_error(&app, &qr_id, "ValidationError", format!("Cookies验证失败: {}", e));
                                }
                            }
                        }
                        break;
                    }
                    QrCodeStatus::Scanned => {
                        tracing::debug!(二维码ID = %qr_id, "处理Scanned状态");
                        let event = LoginStatusEvent::with_raw_data(qr_id.clone(), QrCodeStatus::Scanned, None, retcode, msg, data);
                        let _ = app.emit_all("login_status_update", event);
                        tracing::debug!(二维码ID = %qr_id, "Scanned事件已发送至前端");
                    }
                    QrCodeStatus::Rejected | QrCodeStatus::Expired => {
                        tracing::debug!(二维码ID = %qr_id, 状态 = ?status, "处理终止状态");
                        let event = LoginStatusEvent::with_raw_data(qr_id.clone(), status, None, retcode, msg, data);
                        let _ = app.emit_all("login_status_update", event);
                        tracing::debug!(二维码ID = %qr_id, 状态 = ?status, "终止状态事件已发送至前端");
                        break;
                    }
                    _ => {
                        tracing::debug!(二维码ID = %qr_id, 状态 = ?status, "处理其他状态");
                        let event = LoginStatusEvent::with_raw_data(qr_id.clone(), status, None, retcode, msg, data);
                        let _ = app.emit_all("login_status_update", event);
                        tracing::debug!(二维码ID = %qr_id, 状态 = ?status, "状态事件已发送至前端");
                    }
                }
            }
            Err(e) => {
                tracing::error!(二维码ID = %qr_id, 错误 = ?e, 流状态 = "active", "WebSocket错误");
                emit_error(&app, &qr_id, "WebSocketError", format!("{:?}", e));
                break;
            }
        }
    }

    tracing::debug!(二维码ID = %qr_id, "WebSocket消息流已关闭,退出监控循环");

    drop(cleanup_guard); // 显式清理
    tracing::info!(二维码ID = %qr_id, "登录监控已停止");
}

/// 清理守卫: 任务结束或被取消时自动记录日志
struct CleanupGuard {
    qr_id: String,
}

impl CleanupGuard {
    fn new(qr_id: String) -> Self {
        Self { qr_id }
    }
}

impl Drop for CleanupGuard {
    fn drop(&mut self) {
        tracing::debug!(二维码ID = %self.qr_id, "监控任务清理完成 (WebSocket连接已关闭)");
    }
}

fn emit_error(app: &AppHandle, qr_id: &str, error_type: &str, message: String) {
    let error_event = LoginErrorEvent::new(qr_id.to_string(), error_type.to_string(), message);
    let _ = app.emit_all("login_error", error_event);
}
