use serde::Deserialize;
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
            脚本路径 = %playwright_script_path,
            "验证服务初始化完成"
        );
        Self {
            playwright_script_path,
        }
    }

    /// 执行Playwright脚本
    async fn execute_playwright_script(
        &self,
        input_json: &str,
    ) -> Result<std::process::Output, ValidationError> {
        tracing::debug!(
            node命令 = "node",
            脚本路径 = %self.playwright_script_path,
            输入JSON = %input_json,
            "执行Playwright脚本"
        );

        let output = Command::new("node")
            .arg(&self.playwright_script_path)
            .arg(input_json)
            .output()
            .await
            .map_err(|e| {
                tracing::error!(
                    错误 = %e,
                    脚本路径 = %self.playwright_script_path,
                    "执行Playwright脚本失败"
                );
                ValidationError::PlaywrightFailed(format!("Failed to execute: {}", e))
            })?;

        if !output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            tracing::error!(
                退出码 = ?output.status.code(),
                标准输出 = %stdout,
                错误输出 = %stderr,
                "Playwright脚本执行出错"
            );
            return Err(ValidationError::PlaywrightFailed(format!(
                "Script failed with exit code {:?}. stdout: {}. stderr: {}",
                output.status.code(),
                stdout,
                stderr
            )));
        }

        Ok(output)
    }

    /// 解析验证结果
    fn parse_validation_result(
        output: &[u8],
    ) -> Result<PlaywrightValidationResult, ValidationError> {
        serde_json::from_slice(output).map_err(|e| {
            let stdout = String::from_utf8_lossy(output);
            tracing::error!(错误 = %e, 标准输出 = %stdout, "解析Playwright输出失败");
            ValidationError::PlaywrightFailed(format!(
                "Failed to parse output: {}. Output: {}",
                e, stdout
            ))
        })
    }

    /// 提取用户信息
    fn extract_user_info(
        result: PlaywrightValidationResult,
    ) -> Result<(String, String), ValidationError> {
        if !result.valid {
            let error_msg = result.error.unwrap_or_else(|| "Unknown error".to_string());
            tracing::warn!(错误 = %error_msg, "Cookies验证失败");
            return Err(ValidationError::ProfileApiFailed {
                status: 401,
                message: error_msg,
            });
        }

        let uid = result.uid.ok_or_else(|| {
            tracing::error!("验证结果缺少用户ID");
            ValidationError::UidExtractionFailed("Missing UID in validation result".into())
        })?;

        let screen_name = result.screen_name.unwrap_or_else(|| "Unknown".to_string());

        tracing::info!(用户ID = %uid, 昵称 = %screen_name, "Cookies验证成功");
        Ok((uid, screen_name))
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
    /// - 输入: JSON字符串作为第一个参数,格式 `{"SUB": "xxx", "SUBP": "yyy"}`
    /// - 输出: JSON到stdout,格式 `{"valid": bool, "uid": string, "screen_name": string, "error": string}`
    /// - 退出码: 0表示脚本执行成功(但valid可能为false),非0表示脚本崩溃
    pub async fn validate_cookies(
        &self,
        cookies: &HashMap<String, String>,
    ) -> Result<(String, String), ValidationError> {
        // 序列化输入：直接传递cookies键值对
        let input_json = serde_json::to_string(cookies)
            .map_err(|e| ValidationError::PlaywrightFailed(e.to_string()))?;

        tracing::debug!(
            脚本路径 = %self.playwright_script_path,
            cookies数量 = %cookies.len(),
            "开始Playwright验证"
        );

        // 执行脚本
        let output = self.execute_playwright_script(&input_json).await?;

        // 解析结果
        let result = Self::parse_validation_result(&output.stdout)?;

        // 提取用户信息
        Self::extract_user_info(result)
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
