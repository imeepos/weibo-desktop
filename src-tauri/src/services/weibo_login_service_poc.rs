//! 微博登录服务 - 核心业务逻辑
//!
//! 职责:
//! - 生成二维码并监听登录状态
//! - 监听网络响应 (CDP Network domain)
//! - 提取 QR Code 图片
//! - 提取登录成功后的 Cookies

use crate::models::errors::ApiError;
use crate::services::BrowserService;
use base64::{engine::general_purpose, Engine as _};
use chromiumoxide::cdp::browser_protocol::network::*;
use chromiumoxide::page::Page;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info, warn};

const WEIBO_LOGIN_URL: &str = "https://passport.weibo.com/sso/signin?entry=miniblog&source=miniblog&disp=popup&url=https%3A%2F%2Fweibo.com%2Fnewlogin%3Ftabtype%3Dweibo%26gid%3D102803%26openLoginLayer%3D0%26url%3Dhttps%253A%252F%252Fweibo.com%252F&from=weibopro";

/// 登录会话数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginSession {
    pub session_id: String,
    pub qr_image_base64: String,
    pub expires_at: i64,
}

/// QR Code 扫描状态
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum QrCodeStatus {
    Waiting,   // 等待扫描
    Scanned,   // 已扫描,等待确认
    Confirmed, // 已确认
    Expired,   // 已过期
}

/// 登录状态更新
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusUpdate {
    pub session_id: String,
    pub retcode: i32,
    pub msg: String,
    pub timestamp: i64,
}

/// 登录完成数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginConfirmed {
    pub session_id: String,
    pub uid: String,
    pub cookies: HashMap<String, String>,
    pub screen_name: String,
    pub timestamp: i64,
}

/// 微博登录服务
pub struct WeiboLoginService;

impl WeiboLoginService {
    /// 生成二维码并启动监听
    ///
    /// 返回: (session_id, qr_image_base64, status_receiver)
    pub async fn generate_qrcode() -> Result<LoginSession, ApiError> {
        info!("开始生成微博登录二维码");

        let browser = BrowserService::get_browser().await?;
        let page = browser
            .new_page(WEIBO_LOGIN_URL)
            .await
            .map_err(|e| ApiError::BrowserError(format!("创建页面失败: {}", e)))?;

        // 设置 UserAgent
        page.set_user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36")
            .await
            .map_err(|e| ApiError::BrowserError(format!("设置 UserAgent 失败: {}", e)))?;

        // 等待页面加载
        page.wait_for_navigation()
            .await
            .map_err(|e| ApiError::BrowserError(format!("页面导航失败: {}", e)))?;

        info!("页面加载完成,等待 3 秒");
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

        // 尝试点击"扫描二维码登录"按钮
        let click_result = page.find_element("text=扫描二维码登录").await;

        if let Ok(element) = click_result {
            info!("找到二维码登录按钮,尝试点击");
            let _ = element.click().await;
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        } else {
            debug!("未找到二维码登录按钮,可能已直接显示二维码");
        }

        // 尝试多种选择器查找 QR Code
        let qr_selectors = vec![
            ".login-qrcode img",
            ".qrcode img",
            "img[src*=\"qrcode\"]",
            "[class*=\"qrcode\"] img",
        ];

        let mut qr_image_src: Option<String> = None;

        for selector in qr_selectors {
            debug!("尝试选择器: {}", selector);
            if let Ok(element) = page.find_element(selector).await {
                if let Ok(Some(src)) = element.attribute("src").await {
                    info!(
                        "找到二维码图片: selector={}, src_length={}",
                        selector,
                        src.len()
                    );
                    qr_image_src = Some(src);
                    break;
                }
            }
        }

        let qr_image_src = qr_image_src
            .ok_or_else(|| ApiError::QrCodeGenerationFailed("未找到二维码图片".to_string()))?;

        // 转换为 Base64
        let qr_image_base64 = if qr_image_src.starts_with("data:image") {
            // 已经是 data URL, 提取 base64 部分
            qr_image_src
                .split(',')
                .nth(1)
                .ok_or_else(|| ApiError::QrCodeGenerationFailed("data URL 格式错误".to_string()))?
                .to_string()
        } else {
            // 远程 URL, 需要下载并转换
            let image_url = if qr_image_src.starts_with("http") {
                qr_image_src.clone()
            } else {
                format!("https://passport.weibo.com{}", qr_image_src)
            };

            info!("下载二维码图片: {}", image_url);

            let response = reqwest::get(&image_url)
                .await
                .map_err(|e| ApiError::NetworkFailed(format!("下载二维码失败: {}", e)))?;

            let image_bytes = response
                .bytes()
                .await
                .map_err(|e| ApiError::NetworkFailed(format!("读取二维码数据失败: {}", e)))?;

            general_purpose::STANDARD.encode(&image_bytes)
        };

        let session_id = format!(
            "qr_{}_{}",
            chrono::Utc::now().timestamp_millis(),
            uuid::Uuid::new_v4()
        );
        let expires_at = chrono::Utc::now().timestamp_millis() + 180_000; // 3分钟

        info!("二维码生成成功: session_id={}", session_id);

        Ok(LoginSession {
            session_id,
            qr_image_base64,
            expires_at,
        })
    }

    /// 监听登录状态 (网络请求监听)
    ///
    /// 这是最复杂的部分,需要监听 CDP Network 事件
    pub async fn monitor_login_status<F>(
        page: Page,
        session_id: String,
        mut on_status_update: F,
    ) -> Result<LoginConfirmed, ApiError>
    where
        F: FnMut(StatusUpdate) + Send + 'static,
    {
        info!("开始监听登录状态: session_id={}", session_id);

        // 启用网络追踪
        page.execute(EnableParams::default())
            .await
            .map_err(|e| ApiError::BrowserError(format!("启用网络追踪失败: {}", e)))?;

        // 订阅网络响应事件
        let mut response_events = page
            .event_listener::<EventResponseReceived>()
            .await
            .map_err(|e| ApiError::BrowserError(format!("订阅响应事件失败: {}", e)))?;

        let mut last_retcode: Option<i32> = None;

        // 监听循环
        while let Some(event) = response_events.next().await {
            let url = &event.response.url;

            // 只关心二维码状态检查API
            if !url.contains("/sso/v2/qrcode/check") {
                continue;
            }

            debug!("收到 qrcode/check 响应: status={}", event.response.status);

            // 获取响应体
            let body_result = page
                .execute(GetResponseBodyParams {
                    request_id: event.request_id.clone(),
                })
                .await;

            match body_result {
                Ok(body_data) => {
                    // 解析 JSON
                    match serde_json::from_str::<serde_json::Value>(&body_data.body) {
                        Ok(json) => {
                            let retcode = json["retcode"].as_i64().unwrap_or(-1) as i32;
                            let msg = json["msg"].as_str().unwrap_or("").to_string();

                            // 只有 retcode 变化时才推送
                            if Some(retcode) != last_retcode {
                                debug!("登录状态变化: retcode={}, msg={}", retcode, msg);
                                on_status_update(StatusUpdate {
                                    session_id: session_id.clone(),
                                    retcode,
                                    msg,
                                    timestamp: chrono::Utc::now().timestamp_millis(),
                                });
                                last_retcode = Some(retcode);
                            }
                        }
                        Err(e) => {
                            warn!("解析响应 JSON 失败: {}", e);
                        }
                    }
                }
                Err(e) => {
                    // 登录成功时,页面会跳转,资源可能被销毁
                    let error_msg = format!("{:?}", e);
                    if error_msg.contains("No resource with given identifier found") {
                        info!("检测到登录成功 (资源已销毁),开始提取 cookies");

                        // 提取 cookies
                        let cookies = page.get_cookies().await.map_err(|e| {
                            ApiError::BrowserError(format!("获取 cookies 失败: {}", e))
                        })?;

                        let mut cookie_map = HashMap::new();
                        for cookie in cookies {
                            cookie_map.insert(cookie.name, cookie.value);
                        }

                        // 从 SUB cookie 提取 uid
                        let uid = cookie_map
                            .get("SUB")
                            .and_then(|sub| {
                                // 格式: _2AxxxxUID
                                let re = regex::Regex::new(r"_2A[A-Za-z0-9-_]+").ok()?;
                                re.find(sub).map(|m| m.as_str().to_string())
                            })
                            .unwrap_or_default();

                        info!("登录成功: uid={}, cookies_count={}", uid, cookie_map.len());

                        return Ok(LoginConfirmed {
                            session_id: session_id.clone(),
                            uid,
                            cookies: cookie_map,
                            screen_name: String::new(), // 待后续验证时获取
                            timestamp: chrono::Utc::now().timestamp_millis(),
                        });
                    } else {
                        warn!("获取响应体失败: {}", error_msg);
                    }
                }
            }
        }

        Err(ApiError::PollingFailed("网络监听意外结束".to_string()))
    }
}
