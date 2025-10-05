use serde::Deserialize;
use std::collections::HashMap;
use tokio::process::Command;

use crate::models::{ApiError, LoginSession, QrCodeStatus};

/// 微博登录服务 (Playwright实现)
///
/// 存在即合理:
/// - 移除对微博OAuth2 API和App Key的依赖
/// - 使用Playwright自动化真实的登录流程
/// - 每个方法都有不可替代的职责
///
/// 职责:
/// - 调用Playwright脚本生成二维码
/// - 轮询检测登录状态
/// - 提取cookies和用户信息
pub struct WeiboApiClient {
    playwright_login_script: String,
}

/// Playwright生成二维码响应
#[derive(Debug, Deserialize)]
struct PlaywrightGenerateResponse {
    session_id: String,
    qr_image: String,
    expires_in: i64,
}

/// Playwright状态检查响应
#[derive(Debug, Deserialize)]
struct PlaywrightStatusResponse {
    status: String,
    cookies: Option<HashMap<String, String>>,
    uid: Option<String>,
    screen_name: Option<String>,
}

/// Playwright错误响应
#[derive(Debug, Deserialize)]
struct PlaywrightErrorResponse {
    error: String,
}

impl WeiboApiClient {
    /// 创建新的客户端
    ///
    /// # 参数
    /// - `playwright_login_script`: Playwright登录脚本路径
    pub fn new(playwright_login_script: String) -> Self {
        tracing::info!(
            playwright_script = %playwright_login_script,
            "Weibo API client initialized (Playwright mode)"
        );

        Self {
            playwright_login_script,
        }
    }

    /// 生成二维码
    ///
    /// 调用Playwright脚本访问真实的微博登录页面,提取二维码图片。
    ///
    /// # 返回值
    /// - `LoginSession`: 登录会话
    /// - `String`: base64编码的二维码图片
    ///
    /// # 错误
    /// - `ApiError::NetworkFailed`: Playwright执行失败
    /// - `ApiError::QrCodeGenerationFailed`: 二维码生成失败
    /// - `ApiError::JsonParseFailed`: 响应解析失败
    pub async fn generate_qrcode(&self) -> Result<(LoginSession, String), ApiError> {
        tracing::debug!(
            script = %self.playwright_login_script,
            "Calling Playwright to generate QR code"
        );

        let output = Command::new("node")
            .arg(&self.playwright_login_script)
            .arg("generate")
            .output()
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to execute Playwright script");
                ApiError::NetworkFailed(format!("Playwright execution failed: {}", e))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);

            // 尝试解析错误信息
            if let Ok(err_response) = serde_json::from_str::<PlaywrightErrorResponse>(&stdout) {
                tracing::error!(error = %err_response.error, "Playwright returned error");
                return Err(ApiError::QrCodeGenerationFailed(err_response.error));
            }

            tracing::error!(
                stderr = %stderr,
                stdout = %stdout,
                "Playwright script execution failed"
            );
            return Err(ApiError::QrCodeGenerationFailed(format!(
                "Script failed: {}",
                stderr
            )));
        }

        let response: PlaywrightGenerateResponse =
            serde_json::from_slice(&output.stdout).map_err(|e| {
                let stdout = String::from_utf8_lossy(&output.stdout);
                tracing::error!(
                    error = %e,
                    stdout = %stdout,
                    "Failed to parse Playwright response"
                );
                ApiError::JsonParseFailed(e.to_string())
            })?;

        let session = LoginSession::new(response.session_id, response.expires_in);

        tracing::info!(
            qr_id = %session.qr_id,
            expires_in = %response.expires_in,
            "QR code generated successfully (Playwright)"
        );

        Ok((session, response.qr_image))
    }

    /// 检查二维码状态
    ///
    /// 调用Playwright脚本检测登录状态,自动更新会话状态。
    ///
    /// # 参数
    /// - `session`: 可变的登录会话
    ///
    /// # 返回值
    /// - `Some((uid, cookies))`: 登录成功
    /// - `None`: 未完成登录
    ///
    /// # 错误
    /// - `ApiError::PollingFailed`: 轮询失败
    /// - `ApiError::JsonParseFailed`: 响应解析失败
    ///
    /// # 状态转换
    /// - pending: 无变化
    /// - scanned: 调用 `session.mark_scanned()`
    /// - confirmed: 调用 `session.mark_confirmed()`, 返回cookies
    /// - expired: 调用 `session.mark_expired()`
    pub async fn check_qrcode_status(
        &self,
        session: &mut LoginSession,
    ) -> Result<Option<(String, HashMap<String, String>)>, ApiError> {
        tracing::debug!(
            qr_id = %session.qr_id,
            current_status = ?session.status,
            "Checking QR code status (Playwright)"
        );

        let output = Command::new("node")
            .arg(&self.playwright_login_script)
            .arg("check")
            .arg(&session.qr_id)
            .output()
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to execute Playwright check");
                ApiError::PollingFailed(format!("Playwright execution failed: {}", e))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);

            if let Ok(err_response) = serde_json::from_str::<PlaywrightErrorResponse>(&stdout) {
                tracing::error!(error = %err_response.error, "Playwright check error");
                return Err(ApiError::PollingFailed(err_response.error));
            }

            tracing::error!(
                stderr = %stderr,
                stdout = %stdout,
                "Playwright check failed"
            );
            return Err(ApiError::PollingFailed(format!("Check failed: {}", stderr)));
        }

        let response: PlaywrightStatusResponse =
            serde_json::from_slice(&output.stdout).map_err(|e| {
                let stdout = String::from_utf8_lossy(&output.stdout);
                tracing::error!(
                    error = %e,
                    stdout = %stdout,
                    "Failed to parse status response"
                );
                ApiError::JsonParseFailed(e.to_string())
            })?;

        match response.status.as_str() {
            "pending" => {
                tracing::trace!(qr_id = %session.qr_id, "Status: pending");
            }
            "scanned" => {
                if session.status == QrCodeStatus::Pending {
                    session.mark_scanned();
                    tracing::info!(qr_id = %session.qr_id, "QR code scanned (Playwright)");
                }
            }
            "confirmed" => {
                session.mark_confirmed();
                if let (Some(uid), Some(cookies)) = (response.uid, response.cookies) {
                    tracing::info!(
                        qr_id = %session.qr_id,
                        uid = %uid,
                        cookies_count = %cookies.len(),
                        screen_name = ?response.screen_name,
                        duration_seconds = %session.duration_seconds(),
                        "Login confirmed (Playwright)"
                    );
                    return Ok(Some((uid, cookies)));
                } else {
                    tracing::error!(
                        qr_id = %session.qr_id,
                        "Confirmed status but missing uid or cookies"
                    );
                    return Err(ApiError::PollingFailed(
                        "Missing uid or cookies in confirmed response".into(),
                    ));
                }
            }
            "expired" => {
                session.mark_expired();
                tracing::warn!(
                    qr_id = %session.qr_id,
                    duration_seconds = %session.duration_seconds(),
                    "QR code expired (Playwright)"
                );
            }
            unknown => {
                tracing::warn!(
                    qr_id = %session.qr_id,
                    status = %unknown,
                    "Unknown status from Playwright"
                );
            }
        }

        Ok(None)
    }

    /// 轮询直到终态
    ///
    /// 持续轮询直到登录成功/过期。
    /// 适用于测试场景。
    ///
    /// # 参数
    /// - `session`: 登录会话
    /// - `poll_interval_ms`: 轮询间隔(毫秒)
    ///
    /// # 返回值
    /// - `Some((uid, cookies))`: 登录成功
    /// - `None`: 过期或拒绝
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

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = WeiboApiClient::new("./test/script.js".to_string());
        assert_eq!(client.playwright_login_script, "./test/script.js");
    }
}
