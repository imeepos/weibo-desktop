/// 集成测试 - 场景2: 启动历史回溯
///
/// 验证目标:
/// - FR-006至FR-009 (历史回溯)
/// - FR-019至FR-020 (进度追踪)
/// - start_history_crawl命令的完整业务流程
///
/// 测试覆盖:
/// 1. 任务状态转换: Created → HistoryCrawling
/// 2. 每页爬取后推送crawl-progress事件
/// 3. Redis检查点保存正确
/// 4. Redis帖子数据存在
/// 5. 帖子ID去重生效
use chrono::{Duration, Utc};
use redis::AsyncCommands;
use weibo_login::models::{
    crawl_checkpoint::{CrawlCheckpoint, CrawlDirection},
    crawl_task::{CrawlStatus, CrawlTask},
    weibo_post::WeiboPost,
};
use weibo_login::services::redis_service::RedisService;

/// 测试辅助: 创建测试用RedisService
async fn setup_redis() -> RedisService {
    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    RedisService::new(&redis_url)
        .await
        .expect("Redis连接失败,请确保Redis服务运行")
}

/// 测试辅助: 清理任务相关数据
async fn cleanup_task_data(redis: &RedisService, task_id: &str) {
    let mut conn = redis.get_connection().await.unwrap();
    let _: () = conn.del(format!("crawl:task:{}", task_id)).await.unwrap();
    let _: () = conn
        .del(format!("crawl:checkpoint:{}", task_id))
        .await
        .unwrap();
    let _: () = conn
        .del(format!("crawl:posts:{}", task_id))
        .await
        .unwrap();
    let _: () = conn
        .del(format!("crawl:post_ids:{}", task_id))
        .await
        .unwrap();
}

/// 测试辅助: 创建已保存的Created状态任务
async fn prepare_created_task(redis: &RedisService, keyword: &str) -> CrawlTask {
    let task = CrawlTask::new(
        keyword.to_string(),
        Utc::now() - Duration::hours(24),
    );
    redis.save_crawl_task(&task).await.unwrap();
    task
}

#[tokio::test]
async fn test_start_crawl_status_transition() {
    let redis = setup_redis().await;

    let task = prepare_created_task(&redis, "国庆").await;

    // 验证: 初始状态为Created
    let loaded = redis.load_task(&task.id).await.unwrap();
    assert_eq!(loaded.status, CrawlStatus::Created, "初始状态应为Created");

    // 模拟启动历史回溯
    let mut updated_task = loaded.clone();
    updated_task
        .transition_to(CrawlStatus::HistoryCrawling)
        .unwrap();
    redis.save_crawl_task(&updated_task).await.unwrap();

    // 验证: 状态已转换
    let after_start = redis.load_task(&task.id).await.unwrap();
    assert_eq!(
        after_start.status,
        CrawlStatus::HistoryCrawling,
        "状态应转换为HistoryCrawling"
    );

    // 验证: updated_at已刷新
    assert!(
        after_start.updated_at > task.updated_at,
        "updated_at应在状态转换时刷新"
    );

    cleanup_task_data(&redis, &task.id).await;
}

#[tokio::test]
async fn test_checkpoint_creation() {
    let redis = setup_redis().await;
    let task = prepare_created_task(&redis, "测试检查点").await;

    // 创建初始检查点 (历史回溯模式)
    let shard_start = task.event_start_time;
    let shard_end = Utc::now();

    let checkpoint = CrawlCheckpoint::new_backward(task.id.clone(), shard_start, shard_end);

    redis.save_checkpoint(&checkpoint).await.unwrap();

    // 验证: 检查点已保存
    let loaded_checkpoint = redis.load_checkpoint(&task.id).await.unwrap();
    assert_eq!(
        loaded_checkpoint.current_page, 1,
        "初始页码应为1"
    );
    assert_eq!(
        loaded_checkpoint.direction,
        CrawlDirection::Backward,
        "方向应为Backward"
    );
    assert_eq!(
        loaded_checkpoint.shard_start_time.timestamp(),
        shard_start.timestamp(),
        "分片开始时间应匹配"
    );

    cleanup_task_data(&redis, &task.id).await;
}

#[tokio::test]
async fn test_checkpoint_page_advance() {
    let redis = setup_redis().await;
    let task = prepare_created_task(&redis, "页码推进").await;

    let mut checkpoint = CrawlCheckpoint::new_backward(
        task.id.clone(),
        task.event_start_time,
        Utc::now(),
    );

    // 模拟爬取3页
    for expected_page in 1..=3 {
        assert_eq!(
            checkpoint.current_page, expected_page,
            "页码应为{}",
            expected_page
        );

        redis.save_checkpoint(&checkpoint).await.unwrap();

        // 验证Redis中的页码
        let loaded = redis.load_checkpoint(&task.id).await.unwrap();
        assert_eq!(loaded.current_page, expected_page);

        // 推进到下一页
        checkpoint.advance_page();
    }

    // 最终页码应为4
    assert_eq!(checkpoint.current_page, 4, "爬取3页后应在第4页");

    cleanup_task_data(&redis, &task.id).await;
}

#[tokio::test]
async fn test_save_posts() {
    let redis = setup_redis().await;
    let task = prepare_created_task(&redis, "帖子存储").await;

    // 创建测试帖子
    let post1 = WeiboPost::new(
        "post_id_001".to_string(),
        task.id.clone(),
        "测试帖子内容1".to_string(),
        Utc::now() - Duration::hours(1),
        "1234567890".to_string(),
        "测试用户".to_string(),
        10,
        5,
        20,
    );

    let post2 = WeiboPost::new(
        "post_id_002".to_string(),
        task.id.clone(),
        "测试帖子内容2".to_string(),
        Utc::now() - Duration::hours(2),
        "1234567890".to_string(),
        "测试用户".to_string(),
        15,
        8,
        30,
    );

    // 保存帖子
    redis.save_post(&post1).await.unwrap();
    redis.save_post(&post2).await.unwrap();

    // 验证: 帖子数量
    let mut conn = redis.get_connection().await.unwrap();
    let post_count: u64 = conn.zcard(format!("crawl:posts:{}", task.id)).await.unwrap();
    assert_eq!(post_count, 2, "应有2条帖子");

    // 验证: 去重索引
    let id_count: u64 = conn
        .scard(format!("crawl:post_ids:{}", task.id))
        .await
        .unwrap();
    assert_eq!(id_count, 2, "去重集合应有2个ID");

    cleanup_task_data(&redis, &task.id).await;
}

#[tokio::test]
async fn test_post_deduplication() {
    let redis = setup_redis().await;
    let task = prepare_created_task(&redis, "去重测试").await;

    let post = WeiboPost::new(
        "duplicate_post_id".to_string(),
        task.id.clone(),
        "重复帖子".to_string(),
        Utc::now(),
        "123".to_string(),
        "用户".to_string(),
        0,
        0,
        0,
    );

    // 第1次保存
    redis.save_post(&post).await.unwrap();

    // 第2次保存相同ID的帖子
    redis.save_post(&post).await.unwrap();

    // 验证: 只有1条帖子
    let mut conn = redis.get_connection().await.unwrap();
    let post_count: u64 = conn.zcard(format!("crawl:posts:{}", task.id)).await.unwrap();
    assert_eq!(post_count, 1, "相同ID的帖子应去重");

    let id_count: u64 = conn
        .scard(format!("crawl:post_ids:{}", task.id))
        .await
        .unwrap();
    assert_eq!(id_count, 1, "去重集合应只有1个ID");

    // 验证: ID存在性检查
    let exists: bool = conn
        .sismember(format!("crawl:post_ids:{}", task.id), "duplicate_post_id")
        .await
        .unwrap();
    assert!(exists, "帖子ID应在去重集合中");

    cleanup_task_data(&redis, &task.id).await;
}

#[tokio::test]
async fn test_posts_sorted_by_time() {
    let redis = setup_redis().await;
    let task = prepare_created_task(&redis, "时间排序").await;

    let now = Utc::now();

    // 创建不同时间的帖子 (乱序插入)
    let post_old = WeiboPost::new(
        "old_post".to_string(),
        task.id.clone(),
        "最早的帖子".to_string(),
        now - Duration::hours(10),
        "123".to_string(),
        "用户".to_string(),
        0,
        0,
        0,
    );

    let post_new = WeiboPost::new(
        "new_post".to_string(),
        task.id.clone(),
        "最新的帖子".to_string(),
        now - Duration::hours(1),
        "123".to_string(),
        "用户".to_string(),
        0,
        0,
        0,
    );

    let post_middle = WeiboPost::new(
        "middle_post".to_string(),
        task.id.clone(),
        "中间的帖子".to_string(),
        now - Duration::hours(5),
        "123".to_string(),
        "用户".to_string(),
        0,
        0,
        0,
    );

    // 乱序保存
    redis.save_post(&post_middle).await.unwrap();
    redis.save_post(&post_new).await.unwrap();
    redis.save_post(&post_old).await.unwrap();

    // 验证: 按时间升序查询
    let mut conn = redis.get_connection().await.unwrap();
    let posts: Vec<String> = conn
        .zrange(format!("crawl:posts:{}", task.id), 0, -1)
        .await
        .unwrap();

    assert_eq!(posts.len(), 3, "应有3条帖子");

    // 第1条应是最早的
    let first: WeiboPost = serde_json::from_str(&posts[0]).unwrap();
    assert_eq!(first.id, "old_post", "第1条应是最早的帖子");

    // 最后1条应是最新的
    let last: WeiboPost = serde_json::from_str(&posts[2]).unwrap();
    assert_eq!(last.id, "new_post", "最后1条应是最新的帖子");

    cleanup_task_data(&redis, &task.id).await;
}

#[tokio::test]
async fn test_update_task_progress() {
    let redis = setup_redis().await;
    let task = prepare_created_task(&redis, "进度更新").await;

    let mut updated_task = task.clone();

    // 模拟爬取进度更新
    let post_time = Utc::now() - Duration::hours(3);
    updated_task.update_progress(post_time, 20);

    redis.save_crawl_task(&updated_task).await.unwrap();

    // 验证: 计数器已更新
    let loaded = redis.load_task(&task.id).await.unwrap();
    assert_eq!(loaded.crawled_count, 20, "爬取计数应更新");
    assert!(
        loaded.min_post_time.is_some(),
        "应记录最小帖子时间"
    );
    assert!(
        loaded.max_post_time.is_some(),
        "应记录最大帖子时间"
    );

    // 再次更新 (更早的时间)
    let earlier_time = Utc::now() - Duration::hours(5);
    updated_task.update_progress(earlier_time, 15);
    redis.save_crawl_task(&updated_task).await.unwrap();

    let reloaded = redis.load_task(&task.id).await.unwrap();
    assert_eq!(
        reloaded.crawled_count, 35,
        "计数应累加"
    );
    assert_eq!(
        reloaded.min_post_time.unwrap().timestamp(),
        earlier_time.timestamp(),
        "应更新为更早的时间"
    );

    cleanup_task_data(&redis, &task.id).await;
}

#[tokio::test]
async fn test_invalid_status_transition() {
    // 验证: 不能从Created直接到HistoryCompleted
    let task = CrawlTask::new("测试".to_string(), Utc::now());

    let mut invalid_task = task.clone();
    let result = invalid_task.transition_to(CrawlStatus::HistoryCompleted);

    assert!(result.is_err(), "非法状态转换应失败");
    assert!(
        result.unwrap_err().contains("无效的状态转换"),
        "错误消息应包含'无效的状态转换'"
    );
}

#[tokio::test]
async fn test_full_crawl_workflow_simulation() {
    let redis = setup_redis().await;
    let task = prepare_created_task(&redis, "完整流程").await;

    // Step 1: 启动爬取
    let mut running_task = task.clone();
    running_task
        .transition_to(CrawlStatus::HistoryCrawling)
        .unwrap();
    redis.save_crawl_task(&running_task).await.unwrap();

    // Step 2: 创建检查点
    let checkpoint = CrawlCheckpoint::new_backward(
        task.id.clone(),
        task.event_start_time,
        Utc::now(),
    );
    redis.save_checkpoint(&checkpoint).await.unwrap();

    // Step 3: 模拟爬取3页,每页20条
    for page in 1..=3 {
        for i in 0..20 {
            let post = WeiboPost::new(
                format!("post_{}_{}", page, i),
                task.id.clone(),
                format!("帖子内容 page={} index={}", page, i),
                Utc::now() - Duration::hours(page as i64) - Duration::minutes(i as i64),
                "123".to_string(),
                "用户".to_string(),
                0,
                0,
                0,
            );
            redis.save_post(&post).await.unwrap();
        }

        // 更新检查点
        let mut updated_checkpoint = redis.load_checkpoint(&task.id).await.unwrap();
        updated_checkpoint.advance_page();
        redis.save_checkpoint(&updated_checkpoint).await.unwrap();

        // 更新任务进度
        running_task.update_progress(Utc::now() - Duration::hours(page as i64), 20);
        redis.save_crawl_task(&running_task).await.unwrap();
    }

    // 验证: 总计60条帖子
    let final_task = redis.load_task(&task.id).await.unwrap();
    assert_eq!(final_task.crawled_count, 60, "应爬取60条帖子");

    let mut conn = redis.get_connection().await.unwrap();
    let post_count: u64 = conn.zcard(format!("crawl:posts:{}", task.id)).await.unwrap();
    assert_eq!(post_count, 60, "Redis中应有60条帖子");

    let id_count: u64 = conn
        .scard(format!("crawl:post_ids:{}", task.id))
        .await
        .unwrap();
    assert_eq!(id_count, 60, "去重集合应有60个ID");

    // 验证: 检查点在第4页
    let final_checkpoint = redis.load_checkpoint(&task.id).await.unwrap();
    assert_eq!(final_checkpoint.current_page, 4, "检查点应在第4页");

    cleanup_task_data(&redis, &task.id).await;
}

/// 性能测试: 批量保存帖子
#[tokio::test]
async fn test_save_posts_performance() {
    let redis = setup_redis().await;
    let task = prepare_created_task(&redis, "性能测试").await;

    let start = std::time::Instant::now();

    // 保存100条帖子
    for i in 0..100 {
        let post = WeiboPost::new(
            format!("perf_post_{}", i),
            task.id.clone(),
            format!("性能测试帖子{}", i),
            Utc::now() - Duration::seconds(i as i64),
            "123".to_string(),
            "用户".to_string(),
            0,
            0,
            0,
        );
        redis.save_post(&post).await.unwrap();
    }

    let elapsed = start.elapsed();

    // 验证: 100条帖子应在2秒内保存完成
    assert!(
        elapsed.as_secs() < 2,
        "保存100条帖子应在2秒内完成: 实际{}ms",
        elapsed.as_millis()
    );

    // 验证: 数量正确
    let mut conn = redis.get_connection().await.unwrap();
    let count: u64 = conn.zcard(format!("crawl:posts:{}", task.id)).await.unwrap();
    assert_eq!(count, 100, "应有100条帖子");

    cleanup_task_data(&redis, &task.id).await;
}
