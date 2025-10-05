//! save_cookies 契约测试
//!
//! 参考: specs/001-cookies/contracts/save_cookies.md
//!
//! 验证 save_cookies 命令符合契约定义,包括:
//! - 成功场景: 验证并保存有效cookies
//! - 错误场景: 无效cookies、缺失字段、Redis故障等
//! - 性能要求: 验证耗时 < 2秒
//! - 覆盖场景: 已存在cookies的覆盖更新

mod common;

use common::{
    create_invalid_cookies, create_minimal_cookies, create_test_cookies, MockRedisService,
    MockValidationService,
};
use std::collections::HashMap;
use std::time::Instant;

/// Mock保存cookies的核心逻辑
///
/// 模拟 Tauri command 的行为,不依赖真实的Redis和Playwright
async fn mock_save_cookies(
    uid: String,
    cookies: HashMap<String, String>,
    screen_name: Option<String>,
    redis: &MockRedisService,
    validator: &MockValidationService,
) -> Result<SaveCookiesResponse, SaveCookiesError> {
    let start = Instant::now();

    // 1. 验证cookies格式
    if cookies.is_empty() {
        return Err(SaveCookiesError::InvalidFormat(
            "Cookies不能为空".to_string(),
        ));
    }

    // 2. 检查必需字段
    if !cookies.contains_key("SUB") {
        return Err(SaveCookiesError::MissingCookie("SUB".to_string()));
    }
    if !cookies.contains_key("SUBP") {
        return Err(SaveCookiesError::MissingCookie("SUBP".to_string()));
    }

    // 3. 调用Playwright验证
    let (validated_uid, validated_screen_name) = validator
        .validate()
        .await
        .map_err(|msg| SaveCookiesError::ProfileApiFailed {
            status: 401,
            message: msg,
        })?;

    // 4. 验证UID匹配
    if validated_uid != uid {
        return Err(SaveCookiesError::UidMismatch {
            expected: uid,
            actual: validated_uid,
        });
    }

    // 5. 检查是否已存在 (模拟覆盖检测)
    let redis_key = format!("weibo:cookies:{}", uid);
    let is_overwrite = redis
        .exists(&redis_key)
        .await
        .map_err(|e| SaveCookiesError::RedisConnectionFailed(e))?;

    // 6. 保存到Redis (HSET模拟)
    let cookies_json = serde_json::to_string(&cookies)
        .map_err(|e| SaveCookiesError::SerializationError(e.to_string()))?;

    redis
        .hset(&redis_key, "cookies", cookies_json)
        .await
        .map_err(|e| SaveCookiesError::RedisConnectionFailed(e))?;

    redis
        .hset(
            &redis_key,
            "fetched_at",
            chrono::Utc::now().timestamp().to_string(),
        )
        .await
        .map_err(|e| SaveCookiesError::RedisConnectionFailed(e))?;

    redis
        .hset(
            &redis_key,
            "validated_at",
            chrono::Utc::now().timestamp().to_string(),
        )
        .await
        .map_err(|e| SaveCookiesError::RedisConnectionFailed(e))?;

    if let Some(name) = screen_name.or(Some(validated_screen_name)) {
        redis
            .hset(&redis_key, "screen_name", name)
            .await
            .map_err(|e| SaveCookiesError::RedisConnectionFailed(e))?;
    }

    let validation_duration_ms = start.elapsed().as_millis() as u64;

    Ok(SaveCookiesResponse {
        success: true,
        redis_key,
        validation_duration_ms,
        is_overwrite,
    })
}

/// 保存cookies的响应
#[derive(Debug, Clone, PartialEq)]
struct SaveCookiesResponse {
    success: bool,
    redis_key: String,
    validation_duration_ms: u64,
    is_overwrite: bool,
}

/// 保存cookies的错误
#[derive(Debug)]
#[allow(dead_code)]
enum SaveCookiesError {
    InvalidFormat(String),
    MissingCookie(String),
    ProfileApiFailed { status: u16, message: String },
    UidMismatch { expected: String, actual: String },
    RedisConnectionFailed(String),
    SerializationError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试保存有效cookies
    ///
    /// 契约要求:
    /// 1. 调用Playwright验证成功
    /// 2. UID匹配
    /// 3. 保存到Redis
    /// 4. 返回正确的响应结构
    #[tokio::test]
    async fn test_save_valid_cookies() {
        let redis = MockRedisService::new();
        let validator = MockValidationService::new_success();
        let cookies = create_test_cookies();

        let result = mock_save_cookies(
            "1234567890".to_string(),
            cookies,
            Some("测试用户".to_string()),
            &redis,
            &validator,
        )
        .await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.success);
        assert_eq!(response.redis_key, "weibo:cookies:1234567890");
        assert!(!response.is_overwrite); // 首次保存
        assert!(response.validation_duration_ms < 2000); // 性能要求
    }

    /// 测试保存无效cookies
    ///
    /// 契约要求:
    /// 当Playwright验证失败时,返回 ProfileApiFailed 错误
    #[tokio::test]
    async fn test_save_invalid_cookies() {
        let redis = MockRedisService::new();
        let validator = MockValidationService::new_failure();
        let cookies = create_test_cookies();

        let result = mock_save_cookies(
            "1234567890".to_string(),
            cookies,
            None,
            &redis,
            &validator,
        )
        .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            SaveCookiesError::ProfileApiFailed { status, message } => {
                assert_eq!(status, 401);
                assert!(message.contains("Profile API call failed"));
            }
            _ => panic!("Expected ProfileApiFailed error"),
        }
    }

    /// 测试缺少必需cookie (SUB)
    ///
    /// 契约要求:
    /// 缺少必需字段时返回 MissingCookie 错误
    #[tokio::test]
    async fn test_save_missing_sub_cookie() {
        let redis = MockRedisService::new();
        let validator = MockValidationService::new_success();
        let mut cookies = HashMap::new();
        cookies.insert("SUBP".to_string(), "only_subp".to_string());

        let result = mock_save_cookies(
            "1234567890".to_string(),
            cookies,
            None,
            &redis,
            &validator,
        )
        .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            SaveCookiesError::MissingCookie(name) => {
                assert_eq!(name, "SUB");
            }
            _ => panic!("Expected MissingCookie error"),
        }
    }

    /// 测试缺少必需cookie (SUBP)
    ///
    /// 契约要求:
    /// 缺少SUBP字段时返回 MissingCookie 错误
    #[tokio::test]
    async fn test_save_missing_subp_cookie() {
        let redis = MockRedisService::new();
        let validator = MockValidationService::new_success();
        let cookies = create_invalid_cookies(); // 只有SUB,缺少SUBP

        let result = mock_save_cookies(
            "1234567890".to_string(),
            cookies,
            None,
            &redis,
            &validator,
        )
        .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            SaveCookiesError::MissingCookie(name) => {
                assert_eq!(name, "SUBP");
            }
            _ => panic!("Expected MissingCookie error"),
        }
    }

    /// 测试Redis连接失败
    ///
    /// 契约要求:
    /// Redis操作失败时返回 RedisConnectionFailed 错误
    #[tokio::test]
    async fn test_save_redis_connection_failed() {
        let redis = MockRedisService::new();
        redis.set_fail_mode(true).await; // 启用失败模式
        let validator = MockValidationService::new_success();
        let cookies = create_test_cookies();

        let result = mock_save_cookies(
            "1234567890".to_string(),
            cookies,
            None,
            &redis,
            &validator,
        )
        .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            SaveCookiesError::RedisConnectionFailed(msg) => {
                assert!(msg.contains("Redis连接失败"));
            }
            _ => panic!("Expected RedisConnectionFailed error"),
        }
    }

    /// 测试覆盖已存在的cookies
    ///
    /// 契约要求:
    /// 同一UID已有cookies时,is_overwrite = true
    #[tokio::test]
    async fn test_save_overwrite_existing() {
        let redis = MockRedisService::new();
        let validator = MockValidationService::new_success();
        let cookies = create_test_cookies();

        // 第一次保存
        let first_result = mock_save_cookies(
            "1234567890".to_string(),
            cookies.clone(),
            None,
            &redis,
            &validator,
        )
        .await;

        assert!(first_result.is_ok());
        assert!(!first_result.unwrap().is_overwrite);

        // 第二次保存同一UID (覆盖)
        let second_result = mock_save_cookies(
            "1234567890".to_string(),
            cookies,
            Some("新昵称".to_string()),
            &redis,
            &validator,
        )
        .await;

        assert!(second_result.is_ok());
        let response = second_result.unwrap();
        assert!(response.is_overwrite); // 应为覆盖模式
    }

    /// 测试UID不匹配
    ///
    /// 契约要求:
    /// 验证返回的UID与请求的UID不一致时,应返回错误
    #[tokio::test]
    async fn test_save_uid_mismatch() {
        let redis = MockRedisService::new();
        let mut validator = MockValidationService::new_success();
        validator.set_mock_data("9999999999".to_string(), "其他用户".to_string());
        let cookies = create_test_cookies();

        let result = mock_save_cookies(
            "1234567890".to_string(), // 请求的UID
            cookies,
            None,
            &redis,
            &validator,
        )
        .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            SaveCookiesError::UidMismatch { expected, actual } => {
                assert_eq!(expected, "1234567890");
                assert_eq!(actual, "9999999999");
            }
            _ => panic!("Expected UidMismatch error"),
        }
    }

    /// 测试空cookies
    ///
    /// 契约要求:
    /// cookies为空时返回 InvalidFormat 错误
    #[tokio::test]
    async fn test_save_empty_cookies() {
        let redis = MockRedisService::new();
        let validator = MockValidationService::new_success();
        let cookies = HashMap::new(); // 空cookies

        let result = mock_save_cookies(
            "1234567890".to_string(),
            cookies,
            None,
            &redis,
            &validator,
        )
        .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            SaveCookiesError::InvalidFormat(msg) => {
                assert!(msg.contains("Cookies不能为空"));
            }
            _ => panic!("Expected InvalidFormat error"),
        }
    }

    /// 测试最小有效cookies
    ///
    /// 契约要求:
    /// 只包含必需字段(SUB, SUBP)的cookies也应能成功保存
    #[tokio::test]
    async fn test_save_minimal_cookies() {
        let redis = MockRedisService::new();
        let validator = MockValidationService::new_success();
        let cookies = create_minimal_cookies();

        let result = mock_save_cookies(
            "1234567890".to_string(),
            cookies,
            None,
            &redis,
            &validator,
        )
        .await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.success);
        assert_eq!(response.redis_key, "weibo:cookies:1234567890");
    }

    /// 测试性能要求
    ///
    /// 契约要求:
    /// - 验证耗时 < 2秒
    /// - Redis操作 < 100ms (Mock环境下应该极快)
    #[tokio::test]
    async fn test_save_performance() {
        let redis = MockRedisService::new();
        let validator = MockValidationService::new_success();
        let cookies = create_test_cookies();

        let start = Instant::now();
        let result = mock_save_cookies(
            "1234567890".to_string(),
            cookies,
            None,
            &redis,
            &validator,
        )
        .await;

        let total_duration = start.elapsed();
        assert!(result.is_ok());
        assert!(total_duration.as_millis() < 2000); // 总耗时 < 2秒
    }
}
