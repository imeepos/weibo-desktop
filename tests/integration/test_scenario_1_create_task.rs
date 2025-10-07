/// 集成测试 - 场景1: 创建爬取任务
///
/// 验证目标:
/// - FR-001至FR-005 (任务管理)
/// - create_crawl_task命令的完整业务流程
///
/// 测试覆盖:
/// 1. 成功路径: 创建任务并验证Redis存储
/// 2. 边界测试: 关键字为空、未来时间、cookies过期
use chrono::{Duration, Utc};
use redis::AsyncCommands;
use serde_json::json;
use weibo_login::models::crawl_task::{CrawlStatus, CrawlTask};
use weibo_login::services::redis_service::RedisService;

/// 测试辅助: 创建测试用RedisService
async fn setup_redis() -> RedisService {
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    RedisService::new(&redis_url)
        .await
        .expect("Redis连接失败,请确保Redis服务运行")
}

/// 测试辅助: 清理测试数据
async fn cleanup_task(redis: &RedisService, task_id: &str) {
    let mut conn = redis.get_connection().await.unwrap();
    let _: () = conn.del(format!("crawl:task:{}", task_id)).await.unwrap();
}

/// 测试辅助: 准备有效cookies
async fn prepare_valid_cookies(redis: &RedisService, uid: &str) {
    let mut conn = redis.get_connection().await.unwrap();
    let key = format!("weibo:cookies:{}", uid);

    // 模拟cookies数据 (来自001-cookies功能)
    let _: () = conn.hset_multiple(&key, &[
        ("uid", uid),
        ("screen_name", "测试用户"),
        ("cookies", "SUB=test_sub;SUBP=test_subp"),
        ("validated_at", &Utc::now().timestamp().to_string()),
    ]).await.unwrap();

    // 设置TTL (30天)
    let _: () = conn.expire(&key, 30 * 24 * 3600).await.unwrap();
}

/// 测试辅助: 准备过期cookies
async fn prepare_expired_cookies(redis: &RedisService, uid: &str) {
    let mut conn = redis.get_connection().await.unwrap();
    let key = format!("weibo:cookies:{}", uid);

    // 设置8天前的验证时间 (超过7天阈值)
    let expired_time = Utc::now() - Duration::days(8);
    let _: () = conn.hset_multiple(&key, &[
        ("uid", uid),
        ("screen_name", "过期用户"),
        ("cookies", "SUB=expired_sub;SUBP=expired_subp"),
        ("validated_at", &expired_time.timestamp().to_string()),
    ]).await.unwrap();
}

/// 测试辅助: 清理cookies
async fn cleanup_cookies(redis: &RedisService, uid: &str) {
    let mut conn = redis.get_connection().await.unwrap();
    let _: () = conn.del(format!("weibo:cookies:{}", uid)).await.unwrap();
}

#[tokio::test]
async fn test_create_task_success() {
    let redis = setup_redis().await;
    let test_uid = "1234567890";

    // 准备: 保存有效cookies
    prepare_valid_cookies(&redis, test_uid).await;

    // 构造请求
    let keyword = "国庆";
    let event_start_time = "2025-10-01T00:00:00Z";

    // 模拟调用create_crawl_task命令
    // (实际实现中会通过Tauri command调用,这里直接调用业务逻辑)
    let task = CrawlTask::new(
        keyword.to_string(),
        event_start_time.parse().unwrap(),
    );

    // 保存任务到Redis
    redis.save_crawl_task(&task).await.unwrap();

    // 验证: 任务ID是有效UUID
    assert_eq!(task.id.len(), 36, "任务ID应为UUID v4格式");

    // 验证: 任务状态为Created
    assert_eq!(task.status, CrawlStatus::Created, "初始状态应为Created");

    // 验证: Redis中存在任务记录
    let loaded_task = redis.load_task(&task.id).await.unwrap();
    assert_eq!(loaded_task.id, task.id);
    assert_eq!(loaded_task.keyword, keyword);
    assert_eq!(loaded_task.status, CrawlStatus::Created);

    // 清理
    cleanup_task(&redis, &task.id).await;
    cleanup_cookies(&redis, test_uid).await;
}

#[tokio::test]
async fn test_create_task_invalid_keyword_empty() {
    // 边界测试: 关键字为空
    let empty_keywords = vec!["", "   ", "\t", "\n"];

    for keyword in empty_keywords {
        let task = CrawlTask::new(
            keyword.to_string(),
            Utc::now(),
        );

        // 验证应失败
        let result = task.validate();
        assert!(
            result.is_err(),
            "关键字为'{}'时应验证失败",
            keyword.escape_debug()
        );
        assert!(
            result.unwrap_err().contains("关键字不能为空"),
            "错误消息应包含'关键字不能为空'"
        );
    }
}

#[tokio::test]
async fn test_create_task_invalid_time_future() {
    // 边界测试: 事件开始时间是未来
    let future_time = Utc::now() + Duration::hours(1);

    let task = CrawlTask::new(
        "国庆".to_string(),
        future_time,
    );

    // 验证应失败
    let result = task.validate();
    assert!(result.is_err(), "未来时间应验证失败");
    assert!(
        result.unwrap_err().contains("不能是未来时间"),
        "错误消息应包含'不能是未来时间'"
    );
}

#[tokio::test]
async fn test_create_task_cookies_not_found() {
    let redis = setup_redis().await;
    let nonexistent_uid = "nonexistent_uid_999";

    // 验证: cookies不存在
    let result = redis.query_cookies(nonexistent_uid).await;
    assert!(result.is_err(), "不存在的UID应返回错误");

    // 错误码应为COOKIES_NOT_FOUND (由create_crawl_task命令处理)
    let error_message = result.unwrap_err().to_string();
    assert!(
        error_message.contains("未找到") || error_message.contains("不存在"),
        "错误消息应提示cookies不存在: {}",
        error_message
    );
}

#[tokio::test]
async fn test_create_task_cookies_expired() {
    let redis = setup_redis().await;
    let expired_uid = "expired_uid_123";

    // 准备: 保存过期cookies
    prepare_expired_cookies(&redis, expired_uid).await;

    // 验证: cookies存在但已过期
    let cookies_result = redis.query_cookies(expired_uid).await;

    if let Ok(cookies_data) = cookies_result {
        // 检查验证时间
        let validated_at = cookies_data.validated_at;
        let age = Utc::now() - validated_at;

        assert!(
            age.num_days() > 7,
            "cookies年龄应超过7天: 实际{}天",
            age.num_days()
        );

        // 实际命令应返回COOKIES_EXPIRED错误
        // (这里仅验证数据层,命令层会检查年龄)
    }

    // 清理
    cleanup_cookies(&redis, expired_uid).await;
}

#[tokio::test]
async fn test_create_task_multiple_calls_different_ids() {
    let redis = setup_redis().await;
    let test_uid = "multi_task_uid";

    // 准备
    prepare_valid_cookies(&redis, test_uid).await;

    // 创建3个任务
    let task1 = CrawlTask::new("关键字1".to_string(), Utc::now());
    let task2 = CrawlTask::new("关键字2".to_string(), Utc::now());
    let task3 = CrawlTask::new("关键字3".to_string(), Utc::now());

    // 验证: 每次调用生成不同的UUID
    assert_ne!(task1.id, task2.id, "任务ID应唯一");
    assert_ne!(task2.id, task3.id, "任务ID应唯一");
    assert_ne!(task1.id, task3.id, "任务ID应唯一");

    // 保存并验证
    redis.save_crawl_task(&task1).await.unwrap();
    redis.save_crawl_task(&task2).await.unwrap();
    redis.save_crawl_task(&task3).await.unwrap();

    // 验证: 所有任务都能独立加载
    let loaded1 = redis.load_task(&task1.id).await.unwrap();
    let loaded2 = redis.load_task(&task2.id).await.unwrap();
    let loaded3 = redis.load_task(&task3.id).await.unwrap();

    assert_eq!(loaded1.keyword, "关键字1");
    assert_eq!(loaded2.keyword, "关键字2");
    assert_eq!(loaded3.keyword, "关键字3");

    // 清理
    cleanup_task(&redis, &task1.id).await;
    cleanup_task(&redis, &task2.id).await;
    cleanup_task(&redis, &task3.id).await;
    cleanup_cookies(&redis, test_uid).await;
}

#[tokio::test]
async fn test_create_task_redis_persistence() {
    let redis = setup_redis().await;
    let test_uid = "persistence_test_uid";

    // 准备
    prepare_valid_cookies(&redis, test_uid).await;

    // 创建任务
    let task = CrawlTask::new(
        "测试持久化".to_string(),
        "2025-10-01T00:00:00Z".parse().unwrap(),
    );

    redis.save_crawl_task(&task).await.unwrap();

    // 验证: Redis Hash字段完整
    let mut conn = redis.get_connection().await.unwrap();
    let key = format!("crawl:task:{}", task.id);

    let exists: bool = conn.exists(&key).await.unwrap();
    assert!(exists, "Redis中应存在任务记录");

    let keyword: String = conn.hget(&key, "keyword").await.unwrap();
    assert_eq!(keyword, "测试持久化");

    let status: String = conn.hget(&key, "status").await.unwrap();
    assert_eq!(status, "Created");

    let crawled_count: u64 = conn.hget(&key, "crawled_count").await.unwrap();
    assert_eq!(crawled_count, 0, "初始爬取数量应为0");

    // 清理
    cleanup_task(&redis, &task.id).await;
    cleanup_cookies(&redis, test_uid).await;
}

/// 性能测试: 验证任务创建和加载的响应时间
#[tokio::test]
async fn test_create_task_performance() {
    let redis = setup_redis().await;
    let test_uid = "perf_test_uid";

    prepare_valid_cookies(&redis, test_uid).await;

    let start = std::time::Instant::now();

    // 创建10个任务
    let mut task_ids = Vec::new();
    for i in 0..10 {
        let task = CrawlTask::new(
            format!("性能测试{}", i),
            Utc::now() - Duration::days(1),
        );
        redis.save_crawl_task(&task).await.unwrap();
        task_ids.push(task.id);
    }

    let elapsed = start.elapsed();

    // 验证: 10个任务创建应在1秒内完成
    assert!(
        elapsed.as_millis() < 1000,
        "创建10个任务应在1秒内完成: 实际{}ms",
        elapsed.as_millis()
    );

    // 验证: 批量加载
    let load_start = std::time::Instant::now();
    for task_id in &task_ids {
        redis.load_task(task_id).await.unwrap();
    }
    let load_elapsed = load_start.elapsed();

    // 验证: 10个任务加载应在500ms内完成
    assert!(
        load_elapsed.as_millis() < 500,
        "加载10个任务应在500ms内完成: 实际{}ms",
        load_elapsed.as_millis()
    );

    // 清理
    for task_id in &task_ids {
        cleanup_task(&redis, task_id).await;
    }
    cleanup_cookies(&redis, test_uid).await;
}
