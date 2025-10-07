//! T014 - 集成测试: 场景5增量更新
//!
//! 覆盖:
//! - FR-010至FR-012: 增量更新
//!
//! 验证点:
//! 1. 状态转换: HistoryCompleted → IncrementalCrawling
//! 2. 爬取方向为Forward
//! 3. 仅爬取max_post_time之后的帖子
//! 4. 增量数据正确追加到已有数据集

use chrono::{DateTime, Duration, Utc};
use redis::AsyncCommands;
use weibo_login::models::{CrawlCheckpoint, CrawlDirection, CrawlStatus, CrawlTask, WeiboPost};

/// 测试上下文
struct TestContext {
    redis_client: redis::Client,
    task_id: String,
}

impl TestContext {
    async fn new() -> Self {
        let redis_client =
            redis::Client::open("redis://127.0.0.1:6379").expect("Redis连接失败");

        Self {
            redis_client,
            task_id: uuid::Uuid::new_v4().to_string(),
        }
    }

    async fn cleanup(&self) {
        let mut conn = self.redis_client.get_multiplexed_async_connection().await.ok();
        if let Some(ref mut c) = conn {
            let _: Result<(), redis::RedisError> = redis::cmd("DEL")
                .arg(format!("crawl:task:{}", self.task_id))
                .arg(format!("crawl:checkpoint:{}", self.task_id))
                .arg(format!("crawl:posts:{}", self.task_id))
                .arg(format!("crawl:post_ids:{}", self.task_id))
                .query_async(c)
                .await;
        }
    }
}

/// 创建历史回溯完成的任务
async fn create_history_completed_task(ctx: &TestContext) -> CrawlTask {
    let event_start_time = Utc::now() - Duration::hours(72);
    let mut task = CrawlTask::new("国庆".to_string(), event_start_time);
    task.id = ctx.task_id.clone();

    // 模拟历史回溯已完成
    task.status = CrawlStatus::HistoryCompleted;

    // 设置时间范围: 72小时前到24小时前
    let min_time = event_start_time;
    let max_time = Utc::now() - Duration::hours(24);
    task.min_post_time = Some(min_time);
    task.max_post_time = Some(max_time);
    task.crawled_count = 500;

    task
}

/// 模拟历史帖子数据
async fn simulate_history_posts(ctx: &TestContext, task: &CrawlTask) {
    let mut conn = ctx
        .redis_client
        .get_multiplexed_async_connection()
        .await
        .expect("Redis连接失败");

    let max_time = task.max_post_time.unwrap();

    // 添加50条历史帖子(时间戳均小于max_post_time)
    for i in 1..=50 {
        let post_id = format!("history_post_{}", i);
        let post_time = max_time - Duration::hours(i);

        let post = WeiboPost::new(
            post_id.clone(),
            ctx.task_id.clone(),
            format!("历史帖子内容{}", i),
            post_time,
            format!("author_{}", i),
            format!("作者{}", i),
            10,
            5,
            20,
        );

        // 添加到Sorted Set
        let post_json = serde_json::to_string(&post).unwrap();
        let _: () = conn
            .zadd(
                format!("crawl:posts:{}", ctx.task_id),
                post_json,
                post_time.timestamp(),
            )
            .await
            .expect("Redis ZADD失败");

        // 添加到去重集合
        let _: () = conn
            .sadd(format!("crawl:post_ids:{}", ctx.task_id), &post_id)
            .await
            .expect("Redis SADD失败");
    }
}

/// 保存任务到Redis
async fn save_task_to_redis(client: &redis::Client, task: &CrawlTask) {
    let mut conn = client
        .get_multiplexed_async_connection()
        .await
        .expect("Redis连接失败");

    let task_key = format!("crawl:task:{}", task.id);
    let _: () = conn
        .hset_multiple(
            &task_key,
            &[
                ("id", task.id.as_str()),
                ("keyword", task.keyword.as_str()),
                (
                    "event_start_time",
                    &task.event_start_time.timestamp().to_string(),
                ),
                ("status", serde_json::to_string(&task.status).unwrap().as_str()),
                (
                    "min_post_time",
                    &task
                        .min_post_time
                        .map(|t| t.timestamp().to_string())
                        .unwrap_or_default(),
                ),
                (
                    "max_post_time",
                    &task
                        .max_post_time
                        .map(|t| t.timestamp().to_string())
                        .unwrap_or_default(),
                ),
                ("crawled_count", &task.crawled_count.to_string()),
                ("created_at", &task.created_at.timestamp().to_string()),
                ("updated_at", &task.updated_at.timestamp().to_string()),
            ],
        )
        .await
        .expect("Redis保存任务失败");
}

/// 从Redis加载任务
async fn load_task_from_redis(client: &redis::Client, task_id: &str) -> CrawlTask {
    let mut conn = client
        .get_multiplexed_async_connection()
        .await
        .expect("Redis连接失败");

    let task_key = format!("crawl:task:{}", task_id);
    let fields: Vec<String> = conn.hgetall(&task_key).await.expect("Redis加载任务失败");

    let mut map = std::collections::HashMap::new();
    for i in (0..fields.len()).step_by(2) {
        map.insert(fields[i].clone(), fields[i + 1].clone());
    }

    CrawlTask {
        id: map.get("id").unwrap().clone(),
        keyword: map.get("keyword").unwrap().clone(),
        event_start_time: DateTime::from_timestamp(
            map.get("event_start_time").unwrap().parse().unwrap(),
            0,
        )
        .unwrap(),
        status: serde_json::from_str(map.get("status").unwrap()).unwrap(),
        min_post_time: map
            .get("min_post_time")
            .and_then(|s| s.parse::<i64>().ok())
            .and_then(|ts| DateTime::from_timestamp(ts, 0)),
        max_post_time: map
            .get("max_post_time")
            .and_then(|s| s.parse::<i64>().ok())
            .and_then(|ts| DateTime::from_timestamp(ts, 0)),
        crawled_count: map.get("crawled_count").unwrap().parse().unwrap(),
        created_at: DateTime::from_timestamp(map.get("created_at").unwrap().parse().unwrap(), 0)
            .unwrap(),
        updated_at: DateTime::from_timestamp(map.get("updated_at").unwrap().parse().unwrap(), 0)
            .unwrap(),
        failure_reason: None,
    }
}

/// 保存检查点到Redis
async fn save_checkpoint_to_redis(client: &redis::Client, checkpoint: &CrawlCheckpoint) {
    let mut conn = client
        .get_multiplexed_async_connection()
        .await
        .expect("Redis连接失败");

    let checkpoint_key = format!("crawl:checkpoint:{}", checkpoint.task_id);
    let _: () = conn
        .hset_multiple(
            &checkpoint_key,
            &[
                ("task_id", checkpoint.task_id.as_str()),
                (
                    "shard_start_time",
                    &checkpoint.shard_start_time.timestamp().to_string(),
                ),
                (
                    "shard_end_time",
                    &checkpoint.shard_end_time.timestamp().to_string(),
                ),
                ("current_page", &checkpoint.current_page.to_string()),
                (
                    "direction",
                    serde_json::to_string(&checkpoint.direction)
                        .unwrap()
                        .as_str(),
                ),
                ("saved_at", &checkpoint.saved_at.timestamp().to_string()),
            ],
        )
        .await
        .expect("Redis保存检查点失败");
}

/// 从Redis加载检查点
async fn load_checkpoint_from_redis(
    client: &redis::Client,
    task_id: &str,
) -> CrawlCheckpoint {
    let mut conn = client
        .get_multiplexed_async_connection()
        .await
        .expect("Redis连接失败");

    let checkpoint_key = format!("crawl:checkpoint:{}", task_id);
    let fields: Vec<String> = conn
        .hgetall(&checkpoint_key)
        .await
        .expect("Redis加载检查点失败");

    let mut map = std::collections::HashMap::new();
    for i in (0..fields.len()).step_by(2) {
        map.insert(fields[i].clone(), fields[i + 1].clone());
    }

    CrawlCheckpoint {
        task_id: map.get("task_id").unwrap().clone(),
        shard_start_time: DateTime::from_timestamp(
            map.get("shard_start_time").unwrap().parse().unwrap(),
            0,
        )
        .unwrap(),
        shard_end_time: DateTime::from_timestamp(
            map.get("shard_end_time").unwrap().parse().unwrap(),
            0,
        )
        .unwrap(),
        current_page: map.get("current_page").unwrap().parse().unwrap(),
        direction: serde_json::from_str(map.get("direction").unwrap()).unwrap(),
        completed_shards: vec![],
        saved_at: DateTime::from_timestamp(map.get("saved_at").unwrap().parse().unwrap(), 0)
            .unwrap(),
    }
}

#[tokio::test]
async fn test_transition_to_incremental_crawling() {
    let ctx = TestContext::new().await;

    // 创建历史回溯完成的任务
    let mut task = create_history_completed_task(&ctx).await;
    save_task_to_redis(&ctx.redis_client, &task).await;
    simulate_history_posts(&ctx, &task).await;

    // 启动增量更新
    task.transition_to(CrawlStatus::IncrementalCrawling)
        .expect("状态转换失败");
    save_task_to_redis(&ctx.redis_client, &task).await;

    // 验证: 状态转换成功
    let loaded_task = load_task_from_redis(&ctx.redis_client, &ctx.task_id).await;
    assert_eq!(
        loaded_task.status,
        CrawlStatus::IncrementalCrawling,
        "任务状态应为IncrementalCrawling"
    );

    ctx.cleanup().await;
}

#[tokio::test]
async fn test_incremental_crawl_direction_forward() {
    let ctx = TestContext::new().await;

    // 创建历史回溯完成的任务
    let mut task = create_history_completed_task(&ctx).await;
    save_task_to_redis(&ctx.redis_client, &task).await;

    // 转换到增量爬取
    task.transition_to(CrawlStatus::IncrementalCrawling)
        .expect("状态转换失败");

    // 创建增量检查点
    let max_post_time = task.max_post_time.unwrap();
    let checkpoint = CrawlCheckpoint::new_forward(ctx.task_id.clone(), max_post_time);

    save_checkpoint_to_redis(&ctx.redis_client, &checkpoint).await;

    // 验证: 爬取方向为Forward
    let loaded_checkpoint = load_checkpoint_from_redis(&ctx.redis_client, &ctx.task_id).await;
    assert_eq!(
        loaded_checkpoint.direction,
        CrawlDirection::Forward,
        "增量爬取方向应为Forward"
    );

    // 验证: 起始时间为max_post_time
    assert_eq!(
        loaded_checkpoint.shard_start_time.timestamp(),
        max_post_time.timestamp(),
        "增量爬取起始时间应为max_post_time"
    );

    ctx.cleanup().await;
}

#[tokio::test]
async fn test_incremental_only_crawls_new_posts() {
    let ctx = TestContext::new().await;

    // 创建历史回溯完成的任务
    let mut task = create_history_completed_task(&ctx).await;
    save_task_to_redis(&ctx.redis_client, &task).await;
    simulate_history_posts(&ctx, &task).await;

    let max_post_time = task.max_post_time.unwrap();

    // 转换到增量爬取
    task.transition_to(CrawlStatus::IncrementalCrawling)
        .expect("状态转换失败");

    // 模拟增量爬取: 添加10条新帖子(时间戳均大于max_post_time)
    let mut conn = ctx
        .redis_client
        .get_multiplexed_async_connection()
        .await
        .expect("Redis连接失败");

    for i in 1..=10 {
        let post_id = format!("incremental_post_{}", i);
        let post_time = max_post_time + Duration::hours(i);

        let post = WeiboPost::new(
            post_id.clone(),
            ctx.task_id.clone(),
            format!("增量帖子内容{}", i),
            post_time,
            format!("author_new_{}", i),
            format!("新作者{}", i),
            5,
            3,
            10,
        );

        let post_json = serde_json::to_string(&post).unwrap();
        let _: () = conn
            .zadd(
                format!("crawl:posts:{}", ctx.task_id),
                post_json,
                post_time.timestamp(),
            )
            .await
            .expect("Redis ZADD失败");

        let _: () = conn
            .sadd(format!("crawl:post_ids:{}", ctx.task_id), &post_id)
            .await
            .expect("Redis SADD失败");

        task.update_progress(post_time, 1);
    }

    save_task_to_redis(&ctx.redis_client, &task).await;

    // 验证: 总帖子数 = 历史50条 + 增量10条
    let total_count: u64 = conn
        .scard(format!("crawl:post_ids:{}", ctx.task_id))
        .await
        .expect("Redis SCARD失败");
    assert_eq!(total_count, 60, "总帖子数应为60");

    // 验证: 最新帖子时间戳大于max_post_time
    let latest_posts: Vec<(String, i64)> = conn
        .zrevrange_withscores(format!("crawl:posts:{}", ctx.task_id), 0, 9)
        .await
        .expect("Redis ZREVRANGE失败");

    for (_post_json, timestamp) in latest_posts {
        assert!(
            timestamp > max_post_time.timestamp(),
            "增量帖子时间戳{}应大于max_post_time{}",
            timestamp,
            max_post_time.timestamp()
        );
    }

    // 验证: 任务的max_post_time已更新
    let updated_task = load_task_from_redis(&ctx.redis_client, &ctx.task_id).await;
    assert!(
        updated_task.max_post_time.unwrap() > max_post_time,
        "任务max_post_time应已更新"
    );

    ctx.cleanup().await;
}

#[tokio::test]
async fn test_incremental_no_new_posts() {
    let ctx = TestContext::new().await;

    // 创建历史回溯完成的任务
    let mut task = create_history_completed_task(&ctx).await;
    save_task_to_redis(&ctx.redis_client, &task).await;
    simulate_history_posts(&ctx, &task).await;

    let max_post_time = task.max_post_time.unwrap();

    // 转换到增量爬取
    task.transition_to(CrawlStatus::IncrementalCrawling)
        .expect("状态转换失败");

    let checkpoint = CrawlCheckpoint::new_forward(ctx.task_id.clone(), max_post_time);
    save_checkpoint_to_redis(&ctx.redis_client, &checkpoint).await;

    // 模拟增量爬取: 无新帖子

    // 验证: 检查点方向仍为Forward
    let loaded_checkpoint = load_checkpoint_from_redis(&ctx.redis_client, &ctx.task_id).await;
    assert_eq!(loaded_checkpoint.direction, CrawlDirection::Forward);

    // 验证: 任务状态正常
    let loaded_task = load_task_from_redis(&ctx.redis_client, &ctx.task_id).await;
    assert_eq!(loaded_task.status, CrawlStatus::IncrementalCrawling);

    // 验证: 帖子总数未变
    let mut conn = ctx
        .redis_client
        .get_multiplexed_async_connection()
        .await
        .expect("Redis连接失败");

    let total_count: u64 = conn
        .scard(format!("crawl:post_ids:{}", ctx.task_id))
        .await
        .expect("Redis SCARD失败");
    assert_eq!(total_count, 50, "无新帖子时总数应保持不变");

    ctx.cleanup().await;
}

#[tokio::test]
async fn test_incremental_preserves_history_data() {
    let ctx = TestContext::new().await;

    // 创建历史回溯完成的任务
    let mut task = create_history_completed_task(&ctx).await;
    save_task_to_redis(&ctx.redis_client, &task).await;
    simulate_history_posts(&ctx, &task).await;

    let max_post_time = task.max_post_time.unwrap();
    let original_crawled_count = task.crawled_count;

    // 转换到增量爬取
    task.transition_to(CrawlStatus::IncrementalCrawling)
        .expect("状态转换失败");

    // 添加5条增量帖子
    let mut conn = ctx
        .redis_client
        .get_multiplexed_async_connection()
        .await
        .expect("Redis连接失败");

    for i in 1..=5 {
        let post_id = format!("new_post_{}", i);
        let post_time = max_post_time + Duration::hours(i);

        let post = WeiboPost::new(
            post_id.clone(),
            ctx.task_id.clone(),
            "增量内容".to_string(),
            post_time,
            "author_new".to_string(),
            "新作者".to_string(),
            1,
            1,
            1,
        );

        let post_json = serde_json::to_string(&post).unwrap();
        let _: () = conn
            .zadd(
                format!("crawl:posts:{}", ctx.task_id),
                post_json,
                post_time.timestamp(),
            )
            .await
            .expect("Redis ZADD失败");

        let _: () = conn
            .sadd(format!("crawl:post_ids:{}", ctx.task_id), &post_id)
            .await
            .expect("Redis SADD失败");
    }

    // 验证: 历史数据仍然存在
    let history_count: u64 = conn
        .zcount(
            format!("crawl:posts:{}", ctx.task_id),
            "-inf",
            max_post_time.timestamp(),
        )
        .await
        .expect("Redis ZCOUNT失败");

    assert_eq!(history_count, 50, "历史帖子数应保持不变");

    // 验证: 总数正确
    let total_count: u64 = conn
        .scard(format!("crawl:post_ids:{}", ctx.task_id))
        .await
        .expect("Redis SCARD失败");
    assert_eq!(total_count, 55, "总帖子数 = 历史50 + 增量5");

    ctx.cleanup().await;
}

#[tokio::test]
async fn test_incremental_updates_max_post_time() {
    let ctx = TestContext::new().await;

    // 创建历史回溯完成的任务
    let mut task = create_history_completed_task(&ctx).await;
    let original_max_time = task.max_post_time.unwrap();
    save_task_to_redis(&ctx.redis_client, &task).await;

    // 转换到增量爬取
    task.transition_to(CrawlStatus::IncrementalCrawling)
        .expect("状态转换失败");

    // 添加新帖子并更新进度
    let new_post_time = original_max_time + Duration::hours(12);
    task.update_progress(new_post_time, 1);
    save_task_to_redis(&ctx.redis_client, &task).await;

    // 验证: max_post_time已更新
    let updated_task = load_task_from_redis(&ctx.redis_client, &ctx.task_id).await;
    assert!(
        updated_task.max_post_time.unwrap() > original_max_time,
        "max_post_time应已更新为更晚的时间"
    );

    // 验证: min_post_time保持不变(历史数据下界)
    assert_eq!(
        updated_task.min_post_time,
        task.min_post_time,
        "min_post_time应保持不变"
    );

    ctx.cleanup().await;
}
