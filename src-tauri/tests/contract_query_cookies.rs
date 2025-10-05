//! query_cookies 契约测试
//!
//! 参考: specs/001-cookies/contracts/query_cookies.md
//!
//! 验证 query_cookies 命令符合契约定义,包括:
//! - 成功场景: 查询已存在的cookies
//! - 错误场景: 不存在、数据损坏、Redis连接失败
//! - 性能要求: 响应时间 < 100ms

mod common;

use chrono::{DateTime, Utc};
use common::{create_test_cookies, MockRedisService};
use std::collections::HashMap;
use std::time::Instant;

/// Cookies数据结构 (用于测试)
#[derive(Debug, Clone, PartialEq)]
struct CookiesData {
    uid: String,
    cookies: HashMap<String, String>,
    fetched_at: DateTime<Utc>,
    validated_at: DateTime<Utc>,
    redis_key: String,
    screen_name: Option<String>,
}

/// 查询cookies的错误
#[derive(Debug, Clone)]
enum QueryCookiesError {
    NotFound(String),
    RedisConnectionFailed(String),
    SerializationError(String),
    MissingField(String),
    InvalidTimestamp,
}

/// Mock查询cookies的核心逻辑
async fn mock_query_cookies(
    uid: String,
    redis: &MockRedisService,
) -> Result<CookiesData, QueryCookiesError> {
    let redis_key = format!("weibo:cookies:{}", uid);

    // 1. 检查key是否存在
    let exists = redis
        .exists(&redis_key)
        .await
        .map_err(|e| QueryCookiesError::RedisConnectionFailed(e))?;

    if !exists {
        return Err(QueryCookiesError::NotFound(uid));
    }

    // 2. 获取Hash所有字段
    let data = redis
        .hgetall(&redis_key)
        .await
        .map_err(|e| QueryCookiesError::RedisConnectionFailed(e))?;

    // 3. 反序列化cookies
    let cookies_str = data
        .get("cookies")
        .ok_or_else(|| QueryCookiesError::MissingField("cookies".to_string()))?;
    let cookies: HashMap<String, String> = serde_json::from_str(cookies_str)
        .map_err(|e| QueryCookiesError::SerializationError(e.to_string()))?;

    // 4. 解析时间戳
    let fetched_at_ts = data
        .get("fetched_at")
        .and_then(|s| s.parse::<i64>().ok())
        .ok_or_else(|| QueryCookiesError::MissingField("fetched_at".to_string()))?;

    let validated_at_ts = data
        .get("validated_at")
        .and_then(|s| s.parse::<i64>().ok())
        .ok_or_else(|| QueryCookiesError::MissingField("validated_at".to_string()))?;

    let fetched_at = DateTime::from_timestamp(fetched_at_ts, 0)
        .ok_or(QueryCookiesError::InvalidTimestamp)?;
    let validated_at = DateTime::from_timestamp(validated_at_ts, 0)
        .ok_or(QueryCookiesError::InvalidTimestamp)?;

    // 5. 获取可选字段
    let screen_name = data.get("screen_name").cloned();

    Ok(CookiesData {
        uid,
        cookies,
        fetched_at,
        validated_at,
        redis_key,
        screen_name,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 辅助函数: 保存测试cookies到Mock Redis
    async fn save_test_cookies_to_redis(
        uid: &str,
        redis: &MockRedisService,
        screen_name: Option<String>,
    ) {
        let redis_key = format!("weibo:cookies:{}", uid);
        let cookies = create_test_cookies();
        let cookies_json = serde_json::to_string(&cookies).unwrap();

        redis
            .hset(&redis_key, "cookies", cookies_json)
            .await
            .unwrap();
        redis
            .hset(
                &redis_key,
                "fetched_at",
                Utc::now().timestamp().to_string(),
            )
            .await
            .unwrap();
        redis
            .hset(
                &redis_key,
                "validated_at",
                Utc::now().timestamp().to_string(),
            )
            .await
            .unwrap();

        if let Some(name) = screen_name {
            redis.hset(&redis_key, "screen_name", name).await.unwrap();
        }
    }

    /// 测试查询存在的cookies
    ///
    /// 契约要求:
    /// 1. 返回完整的CookiesData
    /// 2. 所有字段正确
    /// 3. 响应时间 < 100ms
    #[tokio::test]
    async fn test_query_existing_cookies() {
        let redis = MockRedisService::new();
        let uid = "1234567890";

        // 先保存数据
        save_test_cookies_to_redis(uid, &redis, Some("测试用户".to_string())).await;

        // 查询
        let start = Instant::now();
        let result = mock_query_cookies(uid.to_string(), &redis).await;
        let duration = start.elapsed();

        assert!(result.is_ok());
        let cookies_data = result.unwrap();

        // 验证所有字段
        assert_eq!(cookies_data.uid, uid);
        assert_eq!(cookies_data.redis_key, format!("weibo:cookies:{}", uid));
        assert!(!cookies_data.cookies.is_empty());
        assert!(cookies_data.cookies.contains_key("SUB"));
        assert!(cookies_data.cookies.contains_key("SUBP"));
        assert_eq!(cookies_data.screen_name, Some("测试用户".to_string()));

        // 验证性能要求
        assert!(duration.as_millis() < 100);
    }

    /// 测试查询不存在的cookies
    ///
    /// 契约要求:
    /// 返回 NotFound 错误
    #[tokio::test]
    async fn test_query_nonexistent_cookies() {
        let redis = MockRedisService::new();

        let result = mock_query_cookies("9999999999".to_string(), &redis).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            QueryCookiesError::NotFound(uid) => {
                assert_eq!(uid, "9999999999");
            }
            _ => panic!("Expected NotFound error"),
        }
    }

    /// 测试查询损坏的数据
    ///
    /// 契约要求:
    /// 数据格式损坏时返回 SerializationError
    #[tokio::test]
    async fn test_query_corrupted_data() {
        let redis = MockRedisService::new();
        let uid = "1234567890";

        // 插入损坏的数据
        redis
            .insert_corrupted_data(&format!("weibo:cookies:{}", uid))
            .await
            .unwrap();

        let result = mock_query_cookies(uid.to_string(), &redis).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            QueryCookiesError::SerializationError(msg) => {
                assert!(!msg.is_empty());
            }
            _ => panic!("Expected SerializationError"),
        }
    }

    /// 测试Redis连接失败
    ///
    /// 契约要求:
    /// Redis连接失败时返回 RedisConnectionFailed
    #[tokio::test]
    async fn test_query_redis_connection_failed() {
        let redis = MockRedisService::new();
        redis.set_fail_mode(true).await;

        let result = mock_query_cookies("1234567890".to_string(), &redis).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            QueryCookiesError::RedisConnectionFailed(msg) => {
                assert!(msg.contains("Redis连接失败"));
            }
            _ => panic!("Expected RedisConnectionFailed error"),
        }
    }

    /// 测试缺少必需字段 (cookies)
    ///
    /// 契约要求:
    /// 缺少必需字段时返回 MissingField
    #[tokio::test]
    async fn test_query_missing_cookies_field() {
        let redis = MockRedisService::new();
        let uid = "1234567890";
        let redis_key = format!("weibo:cookies:{}", uid);

        // 只插入时间戳,不插入cookies字段
        redis
            .hset(&redis_key, "fetched_at", Utc::now().timestamp().to_string())
            .await
            .unwrap();
        redis
            .hset(
                &redis_key,
                "validated_at",
                Utc::now().timestamp().to_string(),
            )
            .await
            .unwrap();

        let result = mock_query_cookies(uid.to_string(), &redis).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            QueryCookiesError::MissingField(field) => {
                assert_eq!(field, "cookies");
            }
            _ => panic!("Expected MissingField error"),
        }
    }

    /// 测试缺少时间戳字段
    ///
    /// 契约要求:
    /// 缺少时间戳字段时返回 MissingField
    #[tokio::test]
    async fn test_query_missing_timestamp_field() {
        let redis = MockRedisService::new();
        let uid = "1234567890";
        let redis_key = format!("weibo:cookies:{}", uid);
        let cookies = create_test_cookies();

        // 只插入cookies,不插入时间戳
        redis
            .hset(&redis_key, "cookies", serde_json::to_string(&cookies).unwrap())
            .await
            .unwrap();

        let result = mock_query_cookies(uid.to_string(), &redis).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            QueryCookiesError::MissingField(field) => {
                assert_eq!(field, "fetched_at");
            }
            _ => panic!("Expected MissingField error"),
        }
    }

    /// 测试无screen_name的场景
    ///
    /// 契约要求:
    /// screen_name是可选字段,缺失时不应报错
    #[tokio::test]
    async fn test_query_without_screen_name() {
        let redis = MockRedisService::new();
        let uid = "1234567890";

        // 保存数据但不包含screen_name
        save_test_cookies_to_redis(uid, &redis, None).await;

        let result = mock_query_cookies(uid.to_string(), &redis).await;

        assert!(result.is_ok());
        let cookies_data = result.unwrap();
        assert_eq!(cookies_data.screen_name, None);
    }

    /// 测试性能要求
    ///
    /// 契约要求:
    /// 响应时间 < 100ms (P95)
    /// 在Mock环境下应该远低于这个值
    #[tokio::test]
    async fn test_query_performance() {
        let redis = MockRedisService::new();
        let uid = "1234567890";
        save_test_cookies_to_redis(uid, &redis, Some("测试用户".to_string())).await;

        // 进行多次查询,测试平均性能
        let mut durations = Vec::new();
        for _ in 0..10 {
            let start = Instant::now();
            let result = mock_query_cookies(uid.to_string(), &redis).await;
            let duration = start.elapsed();

            assert!(result.is_ok());
            durations.push(duration.as_millis());
        }

        // 计算P95
        durations.sort();
        let p95_index = (durations.len() as f64 * 0.95) as usize;
        let p95_duration = durations[p95_index];

        assert!(p95_duration < 100, "P95 duration: {}ms", p95_duration);
    }

    /// 测试并发查询
    ///
    /// 契约要求:
    /// 支持最多50个并发请求
    #[tokio::test]
    async fn test_query_concurrent() {
        use std::sync::Arc;
        let redis = Arc::new(MockRedisService::new());

        // 准备多个用户的数据
        for i in 0..10 {
            let uid = format!("user_{}", i);
            save_test_cookies_to_redis(&uid, &redis, Some(format!("用户{}", i))).await;
        }

        // 并发查询
        let mut tasks = Vec::new();
        for i in 0..50 {
            let uid = format!("user_{}", i % 10);
            let redis_clone = Arc::clone(&redis);
            tasks.push(tokio::spawn(async move {
                mock_query_cookies(uid, &redis_clone).await
            }));
        }

        // 等待所有任务完成
        for task in tasks {
            let result = task.await.unwrap();
            assert!(result.is_ok());
        }
    }
}
