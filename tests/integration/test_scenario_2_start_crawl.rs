//! T011 - 场景2集成测试: 启动历史回溯
//!
//! 测试范围:
//! - 任务状态转换: Created → HistoryCrawling
//! - crawl-progress事件推送
//! - Redis检查点保存
//! - Redis帖子数据存储
//! - 帖子ID去重机制
//!
//! 参考文档: specs/003-/quickstart.md 场景2

use std::collections::HashMap;
use chrono::{DateTime, Duration, Utc};
use weibo_login::models::{
    crawl_task::{CrawlTask, CrawlStatus},
    crawl_checkpoint::{CrawlCheckpoint, CrawlDirection},
    crawl_events::CrawlProgressEvent,
    weibo_post::WeiboPost,
};

/// 模拟Redis服务
struct MockRedis {
    tasks: HashMap<String, HashMap<String, String>>,
    checkpoints: HashMap<String, HashMap<String, String>>,
    posts: HashMap<String, Vec<(i64, String)>>,
    post_ids: HashMap<String, Vec<String>>,
}

impl MockRedis {
    fn new() -> Self {
        Self {
            tasks: HashMap::new(),
            checkpoints: HashMap::new(),
            posts: HashMap::new(),
            post_ids: HashMap::new(),
        }
    }

    fn save_task(&mut self, task: &CrawlTask) -> Result<(), String> {
        let key = format!("crawl:task:{}", task.id);
        let mut fields = HashMap::new();
        fields.insert("id".to_string(), task.id.clone());
        fields.insert("keyword".to_string(), task.keyword.clone());
        fields.insert("event_start_time".to_string(), task.event_start_time.timestamp().to_string());
        fields.insert("status".to_string(), format!("{:?}", task.status));
        fields.insert("crawled_count".to_string(), task.crawled_count.to_string());
        fields.insert("created_at".to_string(), task.created_at.timestamp().to_string());
        fields.insert("updated_at".to_string(), task.updated_at.timestamp().to_string());

        if let Some(min_time) = task.min_post_time {
            fields.insert("min_post_time".to_string(), min_time.timestamp().to_string());
        }
        if let Some(max_time) = task.max_post_time {
            fields.insert("max_post_time".to_string(), max_time.timestamp().to_string());
        }

        self.tasks.insert(key, fields);
        Ok(())
    }

    fn get_task(&self, task_id: &str) -> Option<HashMap<String, String>> {
        let key = format!("crawl:task:{}", task_id);
        self.tasks.get(&key).cloned()
    }

    fn save_checkpoint(&mut self, checkpoint: &CrawlCheckpoint) -> Result<(), String> {
        let key = format!("crawl:checkpoint:{}", checkpoint.task_id);
        let mut fields = HashMap::new();
        fields.insert("task_id".to_string(), checkpoint.task_id.clone());
        fields.insert("shard_start_time".to_string(), checkpoint.shard_start_time.timestamp().to_string());
        fields.insert("shard_end_time".to_string(), checkpoint.shard_end_time.timestamp().to_string());
        fields.insert("current_page".to_string(), checkpoint.current_page.to_string());
        fields.insert("direction".to_string(), format!("{:?}", checkpoint.direction));
        fields.insert("saved_at".to_string(), checkpoint.saved_at.timestamp().to_string());

        self.checkpoints.insert(key, fields);
        Ok(())
    }

    fn get_checkpoint(&self, task_id: &str) -> Option<HashMap<String, String>> {
        let key = format!("crawl:checkpoint:{}", task_id);
        self.checkpoints.get(&key).cloned()
    }

    fn add_post(&mut self, task_id: &str, post: &WeiboPost) -> Result<bool, String> {
        let key = format!("crawl:posts:{}", task_id);
        let id_key = format!("crawl:post_ids:{}", task_id);

        let post_ids = self.post_ids.entry(id_key.clone()).or_insert_with(Vec::new);
        if post_ids.contains(&post.id) {
            return Ok(false);
        }

        post_ids.push(post.id.clone());

        let posts = self.posts.entry(key).or_insert_with(Vec::new);
        let json = post.to_json().map_err(|e| e.to_string())?;
        posts.push((post.created_at.timestamp(), json));

        Ok(true)
    }

    fn get_post_count(&self, task_id: &str) -> usize {
        let key = format!("crawl:posts:{}", task_id);
        self.posts.get(&key).map(|v| v.len()).unwrap_or(0)
    }

    fn get_post_id_count(&self, task_id: &str) -> usize {
        let key = format!("crawl:post_ids:{}", task_id);
        self.post_ids.get(&key).map(|v| v.len()).unwrap_or(0)
    }

    fn post_id_exists(&self, task_id: &str, post_id: &str) -> bool {
        let key = format!("crawl:post_ids:{}", task_id);
        self.post_ids
            .get(&key)
            .map(|ids| ids.contains(&post_id.to_string()))
            .unwrap_or(false)
    }
}

/// 模拟爬取服务
struct MockCrawlService {
    redis: MockRedis,
    events: Vec<CrawlProgressEvent>,
}

impl MockCrawlService {
    fn new() -> Self {
        Self {
            redis: MockRedis::new(),
            events: Vec::new(),
        }
    }

    async fn start_crawl(&mut self, task_id: &str) -> Result<(), String> {
        let task_data = self.redis.get_task(task_id)
            .ok_or_else(|| "任务不存在".to_string())?;

        let status = task_data.get("status")
            .ok_or_else(|| "状态字段缺失".to_string())?;

        if status != "Created" {
            return Err(format!("任务状态必须是Created,当前为{}", status));
        }

        let event_start_time = task_data.get("event_start_time")
            .and_then(|s| s.parse::<i64>().ok())
            .and_then(|ts| DateTime::from_timestamp(ts, 0))
            .ok_or_else(|| "无效的事件开始时间".to_string())?;

        let now = Utc::now();
        let shard_end = now;
        let shard_start = event_start_time;

        let checkpoint = CrawlCheckpoint {
            task_id: task_id.to_string(),
            shard_start_time: shard_start,
            shard_end_time: shard_end,
            current_page: 1,
            direction: CrawlDirection::Backward,
            completed_shards: Vec::new(),
            saved_at: Utc::now(),
        };

        self.redis.save_checkpoint(&checkpoint)?;

        let mut task_fields = task_data.clone();
        task_fields.insert("status".to_string(), "HistoryCrawling".to_string());
        task_fields.insert("updated_at".to_string(), Utc::now().timestamp().to_string());
        self.redis.tasks.insert(format!("crawl:task:{}", task_id), task_fields);

        Ok(())
    }

    async fn crawl_page(&mut self, task_id: &str, page: u32) -> Result<usize, String> {
        let checkpoint = self.redis.get_checkpoint(task_id)
            .ok_or_else(|| "检查点不存在".to_string())?;

        let current_page: u32 = checkpoint.get("current_page")
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| "无效的页码".to_string())?;

        if page != current_page {
            return Err(format!("页码不匹配: 期望{}, 实际{}", current_page, page));
        }

        let shard_start_ts = checkpoint.get("shard_start_time")
            .and_then(|s| s.parse::<i64>().ok())
            .ok_or_else(|| "无效的分片开始时间".to_string())?;
        let shard_end_ts = checkpoint.get("shard_end_time")
            .and_then(|s| s.parse::<i64>().ok())
            .ok_or_else(|| "无效的分片结束时间".to_string())?;

        let shard_start = DateTime::from_timestamp(shard_start_ts, 0)
            .ok_or_else(|| "时间戳转换失败".to_string())?;
        let shard_end = DateTime::from_timestamp(shard_end_ts, 0)
            .ok_or_else(|| "时间戳转换失败".to_string())?;

        let posts_per_page = 20;
        let mut added_count = 0;

        for i in 0..posts_per_page {
            let offset = Duration::seconds((page - 1) as i64 * posts_per_page + i as i64);
            let post_time = shard_end - offset;

            if post_time < shard_start {
                break;
            }

            let post = WeiboPost {
                id: format!("post_{}_{}", page, i),
                task_id: task_id.to_string(),
                text: format!("测试帖子内容 page={} index={}", page, i),
                created_at: post_time,
                author_uid: "1234567890".to_string(),
                author_screen_name: "测试用户".to_string(),
                reposts_count: 10,
                comments_count: 5,
                attitudes_count: 20,
                crawled_at: Utc::now(),
            };

            if self.redis.add_post(task_id, &post)? {
                added_count += 1;
            }
        }

        let new_page = page + 1;
        let checkpoint_key = format!("crawl:checkpoint:{}", task_id);
        if let Some(mut cp) = self.redis.checkpoints.get_mut(&checkpoint_key) {
            cp.insert("current_page".to_string(), new_page.to_string());
            cp.insert("saved_at".to_string(), Utc::now().timestamp().to_string());
        }

        let event = CrawlProgressEvent {
            task_id: task_id.to_string(),
            status: "HistoryCrawling".to_string(),
            current_time_range: weibo_login::models::crawl_events::TimeRange {
                start: shard_start.to_rfc3339(),
                end: shard_end.to_rfc3339(),
            },
            current_page: page,
            crawled_count: self.redis.get_post_count(task_id) as u64,
            timestamp: Utc::now().to_rfc3339(),
        };

        self.events.push(event);

        Ok(added_count)
    }

    fn get_events(&self) -> &[CrawlProgressEvent] {
        &self.events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_scenario_2_start_crawl_and_progress() {
        let mut service = MockCrawlService::new();

        let task_id = "task_001";
        let event_start = Utc::now() - Duration::hours(24);

        let mut task_fields = HashMap::new();
        task_fields.insert("id".to_string(), task_id.to_string());
        task_fields.insert("keyword".to_string(), "国庆".to_string());
        task_fields.insert("event_start_time".to_string(), event_start.timestamp().to_string());
        task_fields.insert("status".to_string(), "Created".to_string());
        task_fields.insert("crawled_count".to_string(), "0".to_string());
        task_fields.insert("created_at".to_string(), Utc::now().timestamp().to_string());
        task_fields.insert("updated_at".to_string(), Utc::now().timestamp().to_string());

        service.redis.tasks.insert(format!("crawl:task:{}", task_id), task_fields);

        let result = service.start_crawl(task_id).await;
        assert!(result.is_ok(), "启动爬取应该成功");

        let task_data = service.redis.get_task(task_id).expect("任务应该存在");
        assert_eq!(
            task_data.get("status").unwrap(),
            "HistoryCrawling",
            "任务状态应转换为HistoryCrawling"
        );

        let checkpoint = service.redis.get_checkpoint(task_id).expect("检查点应该存在");
        assert_eq!(
            checkpoint.get("current_page").unwrap(),
            "1",
            "初始页码应为1"
        );
        assert_eq!(
            checkpoint.get("direction").unwrap(),
            "Backward",
            "爬取方向应为Backward"
        );

        let result = service.crawl_page(task_id, 1).await;
        assert!(result.is_ok(), "第1页爬取应该成功");
        let added = result.unwrap();
        assert!(added > 0, "应该添加了帖子");

        let result = service.crawl_page(task_id, 2).await;
        assert!(result.is_ok(), "第2页爬取应该成功");

        let result = service.crawl_page(task_id, 3).await;
        assert!(result.is_ok(), "第3页爬取应该成功");

        let events = service.get_events();
        assert_eq!(events.len(), 3, "应该有3个进度事件");

        assert_eq!(events[0].current_page, 1, "第1个事件页码应为1");
        assert_eq!(events[1].current_page, 2, "第2个事件页码应为2");
        assert_eq!(events[2].current_page, 3, "第3个事件页码应为3");

        for event in events {
            assert_eq!(event.status, "HistoryCrawling", "事件状态应为HistoryCrawling");
            assert_eq!(event.task_id, task_id, "事件任务ID应匹配");
        }

        let checkpoint = service.redis.get_checkpoint(task_id).expect("检查点应该存在");
        assert_eq!(
            checkpoint.get("current_page").unwrap(),
            "4",
            "检查点页码应更新为4"
        );

        let post_count = service.redis.get_post_count(task_id);
        assert!(post_count > 0, "应该有帖子数据");
        assert!(post_count <= 60, "3页最多60条帖子");

        let post_id_count = service.redis.get_post_id_count(task_id);
        assert_eq!(post_count, post_id_count, "帖子数量应与ID集合数量一致");

        assert!(
            service.redis.post_id_exists(task_id, "post_1_0"),
            "第1页第1条帖子ID应存在"
        );
        assert!(
            service.redis.post_id_exists(task_id, "post_2_0"),
            "第2页第1条帖子ID应存在"
        );
        assert!(
            service.redis.post_id_exists(task_id, "post_3_0"),
            "第3页第1条帖子ID应存在"
        );
    }

    #[tokio::test]
    async fn test_deduplication() {
        let mut service = MockCrawlService::new();
        let task_id = "task_dedup";

        let post1 = WeiboPost {
            id: "duplicate_post".to_string(),
            task_id: task_id.to_string(),
            text: "测试去重".to_string(),
            created_at: Utc::now(),
            author_uid: "123".to_string(),
            author_screen_name: "用户".to_string(),
            reposts_count: 0,
            comments_count: 0,
            attitudes_count: 0,
            crawled_at: Utc::now(),
        };

        let added1 = service.redis.add_post(task_id, &post1).expect("第1次添加应成功");
        assert!(added1, "第1次添加应返回true");

        let added2 = service.redis.add_post(task_id, &post1).expect("第2次添加应成功");
        assert!(!added2, "第2次添加应返回false(已存在)");

        let count = service.redis.get_post_count(task_id);
        assert_eq!(count, 1, "应该只有1条帖子");

        let id_count = service.redis.get_post_id_count(task_id);
        assert_eq!(id_count, 1, "去重集合应只有1个ID");
    }

    #[tokio::test]
    async fn test_start_crawl_invalid_status() {
        let mut service = MockCrawlService::new();
        let task_id = "task_invalid";

        let mut task_fields = HashMap::new();
        task_fields.insert("id".to_string(), task_id.to_string());
        task_fields.insert("status".to_string(), "HistoryCrawling".to_string());
        task_fields.insert("event_start_time".to_string(), Utc::now().timestamp().to_string());

        service.redis.tasks.insert(format!("crawl:task:{}", task_id), task_fields);

        let result = service.start_crawl(task_id).await;
        assert!(result.is_err(), "非Created状态应无法启动");
        assert!(
            result.unwrap_err().contains("Created"),
            "错误消息应提示状态要求"
        );
    }

    #[tokio::test]
    async fn test_crawl_page_wrong_page_number() {
        let mut service = MockCrawlService::new();
        let task_id = "task_page";

        let event_start = Utc::now() - Duration::hours(1);
        let mut task_fields = HashMap::new();
        task_fields.insert("id".to_string(), task_id.to_string());
        task_fields.insert("status".to_string(), "Created".to_string());
        task_fields.insert("event_start_time".to_string(), event_start.timestamp().to_string());
        task_fields.insert("created_at".to_string(), Utc::now().timestamp().to_string());
        task_fields.insert("updated_at".to_string(), Utc::now().timestamp().to_string());

        service.redis.tasks.insert(format!("crawl:task:{}", task_id), task_fields);

        service.start_crawl(task_id).await.expect("启动应成功");

        let result = service.crawl_page(task_id, 3).await;
        assert!(result.is_err(), "跳页应该失败");
        assert!(
            result.unwrap_err().contains("页码不匹配"),
            "错误消息应提示页码不匹配"
        );
    }

    #[tokio::test]
    async fn test_redis_checkpoint_persistence() {
        let mut redis = MockRedis::new();
        let task_id = "task_checkpoint";

        let checkpoint = CrawlCheckpoint {
            task_id: task_id.to_string(),
            shard_start_time: Utc::now() - Duration::hours(24),
            shard_end_time: Utc::now(),
            current_page: 15,
            direction: CrawlDirection::Backward,
            completed_shards: Vec::new(),
            saved_at: Utc::now(),
        };

        redis.save_checkpoint(&checkpoint).expect("保存应成功");

        let loaded = redis.get_checkpoint(task_id).expect("检查点应存在");
        assert_eq!(loaded.get("current_page").unwrap(), "15", "页码应正确");
        assert_eq!(loaded.get("direction").unwrap(), "Backward", "方向应正确");
    }
}
