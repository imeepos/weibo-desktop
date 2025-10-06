use serde::Deserialize;
use std::collections::HashMap;
use tokio_tungstenite::{connect_async, tungstenite::Message, WebSocketStream, MaybeTlsStream};
use tokio::net::TcpStream;
use futures_util::{StreamExt, SinkExt};

use crate::models::{ApiError, LoginSession};

/// WebSocket Stream 类型别名
pub type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// 微博登录服务 (WebSocket模式)
///
/// 存在即合理:
/// - 通过WebSocket与长驻的Playwright server通信
/// - 实时推送状态变化,替代低效轮询
/// - 单一职责:连接管理
///
/// 职责:
/// - 建立WebSocket连接
/// - 生成二维码并返回连接流
pub struct WeiboApiClient {
    #[allow(dead_code)]
    server_url: String,
}

/// WebSocket事件
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsEvent {
    QrcodeGenerated {
        session_id: String,
        qr_image: String,
        expires_in: i64,
        expires_at: i64,
        timestamp: i64,
    },
    StatusUpdate {
        session_id: String,
        retcode: i32,
        msg: String,
        data: Option<serde_json::Value>,
        timestamp: i64,
    },
    LoginConfirmed {
        session_id: String,
        status: String,
        cookies: HashMap<String, String>,
        uid: String,
        screen_name: String,
        timestamp: i64,
    },
    Error {
        error_type: String,
        message: String,
        timestamp: i64,
    },
}

impl WeiboApiClient {
    /// 创建新的客户端
    ///
    /// # 参数
    /// - `server_url`: Playwright WebSocket server地址 (如 "ws://localhost:9223")
    pub fn new(server_url: String) -> Self {
        tracing::info!(
            服务器地址 = %server_url,
            "微博API客户端已初始化 (WebSocket模式)"
        );

        Self { server_url }
    }

    /// 生成二维码
    ///
    /// 通过WebSocket调用Playwright server生成二维码
    ///
    /// # 返回值
    /// - `LoginSession`: 登录会话
    /// - `String`: base64编码的二维码图片
    /// - `WsStream`: WebSocket连接流 (供后续监控使用)
    ///
    /// # 错误
    /// - `ApiError::NetworkFailed`: WebSocket连接失败
    /// - `ApiError::QrCodeGenerationFailed`: 二维码生成失败
    /// - `ApiError::JsonParseFailed`: 响应解析失败
    pub async fn generate_qrcode(&self) -> Result<(LoginSession, String, WsStream), ApiError> {
        tracing::info!("通过WebSocket生成二维码");

        let ws_url = "ws://localhost:9223";

        // 连接WebSocket (带重试)
        let (mut ws_stream, _) = Self::connect_with_retry(ws_url, 3).await?;

        tracing::debug!("WebSocket连接成功,发送 generate_qrcode 消息");

        // 发送生成二维码请求
        let request = serde_json::json!({
            "type": "generate_qrcode"
        });

        ws_stream.send(Message::Text(request.to_string())).await.map_err(|e| {
            tracing::error!(错误 = %e, "发送WebSocket消息失败");
            ApiError::NetworkFailed(format!("Failed to send message: {}", e))
        })?;

        // 等待响应 (循环直到收到qrcode_generated或error)
        while let Some(msg_result) = ws_stream.next().await {
            match msg_result {
                Ok(Message::Text(text)) => {
                    tracing::debug!(消息内容 = %text, "收到WebSocket响应");

                    match serde_json::from_str::<WsEvent>(&text) {
                        Ok(WsEvent::QrcodeGenerated { session_id, qr_image, expires_in, expires_at, .. }) => {
                            let session = LoginSession::from_timestamp(session_id, expires_at);

                            tracing::info!(
                                二维码ID = %session.qr_id,
                                实际剩余秒数 = %expires_in,
                                过期时间戳 = %expires_at,
                                "二维码生成成功 (WebSocket),连接保持活跃"
                            );

                            return Ok((session, qr_image, ws_stream));
                        }
                        Ok(WsEvent::Error { error_type, message, .. }) => {
                            tracing::error!(错误类型 = %error_type, 错误信息 = %message, "收到错误消息");
                            return Err(ApiError::QrCodeGenerationFailed(format!("{}: {}", error_type, message)));
                        }
                        Ok(_other) => {
                            tracing::debug!("跳过中间消息,继续等待qrcode_generated");
                            continue;
                        }
                        Err(e) => {
                            tracing::error!(错误 = %e, 原始消息 = %text, "消息解析失败");
                            return Err(ApiError::JsonParseFailed(e.to_string()));
                        }
                    }
                }
                Ok(msg) => {
                    tracing::error!(消息类型 = ?msg, "收到非文本消息");
                    return Err(ApiError::QrCodeGenerationFailed("Received non-text message".to_string()));
                }
                Err(e) => {
                    tracing::error!(错误 = %e, "WebSocket消息接收失败");
                    return Err(ApiError::NetworkFailed(format!("WebSocket error: {}", e)));
                }
            }
        }

        tracing::error!("WebSocket连接意外关闭");
        Err(ApiError::NetworkFailed("WebSocket connection closed unexpectedly".to_string()))
    }

    /// WebSocket连接重试
    ///
    /// # 参数
    /// - `url`: WebSocket地址
    /// - `max_retries`: 最大重试次数
    ///
    /// # 返回值
    /// - `WsStream`: WebSocket连接流
    ///
    /// # 错误
    /// - `ApiError::NetworkFailed`: 重试耗尽后仍然失败
    async fn connect_with_retry(url: &str, max_retries: u32) -> Result<(WsStream, tokio_tungstenite::tungstenite::http::Response<Option<Vec<u8>>>), ApiError> {
        use tokio::time::{sleep, Duration};

        for attempt in 0..max_retries {
            match connect_async(url).await {
                Ok(result) => {
                    if attempt > 0 {
                        tracing::info!(尝试次数 = attempt + 1, "WebSocket重连成功");
                    }
                    return Ok(result);
                }
                Err(e) if attempt < max_retries - 1 => {
                    tracing::warn!(
                        尝试 = attempt + 1,
                        最大重试 = max_retries,
                        错误 = %e,
                        "WebSocket连接失败,重试中"
                    );
                    sleep(Duration::from_secs(2)).await;
                }
                Err(e) => {
                    tracing::error!(错误 = %e, URL = %url, 尝试次数 = max_retries, "WebSocket连接失败 (重试耗尽)");
                    return Err(ApiError::NetworkFailed(format!("WebSocket connection failed after {} retries: {}", max_retries, e)));
                }
            }
        }

        unreachable!()
    }

}
