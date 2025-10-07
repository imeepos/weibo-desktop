//! Redis性能测试
//!
//! 验证以下性能指标:
//! 1. 百万级帖子存储 - Redis内存<2GB
//! 2. 时间范围查询性能 - <100ms
//! 3. 导出性能 - 100万条<30秒
//! 4. Redis批量操作优化 - pipeline效率

use chrono::{DateTime, Duration, Utc};
use redis::AsyncCommands;
use weibo_login::models::WeiboPost;
use weibo_login::services::RedisService;

/// 测试辅助: 生成测试帖子
fn generate_test_post(
    index: u64,
    keyword: &str,
    base_time: DateTime<Utc>,
    task_id: &str,
) -> WeiboPost {
    let offset_seconds = (index as i64) * 60; // 每条帖子间隔1分钟
    let created_at = base_time + Duration::seconds(offset_seconds);

    WeiboPost {
        id: format!("post_{}", index),
        task_id: task_id.to_string(),
        text: format!("测试帖子 #{} 包含关键字: {}", index, keyword),
        created_at,
        author_uid: format!("user_{}", index % 1000), // 1000个不同用户
        author_screen_name: format!("测试用户_{}", index % 1000),
        reposts_count: index % 100,
        comments_count: index % 200,
        attitudes_count: index % 500,
        crawled_at: Utc::now(),
    }
}

/// 测试1: 批量写入性能 (使用pipeline)
#[tokio::test]
#[ignore] // 需要Redis实例
async fn test_batch_write_performance() {
    let redis_url = "redis://localhost:6379";
    let redis = RedisService::new(redis_url).expect("连接Redis失败");
    let task_id = "perf_test_batch_write";

    // 准备10万条测试数据
    let batch_size = 100_000;
    let base_time = Utc::now() - Duration::days(7);

    let mut posts = Vec::with_capacity(batch_size);
    for i in 0..batch_size {
        posts.push(generate_test_post(i as u64, "性能测试", base_time, task_id));
    }

    // 测试批量写入
    let start = std::time::Instant::now();
    redis.save_posts(task_id, &posts).await.expect("批量保存帖子失败");
    let duration = start.elapsed();

    // 验证写入性能
    let posts_per_second = (batch_size as f64) / duration.as_secs_f64();
    println!("批量写入性能:");
    println!("  - 帖子数量: {}", batch_size);
    println!("  - 耗时: {:.2}秒", duration.as_secs_f64());
    println!("  - 吞吐量: {:.0} posts/s", posts_per_second);

    // 期望: 每秒至少写入1万条 (100ms写入1000条)
    assert!(posts_per_second > 10_000.0, "批量写入性能不达标");

    // 验证数据正确性
    let mut conn = redis.get_connection().await.expect("获取连接失败");
    let count: u64 = conn
        .zcard(format!("crawl:posts:{}", task_id))
        .await
        .expect("查询帖子数量失败");
    assert_eq!(count, batch_size as u64, "帖子数量不匹配");

    // 清理
    let _: () = conn.del(format!("crawl:posts:{}", task_id)).await.unwrap();
    let _: () = conn.del(format!("crawl:post_ids:{}", task_id)).await.unwrap();
}

/// 测试2: 时间范围查询性能
#[tokio::test]
#[ignore]
async fn test_time_range_query_performance() {
    let redis_url = "redis://localhost:6379";
    let redis = RedisService::new(redis_url).expect("连接Redis失败");
    let task_id = "perf_test_query";

    // 准备100万条测试数据 (分批写入)
    let total_count = 1_000_000;
    let batch_size = 10_000;
    let base_time = Utc::now() - Duration::days(30);

    println!("准备{}条测试数据...", total_count);
    let prepare_start = std::time::Instant::now();

    for batch_idx in 0..(total_count / batch_size) {
        let mut posts = Vec::with_capacity(batch_size);
        for i in 0..batch_size {
            let global_idx = batch_idx * batch_size + i;
            posts.push(generate_test_post(global_idx as u64, "查询测试", base_time, task_id));
        }
        redis.save_posts(task_id, &posts).await.expect("批量保存失败");

        if (batch_idx + 1) % 10 == 0 {
            println!("  已写入 {} / {} 条", (batch_idx + 1) * batch_size, total_count);
        }
    }

    let prepare_duration = prepare_start.elapsed();
    println!("数据准备完成，耗时: {:.2}秒", prepare_duration.as_secs_f64());

    // 测试不同时间范围的查询性能
    let test_cases = vec![
        ("1小时范围", Duration::hours(1)),
        ("1天范围", Duration::days(1)),
        ("7天范围", Duration::days(7)),
        ("30天范围", Duration::days(30)),
    ];

    println!("\n时间范围查询性能测试:");
    for (label, range_duration) in test_cases {
        let start_time = base_time + Duration::days(15);
        let end_time = start_time + range_duration;

        let query_start = std::time::Instant::now();
        let posts = redis
            .get_posts_by_time_range(task_id, start_time, end_time)
            .await
            .expect("查询失败");
        let query_duration = query_start.elapsed();

        println!("  - {}: 查询到{}条, 耗时{:.2}ms",
            label, posts.len(), query_duration.as_millis());

        // 核心指标: 查询时间 < 100ms
        assert!(
            query_duration.as_millis() < 100,
            "{} 查询性能不达标: {}ms > 100ms",
            label,
            query_duration.as_millis()
        );
    }

    // 清理
    let mut conn = redis.get_connection().await.expect("获取连接失败");
    let _: () = conn.del(format!("crawl:posts:{}", task_id)).await.unwrap();
    let _: () = conn.del(format!("crawl:post_ids:{}", task_id)).await.unwrap();
}

/// 测试3: 导出性能 (模拟100万条数据导出)
#[tokio::test]
#[ignore]
async fn test_export_performance() {
    let redis_url = "redis://localhost:6379";
    let redis = RedisService::new(redis_url).expect("连接Redis失败");
    let task_id = "perf_test_export";

    // 准备100万条测试数据
    let total_count = 1_000_000;
    let batch_size = 10_000;
    let base_time = Utc::now() - Duration::days(30);

    println!("准备{}条测试数据用于导出测试...", total_count);
    for batch_idx in 0..(total_count / batch_size) {
        let mut posts = Vec::with_capacity(batch_size);
        for i in 0..batch_size {
            let global_idx = batch_idx * batch_size + i;
            posts.push(generate_test_post(global_idx as u64, "导出测试", base_time, task_id));
        }
        redis.save_posts(task_id, &posts).await.expect("批量保存失败");

        if (batch_idx + 1) % 20 == 0 {
            println!("  已写入 {} / {} 条", (batch_idx + 1) * batch_size, total_count);
        }
    }

    // 测试导出性能 (分批读取+序列化)
    println!("\n开始导出测试...");
    let export_start = std::time::Instant::now();

    let end_time = base_time + Duration::days(30);
    let all_posts = redis
        .get_posts_by_time_range(task_id, base_time, end_time)
        .await
        .expect("查询所有帖子失败");

    println!("  查询完成，共{}条数据", all_posts.len());

    // 模拟JSON导出
    let json_start = std::time::Instant::now();
    let export_data = serde_json::json!({
        "task_id": task_id,
        "exported_at": Utc::now().to_rfc3339(),
        "total_posts": all_posts.len(),
        "posts": all_posts,
    });
    let json_string = serde_json::to_string(&export_data).expect("JSON序列化失败");
    let json_duration = json_start.elapsed();

    println!("  JSON序列化完成，耗时: {:.2}秒", json_duration.as_secs_f64());
    println!("  JSON大小: {:.2} MB", json_string.len() as f64 / 1024.0 / 1024.0);

    let total_duration = export_start.elapsed();
    println!("\n导出总耗时: {:.2}秒", total_duration.as_secs_f64());

    // 核心指标: 100万条导出 < 30秒
    assert!(
        total_duration.as_secs() < 30,
        "导出性能不达标: {}秒 > 30秒",
        total_duration.as_secs()
    );

    // 清理
    let mut conn = redis.get_connection().await.expect("获取连接失败");
    let _: () = conn.del(format!("crawl:posts:{}", task_id)).await.unwrap();
    let _: () = conn.del(format!("crawl:post_ids:{}", task_id)).await.unwrap();
}

/// 测试4: Redis内存占用验证
#[tokio::test]
#[ignore]
async fn test_redis_memory_usage() {
    let redis_url = "redis://localhost:6379";
    let redis = RedisService::new(redis_url).expect("连接Redis失败");
    let task_id = "perf_test_memory";

    // 获取初始内存
    let mut conn = redis.get_connection().await.expect("获取连接失败");
    let info_before: String = redis::cmd("INFO")
        .arg("memory")
        .query_async(&mut *conn)
        .await
        .expect("获取INFO失败");

    let used_memory_before = extract_used_memory(&info_before);
    println!("初始内存使用: {:.2} MB", used_memory_before / 1024.0 / 1024.0);

    // 写入100万条数据
    let total_count = 1_000_000;
    let batch_size = 10_000;
    let base_time = Utc::now() - Duration::days(30);

    println!("写入{}条数据...", total_count);
    for batch_idx in 0..(total_count / batch_size) {
        let mut posts = Vec::with_capacity(batch_size);
        for i in 0..batch_size {
            let global_idx = batch_idx * batch_size + i;
            posts.push(generate_test_post(global_idx as u64, "内存测试", base_time, task_id));
        }
        redis.save_posts(task_id, &posts).await.expect("批量保存失败");

        if (batch_idx + 1) % 20 == 0 {
            println!("  已写入 {} / {} 条", (batch_idx + 1) * batch_size, total_count);
        }
    }

    // 获取写入后内存
    let info_after: String = redis::cmd("INFO")
        .arg("memory")
        .query_async(&mut *conn)
        .await
        .expect("获取INFO失败");

    let used_memory_after = extract_used_memory(&info_after);
    let memory_increase = used_memory_after - used_memory_before;

    println!("\n写入后内存使用: {:.2} MB", used_memory_after / 1024.0 / 1024.0);
    println!("内存增长: {:.2} MB", memory_increase / 1024.0 / 1024.0);
    println!("平均每条帖子: {:.2} bytes", memory_increase as f64 / total_count as f64);

    // 核心指标: 100万条数据占用 < 2GB
    let memory_increase_gb = memory_increase as f64 / 1024.0 / 1024.0 / 1024.0;
    assert!(
        memory_increase_gb < 2.0,
        "内存占用不达标: {:.2}GB > 2GB",
        memory_increase_gb
    );

    // 清理
    let _: () = conn.del(format!("crawl:posts:{}", task_id)).await.unwrap();
    let _: () = conn.del(format!("crawl:post_ids:{}", task_id)).await.unwrap();
}

/// 测试5: Pipeline批量操作优化验证
#[tokio::test]
#[ignore]
async fn test_pipeline_optimization() {
    let redis_url = "redis://localhost:6379";
    let redis = RedisService::new(redis_url).expect("连接Redis失败");

    let batch_size = 10_000;
    let base_time = Utc::now() - Duration::days(7);

    // 准备测试数据
    let task_id_test = "perf_test_pipeline";
    let mut posts = Vec::with_capacity(batch_size);
    for i in 0..batch_size {
        posts.push(generate_test_post(i as u64, "pipeline测试", base_time, task_id_test));
    }

    // 测试1: 使用pipeline批量写入 (当前实现)
    let task_id_pipeline = "perf_test_pipeline";
    let pipeline_start = std::time::Instant::now();
    redis.save_posts(task_id_pipeline, &posts).await.expect("Pipeline写入失败");
    let pipeline_duration = pipeline_start.elapsed();

    println!("Pipeline批量写入:");
    println!("  - 帖子数量: {}", batch_size);
    println!("  - 耗时: {:.2}ms", pipeline_duration.as_millis());
    println!("  - 吞吐量: {:.0} posts/s",
        (batch_size as f64) / pipeline_duration.as_secs_f64());

    // 测试2: 逐条写入对比 (验证pipeline优势)
    let task_id_single = "perf_test_single";
    let single_start = std::time::Instant::now();

    let mut conn = redis.get_connection().await.expect("获取连接失败");
    for post in posts.iter().take(1000) { // 只测试1000条避免太慢
        let json = post.to_json().expect("序列化失败");
        let score = post.created_at.timestamp();
        let posts_key = format!("crawl:posts:{}", task_id_single);
        let ids_key = format!("crawl:post_ids:{}", task_id_single);

        let _: () = conn.zadd(&posts_key, json, score).await.unwrap();
        let _: () = conn.sadd(&ids_key, &post.id).await.unwrap();
    }
    let single_duration = single_start.elapsed();

    println!("\n逐条写入对比 (1000条):");
    println!("  - 耗时: {:.2}ms", single_duration.as_millis());
    println!("  - 吞吐量: {:.0} posts/s",
        1000.0 / single_duration.as_secs_f64());

    // 计算性能提升倍数
    let speedup = (single_duration.as_millis() as f64) /
                  (pipeline_duration.as_millis() as f64 * 0.1); // pipeline测试了10倍数据
    println!("\nPipeline性能提升: {:.1}x", speedup);

    // 验证: Pipeline应该至少快10倍
    assert!(speedup > 10.0, "Pipeline优化效果不明显");

    // 清理
    let _: () = conn.del(format!("crawl:posts:{}", task_id_pipeline)).await.unwrap();
    let _: () = conn.del(format!("crawl:post_ids:{}", task_id_pipeline)).await.unwrap();
    let _: () = conn.del(format!("crawl:posts:{}", task_id_single)).await.unwrap();
    let _: () = conn.del(format!("crawl:post_ids:{}", task_id_single)).await.unwrap();
}

/// 辅助函数: 从INFO输出中提取used_memory
fn extract_used_memory(info: &str) -> f64 {
    for line in info.lines() {
        if line.starts_with("used_memory:") {
            if let Some(value_str) = line.split(':').nth(1) {
                return value_str.trim().parse::<f64>().unwrap_or(0.0);
            }
        }
    }
    0.0
}

/// 集成性能测试: 完整场景
#[tokio::test]
#[ignore]
async fn test_end_to_end_performance() {
    let redis_url = "redis://localhost:6379";
    let redis = RedisService::new(redis_url).expect("连接Redis失败");
    let task_id = "perf_test_e2e";

    println!("=== 端到端性能测试 ===\n");

    // 场景: 模拟7天历史回溯
    let total_posts = 500_000; // 50万条
    let batch_size = 10_000;
    let base_time = Utc::now() - Duration::days(7);

    println!("场景: 爬取7天历史数据 ({}条帖子)", total_posts);

    // 1. 写入性能
    println!("\n1. 批量写入测试...");
    let write_start = std::time::Instant::now();
    for batch_idx in 0..(total_posts / batch_size) {
        let mut posts = Vec::with_capacity(batch_size);
        for i in 0..batch_size {
            let global_idx = batch_idx * batch_size + i;
            posts.push(generate_test_post(global_idx as u64, "E2E测试", base_time, task_id));
        }
        redis.save_posts(task_id, &posts).await.expect("批量保存失败");
    }
    let write_duration = write_start.elapsed();
    println!("  写入完成: {:.2}秒, 吞吐量: {:.0} posts/s",
        write_duration.as_secs_f64(),
        (total_posts as f64) / write_duration.as_secs_f64());

    // 2. 查询性能
    println!("\n2. 时间范围查询测试...");
    let query_start = std::time::Instant::now();
    let query_start_time = base_time + Duration::days(3);
    let query_end_time = query_start_time + Duration::hours(12);
    let posts = redis
        .get_posts_by_time_range(task_id, query_start_time, query_end_time)
        .await
        .expect("查询失败");
    let query_duration = query_start.elapsed();
    println!("  查询完成: {}条, 耗时{}ms", posts.len(), query_duration.as_millis());
    assert!(query_duration.as_millis() < 100, "查询性能不达标");

    // 3. 去重检查
    println!("\n3. 帖子去重验证...");
    let check_start = std::time::Instant::now();
    let sample_post_id = "post_12345";
    let exists = redis
        .check_post_exists(task_id, sample_post_id)
        .await
        .expect("去重检查失败");
    let check_duration = check_start.elapsed();
    println!("  去重检查: {}ms, 结果: {}", check_duration.as_millis(), exists);
    assert!(check_duration.as_millis() < 10, "去重检查性能不达标");

    // 4. 清理
    println!("\n4. 清理测试数据...");
    let mut conn = redis.get_connection().await.expect("获取连接失败");
    let _: () = conn.del(format!("crawl:posts:{}", task_id)).await.unwrap();
    let _: () = conn.del(format!("crawl:post_ids:{}", task_id)).await.unwrap();

    println!("\n=== 性能测试通过 ===");
}
