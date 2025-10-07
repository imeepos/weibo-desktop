use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use std::collections::HashMap;
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};

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
    Pong {
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

        tracing::debug!("WebSocket连接成功,执行健康检查");

        // 健康检查: 发送ping并等待pong响应
        Self::verify_connection_health(&mut ws_stream).await?;

        tracing::debug!("健康检查通过,发送 generate_qrcode 消息");

        // 发送生成二维码请求
        let request = serde_json::json!({
            "type": "generate_qrcode"
        });

        ws_stream
            .send(Message::Text(request.to_string()))
            .await
            .map_err(|e| {
                tracing::error!(错误 = %e, "发送WebSocket消息失败");
                ApiError::NetworkFailed(format!("Failed to send message: {}", e))
            })?;

        // 等待响应 (循环直到收到qrcode_generated或error)
        while let Some(msg_result) = ws_stream.next().await {
            match msg_result {
                Ok(Message::Text(text)) => {
                    tracing::debug!(消息内容 = %text, "收到WebSocket响应");

                    match serde_json::from_str::<WsEvent>(&text) {
                        Ok(WsEvent::QrcodeGenerated {
                            session_id,
                            qr_image,
                            expires_in,
                            expires_at,
                            ..
                        }) => {
                            let session = LoginSession::from_timestamp(session_id, expires_at);

                            tracing::info!(
                                二维码ID = %session.qr_id,
                                实际剩余秒数 = %expires_in,
                                过期时间戳 = %expires_at,
                                "二维码生成成功 (WebSocket),连接保持活跃"
                            );

                            return Ok((session, qr_image, ws_stream));
                        }
                        Ok(WsEvent::Error {
                            error_type,
                            message,
                            ..
                        }) => {
                            tracing::error!(错误类型 = %error_type, 错误信息 = %message, "收到错误消息");
                            return Err(ApiError::QrCodeGenerationFailed(format!(
                                "{}: {}",
                                error_type, message
                            )));
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
                    return Err(ApiError::QrCodeGenerationFailed(
                        "Received non-text message".to_string(),
                    ));
                }
                Err(e) => {
                    tracing::error!(错误 = %e, "WebSocket消息接收失败");
                    return Err(ApiError::NetworkFailed(format!("WebSocket error: {}", e)));
                }
            }
        }

        tracing::error!("WebSocket连接意外关闭");
        Err(ApiError::NetworkFailed(
            "WebSocket connection closed unexpectedly".to_string(),
        ))
    }

    /// 验证WebSocket连接健康状态
    ///
    /// 发送ping消息并等待pong响应以确认服务器正常运行
    ///
    /// # 参数
    /// - `ws_stream`: WebSocket连接流
    ///
    /// # 错误
    /// - `ApiError::PlaywrightServerNotRunning`: 服务器未响应健康检查
    /// - `ApiError::NetworkFailed`: 网络通信失败
    async fn verify_connection_health(ws_stream: &mut WsStream) -> Result<(), ApiError> {
        use tokio::time::{timeout, Duration};

        tracing::debug!("发送ping消息验证服务器健康状态");

        // 发送ping请求
        let ping_request = serde_json::json!({
            "type": "ping"
        });

        ws_stream
            .send(Message::Text(ping_request.to_string()))
            .await
            .map_err(|e| {
                tracing::error!(错误 = %e, "发送ping消息失败");
                ApiError::NetworkFailed(format!("Failed to send ping: {}", e))
            })?;

        // 等待pong响应 (超时3秒)
        let pong_result = timeout(Duration::from_secs(3), async {
            while let Some(msg_result) = ws_stream.next().await {
                match msg_result {
                    Ok(Message::Text(text)) => match serde_json::from_str::<WsEvent>(&text) {
                        Ok(WsEvent::Pong { timestamp }) => {
                            tracing::info!(服务器时间戳 = timestamp, "收到pong响应,服务器健康");
                            return Ok(());
                        }
                        Ok(_other) => {
                            tracing::debug!("跳过非pong消息,继续等待");
                            continue;
                        }
                        Err(e) => {
                            tracing::warn!(错误 = %e, 消息 = %text, "消息解析失败,继续等待");
                            continue;
                        }
                    },
                    Ok(_msg) => {
                        tracing::debug!("跳过非文本消息");
                        continue;
                    }
                    Err(e) => {
                        tracing::error!(错误 = %e, "接收消息失败");
                        return Err(ApiError::NetworkFailed(format!(
                            "Failed to receive pong: {}",
                            e
                        )));
                    }
                }
            }
            Err(ApiError::NetworkFailed(
                "WebSocket连接在等待pong时关闭".to_string(),
            ))
        })
        .await;

        match pong_result {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(_) => {
                tracing::error!("健康检查超时,服务器未响应pong");
                Err(ApiError::PlaywrightServerNotRunning)
            }
        }
    }

    /// 诊断Playwright服务器状态
    ///
    /// 检测:
    /// 1. 端口9223是否被监听
    /// 2. 能否建立TCP连接
    /// 3. 能否建立WebSocket连接
    ///
    /// # 返回值
    /// - `String`: 详细的诊断信息
    pub async fn diagnose_server_status(&self) -> String {
        use tokio::time::{timeout, Duration};

        let mut diagnosis = String::from("=== Playwright服务器状态诊断 ===\n");

        tracing::debug!("开始诊断Playwright服务器状态");

        // 1. TCP连接测试
        diagnosis.push_str("\n[1] TCP连接测试 (127.0.0.1:9223):\n");
        match timeout(Duration::from_secs(2), TcpStream::connect("127.0.0.1:9223")).await {
            Ok(Ok(_stream)) => {
                diagnosis.push_str("  ✓ TCP连接成功 - 端口正在监听\n");
                tracing::debug!("TCP连接测试通过");
            }
            Ok(Err(e)) => {
                diagnosis.push_str(&format!("  ✗ TCP连接失败: {}\n", e));
                diagnosis.push_str("  → 端口9223未被监听\n");
                tracing::debug!(错误 = %e, "TCP连接测试失败");
            }
            Err(_) => {
                diagnosis.push_str("  ✗ TCP连接超时 (2秒)\n");
                tracing::debug!("TCP连接测试超时");
            }
        }

        // 2. WebSocket连接测试
        diagnosis.push_str("\n[2] WebSocket连接测试 (ws://localhost:9223):\n");
        match timeout(Duration::from_secs(3), connect_async("ws://localhost:9223")).await {
            Ok(Ok(_)) => {
                diagnosis.push_str("  ✓ WebSocket连接成功\n");
                tracing::debug!("WebSocket连接测试通过");
            }
            Ok(Err(e)) => {
                diagnosis.push_str(&format!("  ✗ WebSocket连接失败: {}\n", e));
                tracing::debug!(错误 = %e, "WebSocket连接测试失败");
            }
            Err(_) => {
                diagnosis.push_str("  ✗ WebSocket连接超时 (3秒)\n");
                tracing::debug!("WebSocket连接测试超时");
            }
        }

        // 3. 解决方案建议
        diagnosis.push_str("\n[解决方案]:\n");
        diagnosis.push_str("  请在项目根目录运行以下命令启动Playwright服务器:\n");
        diagnosis.push_str("  $ pnpm --filter playwright dev\n");
        diagnosis.push_str("\n  或使用Docker Compose:\n");
        diagnosis.push_str("  $ docker compose up playwright\n");

        tracing::debug!("诊断完成");
        diagnosis
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
    /// - `ApiError::PlaywrightServerNotRunning`: Playwright服务器未启动
    /// - `ApiError::NetworkFailed`: 其他网络错误
    async fn connect_with_retry(
        url: &str,
        max_retries: u32,
    ) -> Result<
        (
            WsStream,
            tokio_tungstenite::tungstenite::http::Response<Option<Vec<u8>>>,
        ),
        ApiError,
    > {
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
                    let error_msg = e.to_string().to_lowercase();

                    if error_msg.contains("connection refused")
                        || error_msg.contains("connect error")
                    {
                        tracing::error!(
                            错误 = %e,
                            URL = %url,
                            "Playwright WebSocket服务器未运行"
                        );

                        // 执行服务器状态诊断
                        let client = WeiboApiClient::new(url.to_string());
                        let diagnosis = client.diagnose_server_status().await;
                        tracing::error!(诊断结果 = %diagnosis, "服务器状态诊断");

                        return Err(ApiError::PlaywrightServerNotRunning);
                    }

                    tracing::error!(错误 = %e, URL = %url, 尝试次数 = max_retries, "WebSocket连接失败");
                    return Err(ApiError::NetworkFailed(format!("WebSocket连接失败: {}", e)));
                }
            }
        }

        unreachable!()
    }

    /// 获取Cookies
    pub async fn get_cookies(&self) -> Result<HashMap<String, String>, ApiError> {
        // 暂时返回空的cookies map，后续可以从Redis或数据库中获取
        Ok(HashMap::new())
    }

    /// 爬取帖子
    pub async fn crawl_posts(&self, _request: &crate::services::crawl_service_simple::SimpleCrawlRequest) -> Result<Vec<crate::services::crawl_service_simple::WeiboPostRaw>, ApiError> {
        // 暂时返回空结果，后续会调用Playwright服务器进行实际爬取
        Ok(vec![])
    }
}
