//! 集成测试 T013 - 场景4: 完成历史回溯
//!
//! 参考: specs/003-/quickstart.md (场景4)
//!
//! 验证历史回溯完成场景:
//! - 状态转换: HistoryCrawling → HistoryCompleted
//! - 推送 crawl-completed 事件
//! - min_post_time 接近 event_start_time
//! - 帖子时间范围正确

mod common;

use chrono::{DateTime, Duration, Utc};
use common::MockRedisService;
use weibo_login::models::{
    crawl_checkpoint::CrawlCheckpoint,
    crawl_events::CrawlCompletedEvent,
    crawl_task::{CrawlStatus, CrawlTask},
    weibo_post::WeiboPost,
};

/// Mock爬取引擎 - 场景4: 完成历史回溯
///
/// 模拟爬取流程完成,状态转换并推送事件
struct MockCrawlEngine {
    redis: MockRedisService,
}

impl MockCrawlEngine {
    fn new() -> Self {
        Self {
            redis: MockRedisService::new(),
        }
    }

    /// 执行历史回溯直至完成
    ///
    /// # 返回
    /// - 完成事件 CrawlCompletedEvent
    async fn run_until_history_completed(
        &self,
        task: &mut CrawlTask,
        checkpoint: &mut CrawlCheckpoint,
    ) -> Result<CrawlCompletedEvent, String> {
        let start_time = Utc::now();

        // 1. 验证状态前置条件
        if task.status != CrawlStatus::HistoryCrawling {
            return Err(format!(
                "无效的起始状态: {:?}, 期望 HistoryCrawling",
                task.status
            ));
        }

        // 2. 模拟爬取直到达到 event_start_time
        let mut current_time = checkpoint.shard_end_time;
        let target_time = task.event_start_time;
        let mut total_crawled = task.crawled_count;

        while current_time > target_time {
            // 模拟爬取一页 (20条帖子)
            let posts =
                self.simulate_crawl_page(&task.id, current_time, current_time - Duration::hours(1));

            // 保存帖子到Redis
            for post in posts {
                self.save_post(&task.id, &post).await?;
                total_crawled += 1;

                // 更新任务进度
                task.update_progress(post.created_at, 1);
            }

            // 推进检查点
            checkpoint.advance_page();

            // 推进时间
            current_time = current_time - Duration::hours(1);
        }

        // 3. 状态转换: HistoryCrawling → HistoryCompleted
        task.transition_to(CrawlStatus::HistoryCompleted)?;

        // 4. 验证 min_post_time 接近 event_start_time
        if let Some(min_time) = task.min_post_time {
            let diff = (min_time.timestamp() - target_time.timestamp()).abs();
            if diff > 3600 {
                // 允许1小时误差
                return Err(format!(
                    "min_post_time ({}) 与 event_start_time ({}) 相差过大: {} 秒",
                    min_time, target_time, diff
                ));
            }
        } else {
            return Err("min_post_time 应该已设置".to_string());
        }

        // 5. 构建完成事件
        let duration = (Utc::now() - start_time).num_seconds() as u64;
        let event = CrawlCompletedEvent {
            task_id: task.id.clone(),
            final_status: "HistoryCompleted".to_string(),
            total_crawled,
            duration,
            timestamp: Utc::now().to_rfc3339(),
        };

        Ok(event)
    }

    /// 模拟爬取一页帖子
    fn simulate_crawl_page(
        &self,
        task_id: &str,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Vec<WeiboPost> {
        let mut posts = Vec::new();
        let page_size = 20;

        for i in 0..page_size {
            let post_time = end_time + Duration::minutes(i * 3); // 每条帖子间隔3分钟

            let post = WeiboPost::new(
                format!("post_{}_{}", task_id, post_time.timestamp()),
                task_id.to_string(),
                format!("测试帖子内容 - {}", i),
                post_time,
                "1234567890".to_string(),
                "测试用户".to_string(),
                10,
                5,
                20,
            );
            posts.push(post);
        }

        posts
    }

    /// 保存帖子到Redis
    async fn save_post(&self, task_id: &str, post: &WeiboPost) -> Result<(), String> {
        // 1. 去重检查
        let post_ids_key = format!("crawl:post_ids:{}", task_id);
        if self
            .redis
            .exists(&format!("{}:{}", post_ids_key, post.id))
            .await?
        {
            return Ok(()); // 已存在,跳过
        }

        // 2. 保存帖子ID到去重集合
        self.redis
            .hset(&post_ids_key, &post.id, "1".to_string())
            .await?;

        // 3. 保存帖子内容 (模拟 Sorted Set)
        let posts_key = format!("crawl:posts:{}", task_id);
        let post_json = post.to_json().map_err(|e| e.to_string())?;
        self.redis.hset(&posts_key, &post.id, post_json).await?;

        Ok(())
    }

    /// 获取帖子总数
    async fn get_post_count(&self, task_id: &str) -> Result<u64, String> {
        let posts_key = format!("crawl:posts:{}", task_id);
        let posts = self.redis.hgetall(&posts_key).await?;
        Ok(posts.len() as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试场景4: 完成历史回溯
    ///
    /// 验证:
    /// 1. 状态转换: HistoryCrawling → HistoryCompleted
    /// 2. 推送 crawl-completed 事件
    /// 3. min_post_time 接近 event_start_time
    /// 4. 帖子时间范围正确
    #[tokio::test]
    async fn test_scenario_4_history_completed() {
        let engine = MockCrawlEngine::new();

        // 创建任务
        let event_start_time = Utc::now() - Duration::hours(48); // 2天前
        let mut task = CrawlTask::new("国庆".to_string(), event_start_time);

        // 设置为历史回溯状态
        task.transition_to(CrawlStatus::HistoryCrawling)
            .expect("状态转换失败");

        // 创建检查点
        let mut checkpoint =
            CrawlCheckpoint::new_backward(task.id.clone(), event_start_time, Utc::now());

        // 执行历史回溯直到完成
        let result = engine
            .run_until_history_completed(&mut task, &mut checkpoint)
            .await;

        assert!(result.is_ok(), "历史回溯应该成功完成");

        let event = result.unwrap();

        // 验证任务状态
        assert_eq!(
            task.status,
            CrawlStatus::HistoryCompleted,
            "任务状态应为 HistoryCompleted"
        );

        // 验证完成事件
        assert_eq!(event.task_id, task.id, "事件task_id应匹配");
        assert_eq!(
            event.final_status, "HistoryCompleted",
            "事件状态应为 HistoryCompleted"
        );
        assert!(event.total_crawled > 0, "应该爬取到帖子");

        // 验证 min_post_time 接近 event_start_time
        assert!(task.min_post_time.is_some(), "min_post_time 应该已设置");
        let min_time = task.min_post_time.unwrap();
        let diff = (min_time.timestamp() - event_start_time.timestamp()).abs();
        assert!(
            diff <= 3600,
            "min_post_time 应接近 event_start_time (误差 <= 1小时), 实际误差: {} 秒",
            diff
        );

        // 验证 max_post_time 接近当前时间
        assert!(task.max_post_time.is_some(), "max_post_time 应该已设置");
        let max_time = task.max_post_time.unwrap();
        let now = Utc::now();
        let max_diff = (now.timestamp() - max_time.timestamp()).abs();
        assert!(
            max_diff <= 7200,
            "max_post_time 应接近当前时间 (误差 <= 2小时), 实际误差: {} 秒",
            max_diff
        );

        // 验证帖子数据已保存
        let post_count = engine
            .get_post_count(&task.id)
            .await
            .expect("获取帖子数失败");
        assert!(post_count > 0, "应该有帖子保存到Redis");
        assert_eq!(
            post_count, event.total_crawled,
            "保存的帖子数应与事件中的一致"
        );
    }

    /// 测试从非法状态转换到 HistoryCompleted
    ///
    /// 验证:
    /// - 只有 HistoryCrawling 状态可以转换到 HistoryCompleted
    /// - 其他状态转换应失败
    #[tokio::test]
    async fn test_invalid_state_transition() {
        let engine = MockCrawlEngine::new();

        // 创建 Created 状态的任务
        let event_start_time = Utc::now() - Duration::hours(24);
        let mut task = CrawlTask::new("测试".to_string(), event_start_time);

        let mut checkpoint =
            CrawlCheckpoint::new_backward(task.id.clone(), event_start_time, Utc::now());

        // 尝试从 Created 状态完成历史回溯
        let result = engine
            .run_until_history_completed(&mut task, &mut checkpoint)
            .await;

        assert!(result.is_err(), "应该拒绝从 Created 状态转换");
        let error = result.unwrap_err();
        assert!(
            error.contains("无效的起始状态"),
            "错误消息应提示状态无效: {}",
            error
        );
    }

    /// 测试边界情况: 事件开始时间到现在只有1小时
    ///
    /// 验证:
    /// - 短时间范围内任务也能正常完成
    /// - min_post_time 仍然接近 event_start_time
    #[tokio::test]
    async fn test_short_time_range() {
        let engine = MockCrawlEngine::new();

        // 事件开始时间: 1小时前
        let event_start_time = Utc::now() - Duration::hours(1);
        let mut task = CrawlTask::new("短期事件".to_string(), event_start_time);

        task.transition_to(CrawlStatus::HistoryCrawling)
            .expect("状态转换失败");

        let mut checkpoint =
            CrawlCheckpoint::new_backward(task.id.clone(), event_start_time, Utc::now());

        let result = engine
            .run_until_history_completed(&mut task, &mut checkpoint)
            .await;

        assert!(result.is_ok(), "短时间范围应该也能完成");

        let event = result.unwrap();
        assert_eq!(task.status, CrawlStatus::HistoryCompleted);
        assert_eq!(event.final_status, "HistoryCompleted");

        // 验证时间范围
        if let Some(min_time) = task.min_post_time {
            let diff = (min_time.timestamp() - event_start_time.timestamp()).abs();
            assert!(
                diff <= 3600,
                "即使短时间范围,min_post_time 也应接近 event_start_time"
            );
        }
    }

    /// 测试帖子去重
    ///
    /// 验证:
    /// - 重复的帖子ID不会被重复保存
    /// - 去重机制正常工作
    #[tokio::test]
    async fn test_post_deduplication() {
        let engine = MockCrawlEngine::new();
        let task_id = "test_task_123";

        // 创建测试帖子
        let post = WeiboPost::new(
            "test_post_id".to_string(),
            task_id.to_string(),
            "测试内容".to_string(),
            Utc::now(),
            "1234567890".to_string(),
            "测试用户".to_string(),
            10,
            5,
            20,
        );

        // 第一次保存
        let result1 = engine.save_post(task_id, &post).await;
        assert!(result1.is_ok(), "第一次保存应该成功");

        // 第二次保存相同的帖子
        let result2 = engine.save_post(task_id, &post).await;
        assert!(result2.is_ok(), "去重逻辑应该跳过重复帖子");

        // 验证只保存了一次
        let post_count = engine.get_post_count(task_id).await.unwrap();
        assert_eq!(post_count, 1, "应该只有一条帖子 (去重生效)");
    }

    /// 测试爬取事件数据完整性
    ///
    /// 验证:
    /// - 事件包含所有必需字段
    /// - 字段值符合预期
    #[tokio::test]
    async fn test_completed_event_integrity() {
        let engine = MockCrawlEngine::new();

        let event_start_time = Utc::now() - Duration::hours(24);
        let mut task = CrawlTask::new("测试".to_string(), event_start_time);
        task.transition_to(CrawlStatus::HistoryCrawling).unwrap();

        let mut checkpoint =
            CrawlCheckpoint::new_backward(task.id.clone(), event_start_time, Utc::now());

        let event = engine
            .run_until_history_completed(&mut task, &mut checkpoint)
            .await
            .unwrap();

        // 验证事件字段
        assert!(!event.task_id.is_empty(), "task_id 不应为空");
        assert_eq!(
            event.final_status, "HistoryCompleted",
            "final_status 应为 HistoryCompleted"
        );
        assert!(event.total_crawled > 0, "total_crawled 应 > 0");
        assert!(event.duration > 0, "duration 应 > 0");

        // 验证 timestamp 格式 (RFC3339)
        let parsed_time = DateTime::parse_from_rfc3339(&event.timestamp);
        assert!(
            parsed_time.is_ok(),
            "timestamp 应为有效的 RFC3339 格式: {}",
            event.timestamp
        );
    }
}
