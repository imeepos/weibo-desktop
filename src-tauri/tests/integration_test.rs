//! 端到端集成测试
//!
//! 测试完整的微博扫码登录流程,从生成二维码到保存cookies。
//! 使用Mock服务避免外部依赖,专注验证业务逻辑和数据流转。

use std::collections::HashMap;
use std::sync::Arc;

mod common;
use common::{create_test_cookies, MockRedisService, MockValidationService};

#[cfg(test)]
mod e2e_tests {
    use super::*;

    /// 测试完整登录流程
    ///
    /// 流程:
    /// 1. 生成二维码 (模拟)
    /// 2. 模拟扫码
    /// 3. 模拟确认登录
    /// 4. 验证cookies
    /// 5. 保存到Redis
    /// 6. 查询验证
    #[tokio::test]
    async fn test_complete_login_flow() {
        // 1. 初始化Mock服务
        let redis = Arc::new(MockRedisService::new());
        let validator = Arc::new(MockValidationService::new_success());

        // 2. 生成二维码 (模拟)
        let qr_id = "test_qr_123";
        let uid = "1234567890";

        // 3. 模拟扫码后获取cookies
        let cookies = create_test_cookies();

        // 4. 验证cookies (使用Mock)
        let (validated_uid, screen_name) = validator.validate().await.unwrap();
        assert_eq!(validated_uid, uid);
        assert_eq!(screen_name, "测试用户");

        // 5. 保存到Redis
        let redis_key = format!("weibo:cookies:{}", uid);
        let cookies_json = serde_json::to_string(&cookies).unwrap();
        redis.hset(&redis_key, "cookies", cookies_json).await.unwrap();

        // 保存其他元数据
        let timestamp = chrono::Utc::now().timestamp();
        redis
            .hset(&redis_key, "fetched_at", timestamp.to_string())
            .await
            .unwrap();
        redis
            .hset(&redis_key, "screen_name", screen_name.clone())
            .await
            .unwrap();

        // 6. 查询验证
        let stored = redis.hget(&redis_key, "cookies").await.unwrap().unwrap();
        let retrieved: HashMap<String, String> = serde_json::from_str(&stored).unwrap();

        assert_eq!(retrieved.get("SUB").unwrap(), "test_sub_value_123");
        assert_eq!(retrieved.get("SUBP").unwrap(), "test_subp_value_456");

        // 验证元数据
        let stored_screen_name = redis.hget(&redis_key, "screen_name").await.unwrap().unwrap();
        assert_eq!(stored_screen_name, "测试用户");

        let stored_timestamp = redis.hget(&redis_key, "fetched_at").await.unwrap().unwrap();
        let parsed_timestamp: i64 = stored_timestamp.parse().unwrap();
        assert!(parsed_timestamp > 0);
    }

    /// 测试二维码过期场景
    #[tokio::test]
    async fn test_qrcode_expiry_flow() {
        let redis = MockRedisService::new();

        // 1. 生成二维码
        let qr_id = "expiring_qr_456";
        let qr_key = format!("weibo:qrcode:{}", qr_id);

        // 2. 保存二维码状态
        let created_at = chrono::Utc::now().timestamp();
        redis
            .hset(&qr_key, "status", "pending".to_string())
            .await
            .unwrap();
        redis
            .hset(&qr_key, "created_at", created_at.to_string())
            .await
            .unwrap();

        // 3. 检查状态
        let status = redis.hget(&qr_key, "status").await.unwrap().unwrap();
        assert_eq!(status, "pending");

        // 4. 模拟过期 (300秒后)
        let now = created_at + 301;
        let qr_lifetime = 300;

        if now - created_at > qr_lifetime {
            redis
                .hset(&qr_key, "status", "expired".to_string())
                .await
                .unwrap();
        }

        // 5. 验证过期状态
        let updated_status = redis.hget(&qr_key, "status").await.unwrap().unwrap();
        assert_eq!(updated_status, "expired");

        // 6. 清理过期数据
        redis.delete(&qr_key).await.unwrap();
        let exists = redis.exists(&qr_key).await.unwrap();
        assert!(!exists);
    }

    /// 测试多账户并发登录
    #[tokio::test]
    async fn test_concurrent_login_flow() {
        use tokio::task::JoinSet;

        let redis = Arc::new(MockRedisService::new());
        let mut tasks = JoinSet::new();

        // 启动5个并发登录任务
        for i in 0..5 {
            let redis_clone = Arc::clone(&redis);
            tasks.spawn(async move {
                let uid = format!("user_{}", i);
                let validator = MockValidationService::new(
                    true,
                    uid.clone(),
                    format!("测试用户{}", i),
                );

                // 验证cookies
                let (validated_uid, screen_name) = validator.validate().await.unwrap();
                assert_eq!(validated_uid, uid);

                // 保存到Redis
                let redis_key = format!("weibo:cookies:{}", uid);
                let mut cookies = HashMap::new();
                cookies.insert("SUB".to_string(), format!("sub_{}", i));
                cookies.insert("SUBP".to_string(), format!("subp_{}", i));

                let cookies_json = serde_json::to_string(&cookies).unwrap();
                redis_clone
                    .hset(&redis_key, "cookies", cookies_json)
                    .await
                    .unwrap();
                redis_clone
                    .hset(&redis_key, "screen_name", screen_name)
                    .await
                    .unwrap();

                (uid, cookies)
            });
        }

        // 等待所有任务完成并验证
        let mut results = Vec::new();
        while let Some(result) = tasks.join_next().await {
            let (uid, cookies) = result.unwrap();
            results.push((uid.clone(), cookies));

            // 验证数据隔离
            let redis_key = format!("weibo:cookies:{}", uid);
            let stored = redis.hget(&redis_key, "cookies").await.unwrap().unwrap();
            let retrieved: HashMap<String, String> = serde_json::from_str(&stored).unwrap();

            let user_index = uid.strip_prefix("user_").unwrap();
            assert_eq!(
                retrieved.get("SUB").unwrap(),
                &format!("sub_{}", user_index)
            );
            assert_eq!(
                retrieved.get("SUBP").unwrap(),
                &format!("subp_{}", user_index)
            );
        }

        // 验证所有账户成功保存
        assert_eq!(results.len(), 5);

        // 验证无数据污染
        for i in 0..5 {
            let uid = format!("user_{}", i);
            let redis_key = format!("weibo:cookies:{}", uid);
            let exists = redis.exists(&redis_key).await.unwrap();
            assert!(exists, "User {} data should exist", i);
        }
    }

    /// 测试网络中断恢复
    #[tokio::test]
    async fn test_network_interruption_recovery() {
        let redis = Arc::new(MockRedisService::new());
        let uid = "network_test_user";
        let redis_key = format!("weibo:cookies:{}", uid);

        // 1. 第一次尝试保存 (成功)
        let cookies = create_test_cookies();
        let cookies_json = serde_json::to_string(&cookies).unwrap();
        redis
            .hset(&redis_key, "cookies", cookies_json.clone())
            .await
            .unwrap();

        // 2. 模拟网络中断
        redis.set_fail_mode(true).await;

        // 3. 验证操作失败
        let result = redis.hset(&redis_key, "updated", "true".to_string()).await;
        assert!(result.is_err());

        // 4. 网络恢复
        redis.set_fail_mode(false).await;

        // 5. 重试成功
        let retry_result = redis.hset(&redis_key, "updated", "true".to_string()).await;
        assert!(retry_result.is_ok());

        // 6. 验证数据完整性
        let stored = redis.hget(&redis_key, "cookies").await.unwrap().unwrap();
        let retrieved: HashMap<String, String> = serde_json::from_str(&stored).unwrap();
        assert_eq!(retrieved.get("SUB").unwrap(), "test_sub_value_123");

        let updated = redis.hget(&redis_key, "updated").await.unwrap().unwrap();
        assert_eq!(updated, "true");
    }

    /// 测试Redis故障恢复
    #[tokio::test]
    async fn test_redis_failure_recovery() {
        let redis = MockRedisService::new();
        let uid = "failure_test_user";
        let redis_key = format!("weibo:cookies:{}", uid);

        // 1. Redis不可用
        redis.set_fail_mode(true).await;

        // 2. 尝试保存 (失败)
        let cookies = create_test_cookies();
        let cookies_json = serde_json::to_string(&cookies).unwrap();
        let result = redis.hset(&redis_key, "cookies", cookies_json.clone()).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Redis连接失败");

        // 3. Redis恢复
        redis.set_fail_mode(false).await;

        // 4. 重新保存 (成功)
        let retry_result = redis.hset(&redis_key, "cookies", cookies_json).await;
        assert!(retry_result.is_ok());

        // 5. 验证数据正确保存
        let exists = redis.exists(&redis_key).await.unwrap();
        assert!(exists);

        let stored = redis.hget(&redis_key, "cookies").await.unwrap().unwrap();
        let retrieved: HashMap<String, String> = serde_json::from_str(&stored).unwrap();
        assert_eq!(retrieved.get("SUB").unwrap(), "test_sub_value_123");
        assert_eq!(retrieved.get("SUBP").unwrap(), "test_subp_value_456");
    }

    /// 测试登录会话状态转换
    #[tokio::test]
    async fn test_login_session_state_transitions() {
        let redis = MockRedisService::new();
        let qr_id = "state_test_qr";
        let qr_key = format!("weibo:qrcode:{}", qr_id);

        // 1. 初始状态: pending
        redis
            .hset(&qr_key, "status", "pending".to_string())
            .await
            .unwrap();
        let status = redis.hget(&qr_key, "status").await.unwrap().unwrap();
        assert_eq!(status, "pending");

        // 2. 扫码: pending -> scanned
        redis
            .hset(&qr_key, "status", "scanned".to_string())
            .await
            .unwrap();
        let status = redis.hget(&qr_key, "status").await.unwrap().unwrap();
        assert_eq!(status, "scanned");

        // 3. 确认登录: scanned -> confirmed
        redis
            .hset(&qr_key, "status", "confirmed".to_string())
            .await
            .unwrap();
        redis
            .hset(&qr_key, "uid", "1234567890".to_string())
            .await
            .unwrap();
        let status = redis.hget(&qr_key, "status").await.unwrap().unwrap();
        assert_eq!(status, "confirmed");

        // 4. 验证关联的UID
        let uid = redis.hget(&qr_key, "uid").await.unwrap().unwrap();
        assert_eq!(uid, "1234567890");

        // 5. 清理会话
        redis.delete(&qr_key).await.unwrap();
        let exists = redis.exists(&qr_key).await.unwrap();
        assert!(!exists);
    }

    /// 测试Cookies验证失败场景
    #[tokio::test]
    async fn test_cookies_validation_failure() {
        let redis = MockRedisService::new();
        let validator = MockValidationService::new_failure();

        // 1. 尝试验证无效cookies
        let result = validator.validate().await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("Profile API call failed with status 401"));

        // 2. 验证Redis中不应保存失败的cookies
        let uid = "invalid_user";
        let redis_key = format!("weibo:cookies:{}", uid);
        let exists = redis.exists(&redis_key).await.unwrap();
        assert!(!exists);
    }

    /// 测试数据序列化和反序列化
    #[tokio::test]
    async fn test_data_serialization_roundtrip() {
        let redis = MockRedisService::new();
        let uid = "serialization_test_user";
        let redis_key = format!("weibo:cookies:{}", uid);

        // 1. 创建复杂的cookies数据
        let mut cookies = HashMap::new();
        cookies.insert("SUB".to_string(), "test_sub_value_with_special_chars!@#$%".to_string());
        cookies.insert("SUBP".to_string(), "test_subp_value_123456789".to_string());
        cookies.insert("_T_WM".to_string(), "Chinese中文测试".to_string());

        // 2. 序列化并保存
        let cookies_json = serde_json::to_string(&cookies).unwrap();
        redis
            .hset(&redis_key, "cookies", cookies_json)
            .await
            .unwrap();

        // 3. 查询并反序列化
        let stored = redis.hget(&redis_key, "cookies").await.unwrap().unwrap();
        let retrieved: HashMap<String, String> = serde_json::from_str(&stored).unwrap();

        // 4. 验证数据完整性
        assert_eq!(retrieved.len(), cookies.len());
        assert_eq!(
            retrieved.get("SUB").unwrap(),
            "test_sub_value_with_special_chars!@#$%"
        );
        assert_eq!(retrieved.get("SUBP").unwrap(), "test_subp_value_123456789");
        assert_eq!(retrieved.get("_T_WM").unwrap(), "Chinese中文测试");
    }
}
