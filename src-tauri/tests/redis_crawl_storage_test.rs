//! Redis存储集成测试 - 爬取任务、检查点、帖子
//!
//! 验证 T023 实现的所有存储方法

#[cfg(test)]
mod redis_crawl_storage_tests {
    use chrono::{Duration, Utc};
    use weibo_login::models::{CrawlCheckpoint, CrawlDirection, CrawlStatus, CrawlTask, WeiboPost};
    use weibo_login::services::RedisService;

    const REDIS_URL: &str = "redis://localhost:6379";

    #[tokio::test]
    #[ignore] // 需要运行Redis实例
    async fn test_save_and_load_crawl_task() {
        let service = RedisService::new(REDIS_URL).unwrap();

        let event_start_time = Utc::now() - Duration::days(7);
        let task = CrawlTask::new("国庆".to_string(), event_start_time);

        // 保存任务
        service.save_crawl_task(&task).await.unwrap();

        // 加载任务
        let loaded = service.load_task(&task.id).await.unwrap();

        // 验证数据一致性
        assert_eq!(loaded.id, task.id);
        assert_eq!(loaded.keyword, task.keyword);
        assert_eq!(loaded.status, CrawlStatus::Created);
        assert_eq!(loaded.crawled_count, 0);

        // 清理
        let mut conn = service.get_connection().await.unwrap();
        redis::cmd("DEL")
            .arg(&task.redis_key())
            .query_async::<_, ()>(&mut *conn)
            .await
            .unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_list_all_tasks() {
        let service = RedisService::new(REDIS_URL).unwrap();

        // 创建多个任务
        let task1 = CrawlTask::new("国庆".to_string(), Utc::now() - Duration::days(7));
        let task2 = CrawlTask::new("春节".to_string(), Utc::now() - Duration::days(14));

        service.save_crawl_task(&task1).await.unwrap();
        service.save_crawl_task(&task2).await.unwrap();

        // 列出所有任务
        let tasks = service.list_all_tasks().await.unwrap();
        assert!(tasks.len() >= 2);

        // 验证任务存在
        let task_ids: Vec<String> = tasks.iter().map(|t| t.id.clone()).collect();
        assert!(task_ids.contains(&task1.id));
        assert!(task_ids.contains(&task2.id));

        // 清理
        let mut conn = service.get_connection().await.unwrap();
        redis::cmd("DEL")
            .arg(&task1.redis_key())
            .arg(&task2.redis_key())
            .query_async::<_, ()>(&mut *conn)
            .await
            .unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_save_and_load_checkpoint() {
        let service = RedisService::new(REDIS_URL).unwrap();

        let task_id = "test-task-001".to_string();
        let shard_start = Utc::now() - Duration::hours(2);
        let shard_end = Utc::now();

        let checkpoint = CrawlCheckpoint::new_backward(task_id.clone(), shard_start, shard_end);

        // 保存检查点
        service.save_checkpoint(&checkpoint).await.unwrap();

        // 加载检查点
        let loaded = service.load_checkpoint(&task_id).await.unwrap();
        assert!(loaded.is_some());

        let loaded = loaded.unwrap();
        assert_eq!(loaded.task_id, checkpoint.task_id);
        assert_eq!(loaded.current_page, 1);
        assert_eq!(loaded.direction, CrawlDirection::Backward);

        // 清理
        let mut conn = service.get_connection().await.unwrap();
        redis::cmd("DEL")
            .arg(&checkpoint.redis_key())
            .query_async::<_, ()>(&mut *conn)
            .await
            .unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_save_and_query_posts() {
        let service = RedisService::new(REDIS_URL).unwrap();

        let task_id = "test-task-posts-001";

        // 创建测试帖子
        let posts = vec![
            WeiboPost::new(
                "post_001".to_string(),
                task_id.to_string(),
                "测试帖子1".to_string(),
                Utc::now() - Duration::hours(5),
                "user_001".to_string(),
                "测试用户1".to_string(),
                10,
                20,
                30,
            ),
            WeiboPost::new(
                "post_002".to_string(),
                task_id.to_string(),
                "测试帖子2".to_string(),
                Utc::now() - Duration::hours(3),
                "user_002".to_string(),
                "测试用户2".to_string(),
                15,
                25,
                35,
            ),
            WeiboPost::new(
                "post_003".to_string(),
                task_id.to_string(),
                "测试帖子3".to_string(),
                Utc::now() - Duration::hours(1),
                "user_003".to_string(),
                "测试用户3".to_string(),
                20,
                30,
                40,
            ),
        ];

        // 批量保存
        service.save_posts(task_id, &posts).await.unwrap();

        // 检查帖子存在性
        let exists_001 = service.check_post_exists(task_id, "post_001").await.unwrap();
        let exists_002 = service.check_post_exists(task_id, "post_002").await.unwrap();
        let exists_999 = service.check_post_exists(task_id, "post_999").await.unwrap();

        assert!(exists_001);
        assert!(exists_002);
        assert!(!exists_999);

        // 按时间范围查询
        let start = Utc::now() - Duration::hours(4);
        let end = Utc::now();
        let queried = service
            .get_posts_by_time_range(task_id, start, end)
            .await
            .unwrap();

        assert_eq!(queried.len(), 2); // post_002 和 post_003

        // 清理
        let mut conn = service.get_connection().await.unwrap();
        let posts_key = format!("crawl:posts:{}", task_id);
        let ids_key = format!("crawl:post_ids:{}", task_id);
        redis::cmd("DEL")
            .arg(&posts_key)
            .arg(&ids_key)
            .query_async::<_, ()>(&mut *conn)
            .await
            .unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_checkpoint_advance_page() {
        let service = RedisService::new(REDIS_URL).unwrap();

        let task_id = "test-task-002".to_string();
        let shard_start = Utc::now() - Duration::hours(2);
        let shard_end = Utc::now();

        let mut checkpoint = CrawlCheckpoint::new_backward(task_id.clone(), shard_start, shard_end);

        // 推进页码
        checkpoint.advance_page();
        checkpoint.advance_page();
        assert_eq!(checkpoint.current_page, 3);

        // 保存并重新加载
        service.save_checkpoint(&checkpoint).await.unwrap();
        let loaded = service.load_checkpoint(&task_id).await.unwrap().unwrap();
        assert_eq!(loaded.current_page, 3);

        // 清理
        let mut conn = service.get_connection().await.unwrap();
        redis::cmd("DEL")
            .arg(&checkpoint.redis_key())
            .query_async::<_, ()>(&mut *conn)
            .await
            .unwrap();
    }
}
