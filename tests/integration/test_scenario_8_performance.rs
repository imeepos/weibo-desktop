/// 集成测试 - 场景8: 性能验证
///
/// 验证目标:
/// - 支持百万级帖子存储
/// - 帖子查询按时间范围 <100ms
/// - 断点续爬精确性验证
///
/// 测试覆盖:
/// 1. 百万级数据存储性能
/// 2. 时间范围查询性能
/// 3. 断点续爬数据完整性
use chrono::{Duration, Utc};
use redis::AsyncCommands;
use weibo_login::models::crawl_task::{CrawlStatus, CrawlTask};
use weibo_login::models::crawl_checkpoint::{CrawlCheckpoint, CrawlDirection};
use weibo_login::models::weibo_post::WeiboPost;
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
    let _: () = conn.del(format!("crawl:posts:{}", task_id)).await.unwrap();
    let _: () = conn.del(format!("crawl:post_ids:{}", task_id)).await.unwrap();
    let _: () = conn.del(format!("crawl:checkpoint:{}", task_id)).await.unwrap();
}

/// 测试辅助: 生成测试帖子
fn generate_test_post(task_id: &str, index: u64, base_time: chrono::DateTime<Utc>) -> WeiboPost {
    WeiboPost::new(
        format!("5008471{:010}", index),
        task_id.to_string(),
        format!("测试帖子内容 #{}", index),
        base_time + Duration::seconds(index as i64),
        format!("author_{}", index % 100),
        format!("测试用户{}", index % 100),
        index % 1000,
        index % 500,
        index % 2000,
    )
}

/// 性能测试: 验证万级帖子存储
///
/// 由于CI环境限制,使用1万条数据模拟验证,
/// 实际生产环境应支持100万+帖子存储
#[tokio::test]
async fn test_performance_10k_posts_storage() {
    let redis = setup_redis().await;

    // 创建任务
    let task = CrawlTask::new(
        "性能测试_存储".to_string(),
        Utc::now() - Duration::days(30),
    );
    redis.save_crawl_task(&task).await.unwrap();

    let base_time = Utc::now() - Duration::days(30);
    let post_count = 10_000u64;

    // 批量生成帖子
    let mut posts = Vec::with_capacity(post_count as usize);
    for i in 0..post_count {
        posts.push(generate_test_post(&task.id, i, base_time));
    }

    // 测试存储性能
    let start = std::time::Instant::now();

    // 批量保存(模拟实际爬取场景,每批100条)
    for chunk in posts.chunks(100) {
        redis.save_posts(&task.id, chunk.to_vec()).await.unwrap();
    }

    let elapsed = start.elapsed();

    // 验证: 1万条帖子存储应在30秒内完成
    assert!(
        elapsed.as_secs() < 30,
        "存储1万条帖子应在30秒内完成: 实际{}秒",
        elapsed.as_secs()
    );

    // 验证: 数据完整性
    let mut conn = redis.get_connection().await.unwrap();
    let stored_count: u64 = conn
        .zcard(format!("crawl:posts:{}", task.id))
        .await
        .unwrap();

    assert_eq!(
        stored_count, post_count,
        "存储的帖子数量应与生成数量一致"
    );

    // 验证: 去重索引正确
    let dedupe_count: u64 = conn
        .scard(format!("crawl:post_ids:{}", task.id))
        .await
        .unwrap();

    assert_eq!(
        dedupe_count, post_count,
        "去重索引数量应与帖子数量一致"
    );

    // 估算内存占用
    let memory_kb = (stored_count * 512) / 1024; // 假设每条帖子约512字节
    println!("估算内存占用: {}KB ({}MB)", memory_kb, memory_kb / 1024);

    // 清理
    cleanup_task(&redis, &task.id).await;
}

/// 性能测试: 验证时间范围查询响应时间
#[tokio::test]
async fn test_performance_time_range_query() {
    let redis = setup_redis().await;

    // 创建任务并存储1000条帖子
    let task = CrawlTask::new(
        "性能测试_查询".to_string(),
        Utc::now() - Duration::days(10),
    );
    redis.save_crawl_task(&task).await.unwrap();

    let base_time = Utc::now() - Duration::days(10);
    let post_count = 1000u64;

    let mut posts = Vec::with_capacity(post_count as usize);
    for i in 0..post_count {
        posts.push(generate_test_post(&task.id, i, base_time));
    }

    redis.save_posts(&task.id, posts).await.unwrap();

    // 测试查询性能: 查询最近3天的帖子
    let query_start_time = base_time + Duration::days(7);
    let query_end_time = base_time + Duration::days(10);

    let start = std::time::Instant::now();

    let results = redis
        .get_posts_by_time_range(&task.id, query_start_time, query_end_time)
        .await
        .unwrap();

    let elapsed = start.elapsed();

    // 验证: 查询响应时间应<100ms
    assert!(
        elapsed.as_millis() < 100,
        "时间范围查询应在100ms内完成: 实际{}ms",
        elapsed.as_millis()
    );

    // 验证: 查询结果正确
    let expected_count = Duration::days(3).num_seconds() as usize;
    assert!(
        results.len() > 0 && results.len() <= expected_count,
        "查询结果数量应合理: 实际{}条",
        results.len()
    );

    // 验证: 所有结果都在时间范围内
    for post in results {
        assert!(
            post.created_at >= query_start_time && post.created_at <= query_end_time,
            "帖子时间应在查询范围内"
        );
    }

    // 清理
    cleanup_task(&redis, &task.id).await;
}

/// 功能测试: 断点续爬精确性验证
///
/// 验证在不同页码暂停后恢复,数据无重复
#[tokio::test]
async fn test_checkpoint_resume_accuracy() {
    let redis = setup_redis().await;

    // 创建任务
    let task = CrawlTask::new(
        "断点续爬测试".to_string(),
        Utc::now() - Duration::days(7),
    );
    redis.save_crawl_task(&task).await.unwrap();

    let base_time = Utc::now() - Duration::days(7);

    // 场景1: 在第3页暂停
    let checkpoint1 = CrawlCheckpoint::new_backward(
        task.id.clone(),
        base_time,
        base_time + Duration::hours(1),
    );

    let mut current_checkpoint = checkpoint1.clone();
    current_checkpoint.current_page = 3;
    redis.save_checkpoint(&current_checkpoint).await.unwrap();

    // 模拟第1-3页的数据
    let mut posts_batch1 = Vec::new();
    for i in 0..60 {
        posts_batch1.push(generate_test_post(&task.id, i, base_time));
    }
    redis.save_posts(&task.id, posts_batch1).await.unwrap();

    // 恢复并从第4页继续
    let loaded_checkpoint = redis.load_checkpoint(&task.id).await.unwrap();
    assert_eq!(loaded_checkpoint.current_page, 3, "检查点页码应为3");

    // 模拟恢复后的第4-6页数据
    let mut posts_batch2 = Vec::new();
    for i in 60..120 {
        posts_batch2.push(generate_test_post(&task.id, i, base_time));
    }
    redis.save_posts(&task.id, posts_batch2).await.unwrap();

    // 验证: 总帖子数应为120(无重复)
    let mut conn = redis.get_connection().await.unwrap();
    let total_count: u64 = conn
        .zcard(format!("crawl:posts:{}", task.id))
        .await
        .unwrap();

    assert_eq!(total_count, 120, "恢复后总帖子数应为120(无重复)");

    // 验证: 去重索引100%有效
    let dedupe_count: u64 = conn
        .scard(format!("crawl:post_ids:{}", task.id))
        .await
        .unwrap();

    assert_eq!(dedupe_count, 120, "去重索引应100%有效");

    // 清理
    cleanup_task(&redis, &task.id).await;
}

/// 功能测试: 多次暂停恢复验证
#[tokio::test]
async fn test_multiple_pause_resume_cycles() {
    let redis = setup_redis().await;

    let task = CrawlTask::new(
        "多次暂停恢复测试".to_string(),
        Utc::now() - Duration::days(5),
    );
    redis.save_crawl_task(&task).await.unwrap();

    let base_time = Utc::now() - Duration::days(5);
    let pause_points = vec![3, 20, 45]; // 在第3、20、45页暂停
    let mut total_posts_added = 0u64;

    for (cycle, &pause_page) in pause_points.iter().enumerate() {
        // 创建检查点
        let mut checkpoint = CrawlCheckpoint::new_backward(
            task.id.clone(),
            base_time,
            base_time + Duration::hours(1),
        );
        checkpoint.current_page = pause_page;
        redis.save_checkpoint(&checkpoint).await.unwrap();

        // 添加该周期的帖子数据
        let posts_in_cycle = 20;
        let mut posts = Vec::new();
        for i in 0..posts_in_cycle {
            let post_index = total_posts_added + i;
            posts.push(generate_test_post(&task.id, post_index, base_time));
        }

        redis.save_posts(&task.id, posts).await.unwrap();
        total_posts_added += posts_in_cycle;

        // 验证检查点保存成功
        let loaded = redis.load_checkpoint(&task.id).await.unwrap();
        assert_eq!(
            loaded.current_page, pause_page,
            "周期{}的检查点页码应为{}",
            cycle + 1,
            pause_page
        );
    }

    // 最终验证: 无重复数据
    let mut conn = redis.get_connection().await.unwrap();
    let final_count: u64 = conn
        .zcard(format!("crawl:posts:{}", task.id))
        .await
        .unwrap();

    assert_eq!(
        final_count, total_posts_added,
        "多次暂停恢复后帖子总数应为{}",
        total_posts_added
    );

    // 验证: 去重机制正常工作
    let dedupe_count: u64 = conn
        .scard(format!("crawl:post_ids:{}", task.id))
        .await
        .unwrap();

    assert_eq!(
        dedupe_count, total_posts_added,
        "去重后数量应与添加数量一致"
    );

    // 清理
    cleanup_task(&redis, &task.id).await;
}

/// 性能测试: 验证导出大数据集性能
#[tokio::test]
async fn test_performance_export_large_dataset() {
    let redis = setup_redis().await;

    let task = CrawlTask::new(
        "导出性能测试".to_string(),
        Utc::now() - Duration::days(7),
    );
    redis.save_crawl_task(&task).await.unwrap();

    let base_time = Utc::now() - Duration::days(7);
    let post_count = 5000u64; // 5000条数据模拟导出场景

    let mut posts = Vec::with_capacity(post_count as usize);
    for i in 0..post_count {
        posts.push(generate_test_post(&task.id, i, base_time));
    }

    // 批量保存
    for chunk in posts.chunks(500) {
        redis.save_posts(&task.id, chunk.to_vec()).await.unwrap();
    }

    // 测试全量导出性能
    let export_start = std::time::Instant::now();

    let all_posts = redis
        .get_posts_by_time_range(
            &task.id,
            base_time,
            base_time + Duration::days(7),
        )
        .await
        .unwrap();

    let export_elapsed = export_start.elapsed();

    // 验证: 导出5000条数据应在5秒内完成
    assert!(
        export_elapsed.as_secs() < 5,
        "导出5000条数据应在5秒内完成: 实际{}秒",
        export_elapsed.as_secs()
    );

    // 验证: 导出数据完整
    assert_eq!(
        all_posts.len() as u64,
        post_count,
        "导出数据数量应完整"
    );

    // 模拟序列化为JSON(实际导出操作)
    let serialize_start = std::time::Instant::now();
    let _json_data = serde_json::to_string(&all_posts).unwrap();
    let serialize_elapsed = serialize_start.elapsed();

    // 验证: JSON序列化应在2秒内完成
    assert!(
        serialize_elapsed.as_secs() < 2,
        "JSON序列化应在2秒内完成: 实际{}秒",
        serialize_elapsed.as_secs()
    );

    // 清理
    cleanup_task(&redis, &task.id).await;
}

/// 功能测试: 验证检查点恢复正确页码
#[tokio::test]
async fn test_checkpoint_resume_from_correct_page() {
    let redis = setup_redis().await;

    let task = CrawlTask::new(
        "页码恢复测试".to_string(),
        Utc::now() - Duration::days(3),
    );
    redis.save_crawl_task(&task).await.unwrap();

    let base_time = Utc::now() - Duration::days(3);

    // 测试场景: 在第1页暂停,恢复后从第2页开始
    let checkpoint = CrawlCheckpoint::new_backward(
        task.id.clone(),
        base_time,
        base_time + Duration::hours(2),
    );

    redis.save_checkpoint(&checkpoint).await.unwrap();

    // 加载并验证
    let loaded = redis.load_checkpoint(&task.id).await.unwrap();
    assert_eq!(loaded.current_page, 1, "初始页码应为1");

    // 模拟推进到下一页
    let mut advanced_checkpoint = loaded.clone();
    advanced_checkpoint.advance_page();
    assert_eq!(advanced_checkpoint.current_page, 2, "推进后页码应为2");

    redis.save_checkpoint(&advanced_checkpoint).await.unwrap();

    // 验证: 恢复后从第2页继续
    let resumed = redis.load_checkpoint(&task.id).await.unwrap();
    assert_eq!(resumed.current_page, 2, "恢复后应从第2页开始");

    // 清理
    cleanup_task(&redis, &task.id).await;
}

/// 功能测试: 验证最后一页暂停后恢复
#[tokio::test]
async fn test_checkpoint_resume_from_last_page() {
    let redis = setup_redis().await;

    let task = CrawlTask::new(
        "最后一页恢复测试".to_string(),
        Utc::now() - Duration::days(1),
    );
    redis.save_crawl_task(&task).await.unwrap();

    let base_time = Utc::now() - Duration::days(1);

    // 在第50页(最后一页)暂停
    let mut checkpoint = CrawlCheckpoint::new_backward(
        task.id.clone(),
        base_time,
        base_time + Duration::hours(1),
    );
    checkpoint.current_page = 50;

    redis.save_checkpoint(&checkpoint).await.unwrap();

    // 模拟完成当前分片,进入下一个分片
    let next_shard_start = base_time - Duration::hours(1);
    let next_shard_end = base_time;

    let mut advanced = checkpoint.clone();
    advanced.complete_current_shard(next_shard_start, next_shard_end);

    // 验证: 完成分片后页码重置为1
    assert_eq!(advanced.current_page, 1, "新分片页码应重置为1");

    // 验证: 已完成分片记录正确
    assert_eq!(
        advanced.completed_shards.len(),
        1,
        "应记录1个已完成分片"
    );

    // 清理
    cleanup_task(&redis, &task.id).await;
}
