//! WebSocket 服务器 - 实时通信
//!
//! 职责:
//! - 接收前端(或其他客户端)的连接
//! - 处理二维码生成请求
//! - 实时推送登录状态更新
//!
//! 消息协议 (兼容 TypeScript 版本):
//! Client -> Server: { type: 'generate_qrcode' } | { type: 'ping' }
//! Server -> Client: { type: 'qrcode_generated' | 'status_update' | 'login_confirmed' | 'error' | 'pong' }

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use tracing::{debug, error, info, warn};

use crate::services::WeiboLoginService;

const WS_PORT: u16 = 9223;

/// 客户端请求消息
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum ClientMessage {
    #[serde(rename = "generate_qrcode")]
    GenerateQrcode,
    #[serde(rename = "ping")]
    Ping,
}

/// 服务器响应消息
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum ServerMessage {
    #[serde(rename = "qrcode_generated")]
    QrcodeGenerated {
        session_id: String,
        qr_image: String,
        expires_in: i64,
        expires_at: i64,
        timestamp: i64,
    },
    #[serde(rename = "status_update")]
    StatusUpdate {
        session_id: String,
        retcode: i32,
        msg: String,
        timestamp: i64,
    },
    #[serde(rename = "login_confirmed")]
    LoginConfirmed {
        session_id: String,
        status: String,
        cookies: std::collections::HashMap<String, String>,
        uid: String,
        screen_name: String,
        timestamp: i64,
    },
    #[serde(rename = "error")]
    Error {
        error_type: String,
        message: String,
        timestamp: i64,
    },
    #[serde(rename = "pong")]
    Pong { timestamp: i64 },
}

/// WebSocket 服务器
pub struct WebSocketServer;

impl WebSocketServer {
    /// 启动 WebSocket 服务器
    pub async fn start() -> Result<(), Box<dyn std::error::Error>> {
        let addr = format!("127.0.0.1:{}", WS_PORT);
        let listener = TcpListener::bind(&addr).await?;
        info!("WebSocket 服务器已启动: {}", addr);

        while let Ok((stream, peer_addr)) = listener.accept().await {
            tokio::spawn(async move {
                if let Err(e) = handle_connection(stream, peer_addr).await {
                    error!("处理连接失败 {}: {}", peer_addr, e);
                }
            });
        }

        Ok(())
    }
}

/// 处理单个 WebSocket 连接
async fn handle_connection(
    stream: TcpStream,
    peer_addr: SocketAddr,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("新连接: {}", peer_addr);

    let ws_stream = accept_async(stream).await?;
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    while let Some(msg_result) = ws_receiver.next().await {
        match msg_result {
            Ok(Message::Text(text)) => {
                debug!("收到消息: {}", text);

                // 解析客户端消息
                match serde_json::from_str::<ClientMessage>(&text) {
                    Ok(ClientMessage::GenerateQrcode) => {
                        info!("处理生成二维码请求");

                        // 生成二维码
                        match WeiboLoginService::generate_qrcode().await {
                            Ok(session) => {
                                let now = chrono::Utc::now().timestamp_millis();
                                let expires_in = (session.expires_at - now) / 1000;

                                // 发送二维码
                                let response = ServerMessage::QrcodeGenerated {
                                    session_id: session.session_id.clone(),
                                    qr_image: session.qr_image_base64,
                                    expires_in,
                                    expires_at: session.expires_at,
                                    timestamp: now,
                                };

                                let response_json = serde_json::to_string(&response)?;
                                ws_sender.send(Message::Text(response_json)).await?;

                                // TODO: 启动监听登录状态
                                // 这需要持有 page 的引用,暂时先简化
                                info!("二维码已发送,session_id={}", session.session_id);
                            }
                            Err(e) => {
                                error!("生成二维码失败: {}", e);
                                let error_response = ServerMessage::Error {
                                    error_type: "QrcodeGenerationFailed".to_string(),
                                    message: e.to_string(),
                                    timestamp: chrono::Utc::now().timestamp_millis(),
                                };
                                let error_json = serde_json::to_string(&error_response)?;
                                ws_sender.send(Message::Text(error_json)).await?;
                            }
                        }
                    }
                    Ok(ClientMessage::Ping) => {
                        debug!("收到 ping");
                        let pong = ServerMessage::Pong {
                            timestamp: chrono::Utc::now().timestamp_millis(),
                        };
                        let pong_json = serde_json::to_string(&pong)?;
                        ws_sender.send(Message::Text(pong_json)).await?;
                    }
                    Err(e) => {
                        warn!("解析消息失败: {}", e);
                        let error_response = ServerMessage::Error {
                            error_type: "InvalidMessage".to_string(),
                            message: "Failed to parse message".to_string(),
                            timestamp: chrono::Utc::now().timestamp_millis(),
                        };
                        let error_json = serde_json::to_string(&error_response)?;
                        ws_sender.send(Message::Text(error_json)).await?;
                    }
                }
            }
            Ok(Message::Close(_)) => {
                info!("客户端关闭连接: {}", peer_addr);
                break;
            }
            Ok(_) => {
                // 忽略其他消息类型 (Binary, Ping, Pong)
            }
            Err(e) => {
                error!("接收消息失败: {}", e);
                break;
            }
        }
    }

    info!("连接关闭: {}", peer_addr);
    Ok(())
}
