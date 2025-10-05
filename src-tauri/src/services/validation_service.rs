use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::process::Command;

use crate::models::ValidationError;

/// Cookies验证服务
///
/// 职责:调用Playwright脚本验证cookies有效性。
/// 通过访问微博个人资料API,确认cookies未过期且可用。
pub struct ValidationService {
    playwright_script_path: String,
}

/// Playwright验证结果
///
/// Node.js脚本返回的JSON结构
#[derive(Debug, Deserialize)]
struct PlaywrightValidationResult {
    /// 验证是否成功
    valid: bool,
    /// 用户ID (验证成功时返回)
    uid: Option<String>,
    /// 用户昵称 (验证成功时返回)
    screen_name: Option<String>,
    /// 错误信息 (验证失败时返回)
    error: Option<String>,
}

/// Playwright脚本输入
///
/// 传递给Node.js脚本的JSON结构
#[derive(Debug, Serialize)]
struct PlaywrightInput {
    /// Cookie键值对
    cookies: HashMap<String, String>,
}

impl ValidationService {
    /// 创建新的验证服务
    ///
    /// # 参数
    /// - `playwright_script_path`: Playwright验证脚本的绝对路径
    ///
    /// # 示例
    /// ```
    /// use weibo_login::services::ValidationService;
    /// let service = ValidationService::new(
    ///     "/workspace/desktop/playwright/validate-cookies.js".to_string()
    /// );
    /// ```
    pub fn new(playwright_script_path: String) -> Self {
        tracing::info!(
            script_path = %playwright_script_path,
            "Validation service initialized"
        );
        Self {
            playwright_script_path,
        }
    }

    /// 验证Cookies有效性
    ///
    /// 调用Playwright脚本,使用cookies访问微博个人资料API。
    /// 如果返回成功,说明cookies有效。
    ///
    /// # 参数
    /// - `cookies`: 待验证的cookie键值对
    ///
    /// # 返回值
    /// - `Ok((uid, screen_name))`: 验证成功,返回用户ID和昵称
    ///
    /// # 错误
    /// - `ValidationError::PlaywrightFailed`: Playwright脚本执行失败
    /// - `ValidationError::ProfileApiFailed`: 个人资料API返回错误(cookies无效)
    ///
    /// # Playwright脚本约定
    /// - 输入: JSON字符串作为第一个参数,格式 `{"cookies": {...}}`
    /// - 输出: JSON到stdout,格式 `{"valid": bool, "uid": string, "screen_name": string, "error": string}`
    /// - 退出码: 0表示脚本执行成功(但valid可能为false),非0表示脚本崩溃
    pub async fn validate_cookies(
        &self,
        cookies: &HashMap<String, String>,
    ) -> Result<(String, String), ValidationError> {
        // 序列化输入
        let input = PlaywrightInput {
            cookies: cookies.clone(),
        };
        let input_json = serde_json::to_string(&input)
            .map_err(|e| ValidationError::PlaywrightFailed(e.to_string()))?;

        tracing::debug!(
            script_path = %self.playwright_script_path,
            cookies_count = %cookies.len(),
            "Starting Playwright validation"
        );

        // 调用Playwright脚本
        let output = Command::new("node")
            .arg(&self.playwright_script_path)
            .arg(&input_json)
            .output()
            .await
            .map_err(|e| {
                tracing::error!(
                    error = %e,
                    script_path = %self.playwright_script_path,
                    "Failed to execute Playwright script"
                );
                ValidationError::PlaywrightFailed(format!("Failed to execute: {}", e))
            })?;

        // 检查退出状态
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            tracing::error!(
                exit_code = ?output.status.code(),
                stderr = %stderr,
                "Playwright script exited with error"
            );
            return Err(ValidationError::PlaywrightFailed(stderr.to_string()));
        }

        // 解析输出
        let result: PlaywrightValidationResult =
            serde_json::from_slice(&output.stdout).map_err(|e| {
                let stdout = String::from_utf8_lossy(&output.stdout);
                tracing::error!(
                    error = %e,
                    stdout = %stdout,
                    "Failed to parse Playwright output"
                );
                ValidationError::PlaywrightFailed(format!(
                    "Failed to parse output: {}. Output: {}",
                    e, stdout
                ))
            })?;

        // 检查验证结果
        if !result.valid {
            let error_msg = result.error.unwrap_or_else(|| "Unknown error".to_string());
            tracing::warn!(
                error = %error_msg,
                cookies_count = %cookies.len(),
                "Cookies validation failed"
            );
            return Err(ValidationError::ProfileApiFailed {
                status: 401,
                message: error_msg,
            });
        }

        // 提取UID和昵称
        let uid = result.uid.ok_or_else(|| {
            tracing::error!("Validation result missing UID");
            ValidationError::UidExtractionFailed("Missing UID in validation result".into())
        })?;

        let screen_name = result.screen_name.unwrap_or_else(|| "Unknown".to_string());

        tracing::info!(
            uid = %uid,
            screen_name = %screen_name,
            "Cookies validation successful"
        );

        Ok((uid, screen_name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // 需要实际的Playwright脚本
    async fn test_validate_cookies() {
        let service =
            ValidationService::new("/workspace/desktop/playwright/validate-cookies.js".to_string());

        let mut cookies = HashMap::new();
        cookies.insert("SUB".to_string(), "invalid_token".to_string());
        cookies.insert("SUBP".to_string(), "invalid_subp".to_string());

        let result = service.validate_cookies(&cookies).await;
        // 预期失败(无效的cookies)
        assert!(result.is_err());
    }

    #[test]
    fn test_service_creation() {
        let service = ValidationService::new("/path/to/script.js".to_string());
        assert_eq!(service.playwright_script_path, "/path/to/script.js");
    }
}
