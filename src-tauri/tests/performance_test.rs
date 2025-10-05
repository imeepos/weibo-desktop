//! 性能测试
//!
//! 验证各操作的性能指标符合要求。
//! 所有测试使用Mock服务,专注于测量业务逻辑性能。

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

mod common;
use common::{create_test_cookies, MockRedisService, MockValidationService};

#[cfg(test)]
mod perf_tests {
    use super::*;

    /// 测试Redis保存性能
    /// 要求: < 100ms (P95)
    #[tokio::test]
    async fn test_redis_save_performance() {
        let redis = MockRedisService::new();
        let mut durations = Vec::new();

        // 执行100次保存操作
        for i in 0..100 {
            let start = Instant::now();

            let key = format!("perf_test_key_{}", i);
            let cookies = create_test_cookies();
            let cookies_json = serde_json::to_string(&cookies).unwrap();

            redis.hset(&key, "cookies", cookies_json).await.unwrap();
            redis
                .hset(&key, "screen_name", "测试用户".to_string())
                .await
                .unwrap();
            redis
                .hset(&key, "fetched_at", "1234567890".to_string())
                .await
                .unwrap();

            durations.push(start.elapsed());
        }

        // 计算P95
        durations.sort();
        let p95_index = (durations.len() as f64 * 0.95) as usize;
        let p95 = durations[p95_index];

        println!("Redis保存性能 P95: {}ms", p95.as_millis());
        println!("Redis保存性能 P50: {}ms", durations[50].as_millis());
        println!("Redis保存性能 平均: {}ms", calculate_average(&durations));

        assert!(
            p95.as_millis() < 100,
            "P95延迟 {}ms 超过100ms要求",
            p95.as_millis()
        );
    }

    /// 测试Redis查询性能
    /// 要求: < 50ms (P95)
    #[tokio::test]
    async fn test_redis_query_performance() {
        let redis = MockRedisService::new();

        // 先保存测试数据
        let cookies = create_test_cookies();
        let cookies_json = serde_json::to_string(&cookies).unwrap();
        redis
            .hset("test_key", "cookies", cookies_json)
            .await
            .unwrap();
        redis
            .hset("test_key", "screen_name", "测试用户".to_string())
            .await
            .unwrap();

        let mut durations = Vec::new();

        // 执行100次查询
        for _ in 0..100 {
            let start = Instant::now();

            let _cookies = redis.hget("test_key", "cookies").await.unwrap();
            let _screen_name = redis.hget("test_key", "screen_name").await.unwrap();

            durations.push(start.elapsed());
        }

        // 计算P95
        durations.sort();
        let p95_index = (durations.len() as f64 * 0.95) as usize;
        let p95 = durations[p95_index];

        println!("Redis查询性能 P95: {}ms", p95.as_millis());
        println!("Redis查询性能 P50: {}ms", durations[50].as_millis());
        println!("Redis查询性能 平均: {}ms", calculate_average(&durations));

        assert!(
            p95.as_millis() < 50,
            "P95延迟 {}ms 超过50ms要求",
            p95.as_millis()
        );
    }

    /// 测试并发性能
    /// 要求: 支持50个并发请求
    #[tokio::test]
    async fn test_concurrent_performance() {
        use tokio::task::JoinSet;

        let redis = Arc::new(MockRedisService::new());
        let mut tasks = JoinSet::new();

        let start = Instant::now();

        // 启动50个并发任务
        for i in 0..50 {
            let redis_clone = Arc::clone(&redis);
            tasks.spawn(async move {
                let key = format!("concurrent_key_{}", i);
                let cookies = create_test_cookies();
                let cookies_json = serde_json::to_string(&cookies).unwrap();

                redis_clone
                    .hset(&key, "cookies", cookies_json)
                    .await
                    .unwrap();
                redis_clone.hget(&key, "cookies").await.unwrap()
            });
        }

        // 等待所有任务完成
        while let Some(result) = tasks.join_next().await {
            assert!(result.is_ok());
            let value = result.unwrap();
            assert!(value.is_some());
        }

        let total_duration = start.elapsed();

        println!("50个并发请求总耗时: {}ms", total_duration.as_millis());
        println!(
            "平均每个请求耗时: {}ms",
            total_duration.as_millis() / 50
        );

        // 验证: 50个并发请求在5秒内完成
        assert!(
            total_duration.as_secs() < 5,
            "并发操作耗时 {}s 超过5s要求",
            total_duration.as_secs()
        );
    }

    /// 测试内存使用
    #[tokio::test]
    async fn test_memory_usage() {
        let redis = MockRedisService::new();

        // 保存1000个账户的cookies
        for i in 0..1000 {
            let key = format!("memory_test_key_{}", i);
            let cookies = create_test_cookies();
            let cookies_json = serde_json::to_string(&cookies).unwrap();

            redis.hset(&key, "cookies", cookies_json).await.unwrap();
            redis
                .hset(&key, "screen_name", format!("用户{}", i))
                .await
                .unwrap();
            redis
                .hset(&key, "fetched_at", "1234567890".to_string())
                .await
                .unwrap();
        }

        // 验证所有数据都可以查询
        for i in 0..1000 {
            let key = format!("memory_test_key_{}", i);
            let exists = redis.exists(&key).await.unwrap();
            assert!(exists, "Key {} should exist", key);
        }

        // 验证数据正确性 (抽样检查)
        for i in (0..1000).step_by(100) {
            let key = format!("memory_test_key_{}", i);
            let cookies_str = redis.hget(&key, "cookies").await.unwrap().unwrap();
            let cookies: HashMap<String, String> = serde_json::from_str(&cookies_str).unwrap();

            assert!(cookies.contains_key("SUB"));
            assert!(cookies.contains_key("SUBP"));
        }

        // 清理测试数据
        for i in 0..1000 {
            let key = format!("memory_test_key_{}", i);
            redis.delete(&key).await.unwrap();
        }

        // 验证清理成功
        for i in 0..1000 {
            let key = format!("memory_test_key_{}", i);
            let exists = redis.exists(&key).await.unwrap();
            assert!(!exists, "Key {} should not exist after cleanup", key);
        }

        println!("内存测试通过: 成功保存、查询、清理1000个账户数据");
    }

    /// 测试Cookies验证性能
    /// 要求: < 2s
    #[tokio::test]
    async fn test_validation_performance() {
        let mut durations = Vec::new();

        // 执行100次验证
        for _ in 0..100 {
            let validator = MockValidationService::new_success();
            let start = Instant::now();

            let result = validator.validate().await;
            assert!(result.is_ok());

            durations.push(start.elapsed());
        }

        // 计算P95
        durations.sort();
        let p95_index = (durations.len() as f64 * 0.95) as usize;
        let p95 = durations[p95_index];

        println!("Cookies验证性能 P95: {}ms", p95.as_millis());
        println!("Cookies验证性能 P50: {}ms", durations[50].as_millis());
        println!("Cookies验证性能 平均: {}ms", calculate_average(&durations));

        assert!(
            p95.as_millis() < 2000,
            "P95延迟 {}ms 超过2000ms要求",
            p95.as_millis()
        );
    }

    /// 测试序列化性能
    #[tokio::test]
    async fn test_serialization_performance() {
        let cookies = create_test_cookies();
        let mut durations = Vec::new();

        // 执行1000次序列化
        for _ in 0..1000 {
            let start = Instant::now();
            let _json = serde_json::to_string(&cookies).unwrap();
            durations.push(start.elapsed());
        }

        // 计算P95
        durations.sort();
        let p95_index = (durations.len() as f64 * 0.95) as usize;
        let p95 = durations[p95_index];

        println!("序列化性能 P95: {}μs", p95.as_micros());
        println!("序列化性能 P50: {}μs", durations[500].as_micros());

        // 序列化应该非常快
        assert!(
            p95.as_millis() < 10,
            "序列化P95延迟 {}ms 超过10ms",
            p95.as_millis()
        );
    }

    /// 测试反序列化性能
    #[tokio::test]
    async fn test_deserialization_performance() {
        let cookies = create_test_cookies();
        let cookies_json = serde_json::to_string(&cookies).unwrap();
        let mut durations = Vec::new();

        // 执行1000次反序列化
        for _ in 0..1000 {
            let start = Instant::now();
            let _parsed: HashMap<String, String> =
                serde_json::from_str(&cookies_json).unwrap();
            durations.push(start.elapsed());
        }

        // 计算P95
        durations.sort();
        let p95_index = (durations.len() as f64 * 0.95) as usize;
        let p95 = durations[p95_index];

        println!("反序列化性能 P95: {}μs", p95.as_micros());
        println!("反序列化性能 P50: {}μs", durations[500].as_micros());

        // 反序列化应该非常快
        assert!(
            p95.as_millis() < 10,
            "反序列化P95延迟 {}ms 超过10ms",
            p95.as_millis()
        );
    }

    /// 测试大规模并发性能
    #[tokio::test]
    async fn test_large_scale_concurrent_performance() {
        use tokio::task::JoinSet;

        let redis = Arc::new(MockRedisService::new());
        let mut tasks = JoinSet::new();

        let start = Instant::now();

        // 启动200个并发任务 (更大规模)
        for i in 0..200 {
            let redis_clone = Arc::clone(&redis);
            tasks.spawn(async move {
                let key = format!("large_concurrent_key_{}", i);
                let cookies = create_test_cookies();
                let cookies_json = serde_json::to_string(&cookies).unwrap();

                // 保存
                redis_clone
                    .hset(&key, "cookies", cookies_json)
                    .await
                    .unwrap();

                // 查询
                redis_clone.hget(&key, "cookies").await.unwrap()
            });
        }

        // 等待所有任务完成
        let mut success_count = 0;
        while let Some(result) = tasks.join_next().await {
            assert!(result.is_ok());
            success_count += 1;
        }

        let total_duration = start.elapsed();

        println!("200个并发请求总耗时: {}ms", total_duration.as_millis());
        println!(
            "平均每个请求耗时: {}ms",
            total_duration.as_millis() / 200
        );
        println!("成功完成: {} / 200", success_count);

        assert_eq!(success_count, 200);
        assert!(
            total_duration.as_secs() < 10,
            "200个并发操作耗时 {}s 超过10s要求",
            total_duration.as_secs()
        );
    }

    /// 计算平均延迟
    fn calculate_average(durations: &[std::time::Duration]) -> u128 {
        let sum: u128 = durations.iter().map(|d| d.as_millis()).sum();
        sum / durations.len() as u128
    }
}
