use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;

use crate::models::{ApiError, LoginSession, QrCodeStatus};

/// 微博API客户端
///
/// 职责:调用微博开放平台API,生成二维码并轮询登录状态。
/// 不涉及数据存储和验证,专注于API交互。
pub struct WeiboApiClient {
    client: Client,
    app_key: String,
}

/// 二维码生成响应
///
/// 微博API返回的二维码数据结构
#[derive(Debug, Deserialize)]
struct QrGenerateResponse {
    /// 二维码唯一标识
    qrcode_key: String,
    /// base64编码的二维码图片
    image: String,
    /// 过期时长(秒)
    expires_in: i64,
}

/// 轮询状态响应
///
/// 微博API返回的登录状态数据
#[derive(Debug, Deserialize)]
struct QrCheckResponse {
    /// 状态: "pending", "scanned", "confirmed", "expired", "rejected"
    status: String,
    /// 登录成功后返回的cookies (仅 confirmed 状态)
    cookies: Option<HashMap<String, String>>,
    /// 用户ID (仅 confirmed 状态)
    uid: Option<String>,
}

impl WeiboApiClient {
    /// 创建新的API客户端
    ///
    /// # 参数
    /// - `app_key`: 微博开放平台应用的App Key
    ///
    /// # 示例
    /// ```
    /// let client = WeiboApiClient::new("your_app_key".to_string());
    /// ```
    pub fn new(app_key: String) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");

        tracing::info!(app_key = %app_key, "Weibo API client initialized");

        Self { client, app_key }
    }

    /// 生成二维码
    ///
    /// 调用微博API生成登录二维码,返回会话和二维码图片。
    ///
    /// # API端点
    /// `POST https://api.weibo.com/oauth2/qrcode/generate`
    ///
    /// # 返回值
    /// - `LoginSession`: 登录会话,追踪状态
    /// - `String`: base64编码的二维码图片
    ///
    /// # 错误
    /// - `ApiError::NetworkFailed`: 网络请求失败
    /// - `ApiError::QrCodeGenerationFailed`: API返回错误
    /// - `ApiError::HttpStatusError`: HTTP状态码非200
    ///
    /// # 注意
    /// 此处使用的API端点是示例,实际使用时需根据微博开放平台文档调整。
    /// 当前微博可能使用不同的认证方式(如扫码登录页面而非API)。
    pub async fn generate_qrcode(&self) -> Result<(LoginSession, String), ApiError> {
        let url = "https://api.weibo.com/oauth2/qrcode/generate";

        tracing::debug!(url = %url, "Requesting QR code generation");

        let response = self
            .client
            .post(url)
            .form(&[("client_id", &self.app_key)])
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            tracing::error!(
                status = %status,
                error_body = %error_body,
                "QR code generation failed"
            );
            return Err(ApiError::HttpStatusError {
                status: status.as_u16(),
                message: error_body,
            });
        }

        let qr_data: QrGenerateResponse = response.json().await.map_err(|e| {
            tracing::error!(error = %e, "Failed to parse QR response");
            ApiError::JsonParseFailed(e.to_string())
        })?;

        let session = LoginSession::new(qr_data.qrcode_key.clone(), qr_data.expires_in);

        tracing::info!(
            qr_id = %session.qr_id,
            expires_in = %qr_data.expires_in,
            "QR code generated successfully"
        );

        Ok((session, qr_data.image))
    }

    /// 轮询二维码状态
    ///
    /// 检查用户是否扫描/确认二维码,自动更新会话状态。
    ///
    /// # API端点
    /// `GET https://api.weibo.com/oauth2/qrcode/check?qrcode_key={qr_id}`
    ///
    /// # 参数
    /// - `session`: 可变的登录会话,状态会被自动更新
    ///
    /// # 返回值
    /// - `Some((uid, cookies))`: 登录成功,返回用户ID和cookies
    /// - `None`: 未完成登录 (pending/scanned/expired/rejected)
    ///
    /// # 错误
    /// - `ApiError::PollingFailed`: 轮询请求失败
    ///
    /// # 状态转换
    /// - `pending` -> 无变化
    /// - `scanned` -> 调用 `session.mark_scanned()`
    /// - `confirmed` -> 调用 `session.mark_confirmed()`, 返回cookies
    /// - `expired` -> 调用 `session.mark_expired()`
    /// - `rejected` -> 调用 `session.mark_rejected()`
    pub async fn check_qrcode_status(
        &self,
        session: &mut LoginSession,
    ) -> Result<Option<(String, HashMap<String, String>)>, ApiError> {
        let url = format!(
            "https://api.weibo.com/oauth2/qrcode/check?qrcode_key={}",
            session.qr_id
        );

        tracing::debug!(
            qr_id = %session.qr_id,
            current_status = ?session.status,
            "Polling QR code status"
        );

        let response = self.client.get(&url).send().await.map_err(|e| {
            tracing::warn!(
                qr_id = %session.qr_id,
                error = %e,
                "Polling request failed"
            );
            ApiError::PollingFailed(e.to_string())
        })?;

        let check_data: QrCheckResponse = response.json().await.map_err(|e| {
            tracing::error!(error = %e, "Failed to parse polling response");
            ApiError::JsonParseFailed(e.to_string())
        })?;

        // 更新session状态
        match check_data.status.as_str() {
            "pending" => {
                // 无变化
                tracing::trace!(qr_id = %session.qr_id, "Status: pending");
            }
            "scanned" => {
                if session.status == QrCodeStatus::Pending {
                    session.mark_scanned();
                    tracing::info!(qr_id = %session.qr_id, "QR code scanned by user");
                }
            }
            "confirmed" => {
                session.mark_confirmed();
                if let (Some(uid), Some(cookies)) = (check_data.uid, check_data.cookies) {
                    tracing::info!(
                        qr_id = %session.qr_id,
                        uid = %uid,
                        cookies_count = %cookies.len(),
                        duration_seconds = %session.duration_seconds(),
                        "Login confirmed, cookies obtained"
                    );
                    return Ok(Some((uid, cookies)));
                } else {
                    tracing::error!(
                        qr_id = %session.qr_id,
                        "Confirmed status but missing uid or cookies"
                    );
                    return Err(ApiError::PollingFailed(
                        "Confirmed status but missing uid or cookies".into(),
                    ));
                }
            }
            "expired" => {
                session.mark_expired();
                tracing::warn!(
                    qr_id = %session.qr_id,
                    duration_seconds = %session.duration_seconds(),
                    "QR code expired"
                );
            }
            "rejected" => {
                session.mark_rejected();
                tracing::warn!(
                    qr_id = %session.qr_id,
                    duration_seconds = %session.duration_seconds(),
                    "User rejected login"
                );
            }
            unknown => {
                tracing::warn!(
                    qr_id = %session.qr_id,
                    status = %unknown,
                    "Unknown status received from API"
                );
            }
        }

        Ok(None)
    }

    /// 轮询直到终态
    ///
    /// 持续轮询,直到登录成功/过期/拒绝。
    /// 适用于命令行工具或测试场景。
    ///
    /// # 参数
    /// - `session`: 登录会话
    /// - `poll_interval_ms`: 轮询间隔(毫秒),建议2000-3000
    ///
    /// # 返回值
    /// - `Some((uid, cookies))`: 登录成功
    /// - `None`: 过期或拒绝
    ///
    /// # 错误
    /// 返回 `ApiError` 如果轮询过程中发生错误
    ///
    /// # 示例
    /// ```
    /// let (session, _) = client.generate_qrcode().await?;
    /// if let Some((uid, cookies)) = client.poll_until_final(&mut session, 2000).await? {
    ///     println!("登录成功: {}", uid);
    /// }
    /// ```
    pub async fn poll_until_final(
        &self,
        session: &mut LoginSession,
        poll_interval_ms: u64,
    ) -> Result<Option<(String, HashMap<String, String>)>, ApiError> {
        let poll_interval = std::time::Duration::from_millis(poll_interval_ms);

        loop {
            if session.is_final_status() {
                break;
            }

            if session.is_expired() {
                session.mark_expired();
                break;
            }

            if let Some(result) = self.check_qrcode_status(session).await? {
                return Ok(Some(result));
            }

            tokio::time::sleep(poll_interval).await;
        }

        // 过期或拒绝
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // 需要真实的微博API访问
    async fn test_generate_qrcode() {
        let client = WeiboApiClient::new("test_app_key".to_string());
        let result = client.generate_qrcode().await;
        // 实际测试中会失败(无效的app_key),仅用于演示结构
        assert!(result.is_err());
    }

    #[test]
    fn test_client_creation() {
        let client = WeiboApiClient::new("test_app_key".to_string());
        assert_eq!(client.app_key, "test_app_key");
    }
}
