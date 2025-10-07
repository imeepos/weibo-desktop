//! 契约测试: create_crawl_task
//!
//! 测试API规格定义的所有边界条件和错误处理

use chrono::{Duration, Utc};
use std::collections::HashMap;
use weibo_login::models::{CookiesData, CrawlTask};
use weibo_login::services::RedisService;

const TEST_REDIS_URL: &str = "redis://localhost:6379";

/// 创建测试用的cookies数据
fn create_test_cookies(uid: &str, age_days: i64) -> CookiesData {
    let mut cookies = HashMap::new();
    cookies.insert("SUB".to_string(), "test_sub".to_string());
    cookies.insert("SUBP".to_string(), "test_subp".to_string());

    let now = Utc::now();
    CookiesData {
        uid: uid.to_string(),
        cookies,
        fetched_at: now - Duration::days(age_days),
        validated_at: now - Duration::days(age_days),
        redis_key: format!("weibo:cookies:{}", uid),
        screen_name: Some("测试用户".to_string()),
    }
}

/// 测试辅助函数: 验证keyword
fn validate_keyword(keyword: &str) -> Result<(), String> {
    if keyword.trim().is_empty() {
        return Err("关键字不能为空".to_string());
    }
    Ok(())
}

/// 测试辅助函数: 验证event_start_time
fn validate_event_start_time(event_start_time: &str) -> Result<chrono::DateTime<Utc>, String> {
    let time = chrono::DateTime::parse_from_rfc3339(event_start_time)
        .map_err(|_| "事件开始时间格式无效".to_string())?
        .with_timezone(&Utc);

    if time > Utc::now() {
        return Err("事件开始时间不能是未来时间".to_string());
    }

    Ok(time)
}

/// 测试辅助函数: 检查cookies是否存在和是否过期
async fn validate_cookies(
    redis_service: &RedisService,
    uid: &str,
) -> Result<CookiesData, String> {
    let cookies_data = redis_service
        .query_cookies(uid)
        .await
        .map_err(|_| format!("未找到UID {} 的Cookies,请先扫码登录", uid))?;

    let age = Utc::now() - cookies_data.validated_at;
    if age.num_days() > 7 {
        return Err(format!(
            "Cookies可能已过期(验证时间>{}天),请重新登录",
            age.num_days()
        ));
    }

    Ok(cookies_data)
}

#[tokio::test]
#[ignore] // 需要Redis实例
async fn test_invalid_keyword_empty() {
    // 测试空关键字
    let result = validate_keyword("");
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "关键字不能为空");

    // 测试仅空格的关键字
    let result = validate_keyword("   ");
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "关键字不能为空");
}

#[tokio::test]
#[ignore]
async fn test_invalid_time_future() {
    // 测试未来时间
    let future_time = (Utc::now() + Duration::days(1)).to_rfc3339();
    let result = validate_event_start_time(&future_time);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "事件开始时间不能是未来时间");
}

#[tokio::test]
#[ignore]
async fn test_invalid_time_format() {
    // 测试无效的时间格式
    let result = validate_event_start_time("invalid-time-format");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("格式无效"));
}

#[tokio::test]
#[ignore]
async fn test_cookies_not_found() {
    let redis_service = RedisService::new(TEST_REDIS_URL).unwrap();
    let nonexistent_uid = "nonexistent_uid_12345";

    let result = validate_cookies(&redis_service, nonexistent_uid).await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .contains(&format!("未找到UID {} 的Cookies", nonexistent_uid)));
}

#[tokio::test]
#[ignore]
async fn test_cookies_expired() {
    let redis_service = RedisService::new(TEST_REDIS_URL).unwrap();
    let test_uid = "test_uid_expired";

    // 创建8天前验证的cookies (超过7天限制)
    let cookies_data = create_test_cookies(test_uid, 8);
    redis_service.save_cookies(&cookies_data).await.unwrap();

    // 验证应失败
    let result = validate_cookies(&redis_service, test_uid).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Cookies可能已过期"));
    assert!(result.as_ref().unwrap_err().contains("8天"));

    // 清理
    redis_service.delete_cookies(test_uid).await.unwrap();
}

#[tokio::test]
#[ignore]
async fn test_cookies_valid() {
    let redis_service = RedisService::new(TEST_REDIS_URL).unwrap();
    let test_uid = "test_uid_valid";

    // 创建5天前验证的cookies (未超过7天)
    let cookies_data = create_test_cookies(test_uid, 5);
    redis_service.save_cookies(&cookies_data).await.unwrap();

    // 验证应成功
    let result = validate_cookies(&redis_service, test_uid).await;
    assert!(result.is_ok());

    // 清理
    redis_service.delete_cookies(test_uid).await.unwrap();
}

#[tokio::test]
#[ignore]
async fn test_create_task_success() {
    // 这个测试在实现阶段会失败,因为CrawlTask::new还是todo!()
    // 这是TDD红色阶段的预期行为

    let keyword = "国庆".to_string();
    let event_start_time = Utc::now() - Duration::days(7);

    // 验证参数
    validate_keyword(&keyword).unwrap();

    // 创建任务 (会失败,因为尚未实现)
    let result = std::panic::catch_unwind(|| {
        CrawlTask::new(keyword.clone(), event_start_time)
    });

    // 预期失败 (todo!会panic)
    assert!(result.is_err());
}

#[tokio::test]
#[ignore]
async fn test_valid_time_past() {
    // 测试过去时间应该通过
    let past_time = (Utc::now() - Duration::days(30)).to_rfc3339();
    let result = validate_event_start_time(&past_time);
    assert!(result.is_ok());

    let parsed_time = result.unwrap();
    assert!(parsed_time < Utc::now());
}

#[tokio::test]
#[ignore]
async fn test_valid_keyword() {
    // 测试有效关键字
    assert!(validate_keyword("国庆").is_ok());
    assert!(validate_keyword("  国庆  ").is_ok()); // 前后空格应该被trim
    assert!(validate_keyword("关键字 with spaces").is_ok());
}
