//! T012 - 集成测试: 场景3暂停并恢复任务
//!
//! 覆盖:
//! - FR-004: 暂停/恢复
//! - FR-013至FR-015: 断点续爬
//!
//! 验证点:
//! 1. 暂停后状态转换到Paused
//! 2. 检查点保存当前页码
//! 3. 恢复后从下一页继续
//! 4. 无重复数据

use chrono::{DateTime, Utc};
use redis::AsyncCommands;
use weibo_login::models::{CrawlCheckpoint, CrawlDirection, CrawlStatus, CrawlTask};

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

/// 创建测试任务
async fn create_test_task(ctx: &TestContext) -> CrawlTask {
    let event_start_time = Utc::now() - chrono::Duration::hours(24);
    CrawlTask::new("国庆".to_string(), event_start_time)
}

/// 模拟任务启动并爬取到第15页
async fn simulate_crawl_to_page_15(ctx: &TestContext, task: &mut CrawlTask) {
    let mut conn = ctx
        .redis_client
        .get_multiplexed_async_connection()
        .await
        .expect("Redis连接失败");

    // 转换状态到HistoryCrawling
    task.transition_to(CrawlStatus::HistoryCrawling)
        .expect("状态转换失败");

    // 创建检查点
    let shard_start = task.event_start_time;
    let shard_end = Utc::now();
    let mut checkpoint = CrawlCheckpoint::new_backward(ctx.task_id.clone(), shard_start, shard_end);

    // 模拟爬取15页
    for page in 1..=15 {
        checkpoint.current_page = page;

        // 模拟每页20条帖子
        for post_idx in 1..=20 {
            let post_id = format!("post_{}_{}", page, post_idx);
            let post_time = Utc::now() - chrono::Duration::hours((page * 2) as i64);

            // 添加到去重集合
            let _: () = conn
                .sadd(format!("crawl:post_ids:{}", ctx.task_id), &post_id)
                .await
                .expect("Redis SADD失败");

            // 更新任务进度
            task.update_progress(post_time, 1);
        }

        checkpoint.saved_at = Utc::now();
    }

    // 保存任务状态
    save_task_to_redis(&ctx.redis_client, task).await;

    // 保存检查点
    save_checkpoint_to_redis(&ctx.redis_client, &checkpoint).await;
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

/// 从Redis加载任务
async fn load_task_from_redis(client: &redis::Client, task_id: &str) -> CrawlTask {
    let mut conn = client
        .get_multiplexed_async_connection()
        .await
        .expect("Redis连接失败");

    let task_key = format!("crawl:task:{}", task_id);
    let fields: Vec<String> = conn.hgetall(&task_key).await.expect("Redis加载任务失败");

    // 解析Redis Hash (key1, value1, key2, value2, ...)
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
async fn test_pause_saves_checkpoint() {
    let ctx = TestContext::new().await;

    // 创建并启动任务,爬取到第15页
    let mut task = create_test_task(&ctx).await;
    task.id = ctx.task_id.clone();
    simulate_crawl_to_page_15(&ctx, &mut task).await;

    // 暂停任务
    task.transition_to(CrawlStatus::Paused)
        .expect("暂停状态转换失败");
    save_task_to_redis(&ctx.redis_client, &task).await;

    // 验证: 任务状态为Paused
    let loaded_task = load_task_from_redis(&ctx.redis_client, &ctx.task_id).await;
    assert_eq!(loaded_task.status, CrawlStatus::Paused);

    // 验证: 检查点保存当前页码15
    let checkpoint = load_checkpoint_from_redis(&ctx.redis_client, &ctx.task_id).await;
    assert_eq!(checkpoint.current_page, 15);

    ctx.cleanup().await;
}

#[tokio::test]
async fn test_resume_continues_from_next_page() {
    let ctx = TestContext::new().await;

    // 创建并启动任务,爬取到第15页并暂停
    let mut task = create_test_task(&ctx).await;
    task.id = ctx.task_id.clone();
    simulate_crawl_to_page_15(&ctx, &mut task).await;

    task.transition_to(CrawlStatus::Paused)
        .expect("暂停状态转换失败");
    save_task_to_redis(&ctx.redis_client, &task).await;

    // 恢复任务
    task.transition_to(CrawlStatus::HistoryCrawling)
        .expect("恢复状态转换失败");

    // 从检查点继续
    let mut checkpoint = load_checkpoint_from_redis(&ctx.redis_client, &ctx.task_id).await;
    let pause_page = checkpoint.current_page;

    // 验证: 应从第16页继续
    checkpoint.advance_page();
    assert_eq!(checkpoint.current_page, pause_page + 1);

    ctx.cleanup().await;
}

#[tokio::test]
async fn test_resume_no_duplicate_data() {
    let ctx = TestContext::new().await;

    // 创建并启动任务,爬取到第15页
    let mut task = create_test_task(&ctx).await;
    task.id = ctx.task_id.clone();
    simulate_crawl_to_page_15(&ctx, &mut task).await;

    let mut conn = ctx
        .redis_client
        .get_multiplexed_async_connection()
        .await
        .expect("Redis连接失败");

    // 验证: 已爬取300条帖子(15页 * 20条)
    let count_before_pause: u64 = conn
        .scard(format!("crawl:post_ids:{}", ctx.task_id))
        .await
        .expect("Redis SCARD失败");
    assert_eq!(count_before_pause, 300);

    // 暂停并恢复
    task.transition_to(CrawlStatus::Paused)
        .expect("暂停状态转换失败");
    task.transition_to(CrawlStatus::HistoryCrawling)
        .expect("恢复状态转换失败");

    // 继续爬取第16-17页
    let checkpoint = load_checkpoint_from_redis(&ctx.redis_client, &ctx.task_id).await;
    for page in 16..=17 {
        for post_idx in 1..=20 {
            let post_id = format!("post_{}_{}", page, post_idx);
            let _: () = conn
                .sadd(format!("crawl:post_ids:{}", ctx.task_id), &post_id)
                .await
                .expect("Redis SADD失败");
        }
    }

    // 验证: 总共340条帖子(无重复)
    let count_after_resume: u64 = conn
        .scard(format!("crawl:post_ids:{}", ctx.task_id))
        .await
        .expect("Redis SCARD失败");
    assert_eq!(count_after_resume, 340);

    ctx.cleanup().await;
}

#[tokio::test]
async fn test_pause_at_first_page() {
    let ctx = TestContext::new().await;

    // 创建任务并只爬取第1页
    let mut task = create_test_task(&ctx).await;
    task.id = ctx.task_id.clone();
    task.transition_to(CrawlStatus::HistoryCrawling)
        .expect("状态转换失败");

    let checkpoint = CrawlCheckpoint::new_backward(
        ctx.task_id.clone(),
        task.event_start_time,
        Utc::now(),
    );
    save_checkpoint_to_redis(&ctx.redis_client, &checkpoint).await;

    // 在第1页暂停
    task.transition_to(CrawlStatus::Paused)
        .expect("暂停状态转换失败");
    save_task_to_redis(&ctx.redis_client, &task).await;

    // 恢复
    task.transition_to(CrawlStatus::HistoryCrawling)
        .expect("恢复状态转换失败");

    // 验证: 从第2页开始
    let mut checkpoint = load_checkpoint_from_redis(&ctx.redis_client, &ctx.task_id).await;
    checkpoint.advance_page();
    assert_eq!(checkpoint.current_page, 2);

    ctx.cleanup().await;
}

#[tokio::test]
async fn test_multiple_pause_resume_cycles() {
    let ctx = TestContext::new().await;

    let mut task = create_test_task(&ctx).await;
    task.id = ctx.task_id.clone();
    task.transition_to(CrawlStatus::HistoryCrawling)
        .expect("状态转换失败");

    let mut checkpoint = CrawlCheckpoint::new_backward(
        ctx.task_id.clone(),
        task.event_start_time,
        Utc::now(),
    );

    // 第一次: 爬取到第5页,暂停
    for _ in 1..=5 {
        checkpoint.advance_page();
    }
    assert_eq!(checkpoint.current_page, 6);

    task.transition_to(CrawlStatus::Paused)
        .expect("第一次暂停失败");
    task.transition_to(CrawlStatus::HistoryCrawling)
        .expect("第一次恢复失败");

    // 第二次: 继续到第10页,暂停
    for _ in 6..=10 {
        checkpoint.advance_page();
    }
    assert_eq!(checkpoint.current_page, 11);

    task.transition_to(CrawlStatus::Paused)
        .expect("第二次暂停失败");
    task.transition_to(CrawlStatus::HistoryCrawling)
        .expect("第二次恢复失败");

    // 第三次: 继续到第15页
    for _ in 11..=15 {
        checkpoint.advance_page();
    }
    assert_eq!(checkpoint.current_page, 16);

    ctx.cleanup().await;
}
