//! 爬取检查点模型
//!
//! 职责: 支持断点续爬,记录任务的精确执行位置
//! 三级定位: 时间分片 + 页码 + 方向

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 爬取检查点
///
/// 每个字段都不可替代:
/// - task_id: 关联到爬取任务
/// - shard_start_time/shard_end_time: 当前时间分片范围
/// - current_page: 当前分片内的页码,支持页面级断点续爬
/// - direction: 爬取方向,决定时间范围的推进策略
/// - completed_shards: 已完成的时间分片列表,避免重复爬取
/// - saved_at: 检查点保存时间,用于追溯和诊断
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlCheckpoint {
    pub task_id: String,
    pub shard_start_time: DateTime<Utc>,
    pub shard_end_time: DateTime<Utc>,
    pub current_page: u32,
    pub direction: CrawlDirection,
    pub completed_shards: Vec<(DateTime<Utc>, DateTime<Utc>)>,
    pub saved_at: DateTime<Utc>,
}

/// 爬取方向
///
/// Backward: 向后回溯 (从现在到event_start_time)
/// Forward: 向前更新 (从max_post_time到现在)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrawlDirection {
    Backward,
    Forward,
}

impl CrawlCheckpoint {
    /// 创建新检查点 (历史回溯模式)
    ///
    /// 从现在向过去爬取,直到event_start_time
    pub fn new_backward(
        task_id: String,
        shard_start_time: DateTime<Utc>,
        shard_end_time: DateTime<Utc>,
    ) -> Self {
        Self {
            task_id,
            shard_start_time,
            shard_end_time,
            current_page: 1,
            direction: CrawlDirection::Backward,
            completed_shards: Vec::new(),
            saved_at: Utc::now(),
        }
    }

    /// 创建增量更新检查点
    ///
    /// 从max_post_time向现在爬取
    pub fn new_forward(task_id: String, shard_start_time: DateTime<Utc>) -> Self {
        Self {
            task_id,
            shard_start_time,
            shard_end_time: Utc::now(),
            current_page: 1,
            direction: CrawlDirection::Forward,
            completed_shards: Vec::new(),
            saved_at: Utc::now(),
        }
    }

    /// 推进到下一页
    ///
    /// 在当前时间分片内移动到下一页
    pub fn advance_page(&mut self) {
        self.current_page += 1;
        self.saved_at = Utc::now();
    }

    /// 标记当前分片完成,进入下一个分片
    ///
    /// 将当前分片添加到completed_shards,重置页码,开始新的时间分片
    pub fn complete_current_shard(&mut self, next_start: DateTime<Utc>, next_end: DateTime<Utc>) {
        self.completed_shards
            .push((self.shard_start_time, self.shard_end_time));
        self.shard_start_time = next_start;
        self.shard_end_time = next_end;
        self.current_page = 1;
        self.saved_at = Utc::now();
    }

    /// Redis存储键
    ///
    /// 格式: crawl:checkpoint:{task_id}
    pub fn redis_key(&self) -> String {
        format!("crawl:checkpoint:{}", self.task_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_new_backward() {
        let task_id = "test-task-001".to_string();
        let start = Utc.with_ymd_and_hms(2025, 10, 1, 0, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2025, 10, 7, 23, 59, 59).unwrap();

        let checkpoint = CrawlCheckpoint::new_backward(task_id.clone(), start, end);

        assert_eq!(checkpoint.task_id, task_id);
        assert_eq!(checkpoint.shard_start_time, start);
        assert_eq!(checkpoint.shard_end_time, end);
        assert_eq!(checkpoint.current_page, 1);
        assert_eq!(checkpoint.direction, CrawlDirection::Backward);
        assert_eq!(checkpoint.completed_shards.len(), 0);
    }

    #[test]
    fn test_new_forward() {
        let task_id = "test-task-002".to_string();
        let start = Utc.with_ymd_and_hms(2025, 10, 1, 0, 0, 0).unwrap();

        let checkpoint = CrawlCheckpoint::new_forward(task_id.clone(), start);

        assert_eq!(checkpoint.task_id, task_id);
        assert_eq!(checkpoint.shard_start_time, start);
        assert_eq!(checkpoint.current_page, 1);
        assert_eq!(checkpoint.direction, CrawlDirection::Forward);
        assert_eq!(checkpoint.completed_shards.len(), 0);
    }

    #[test]
    fn test_advance_page() {
        let task_id = "test-task-003".to_string();
        let start = Utc.with_ymd_and_hms(2025, 10, 1, 0, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2025, 10, 7, 23, 59, 59).unwrap();

        let mut checkpoint = CrawlCheckpoint::new_backward(task_id, start, end);

        assert_eq!(checkpoint.current_page, 1);

        checkpoint.advance_page();
        assert_eq!(checkpoint.current_page, 2);

        checkpoint.advance_page();
        assert_eq!(checkpoint.current_page, 3);
    }

    #[test]
    fn test_complete_current_shard() {
        let task_id = "test-task-004".to_string();
        let shard1_start = Utc.with_ymd_and_hms(2025, 10, 1, 0, 0, 0).unwrap();
        let shard1_end = Utc.with_ymd_and_hms(2025, 10, 3, 23, 59, 59).unwrap();

        let mut checkpoint = CrawlCheckpoint::new_backward(task_id, shard1_start, shard1_end);

        checkpoint.advance_page();
        checkpoint.advance_page();
        assert_eq!(checkpoint.current_page, 3);

        let shard2_start = Utc.with_ymd_and_hms(2025, 10, 4, 0, 0, 0).unwrap();
        let shard2_end = Utc.with_ymd_and_hms(2025, 10, 7, 23, 59, 59).unwrap();
        checkpoint.complete_current_shard(shard2_start, shard2_end);

        assert_eq!(checkpoint.completed_shards.len(), 1);
        assert_eq!(checkpoint.completed_shards[0], (shard1_start, shard1_end));
        assert_eq!(checkpoint.shard_start_time, shard2_start);
        assert_eq!(checkpoint.shard_end_time, shard2_end);
        assert_eq!(checkpoint.current_page, 1);
    }

    #[test]
    fn test_redis_key() {
        let task_id = "550e8400-e29b-41d4-a716-446655440000".to_string();
        let start = Utc.with_ymd_and_hms(2025, 10, 1, 0, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2025, 10, 7, 23, 59, 59).unwrap();

        let checkpoint = CrawlCheckpoint::new_backward(task_id.clone(), start, end);

        assert_eq!(
            checkpoint.redis_key(),
            "crawl:checkpoint:550e8400-e29b-41d4-a716-446655440000"
        );
    }

    #[test]
    fn test_multiple_shards_completion() {
        let task_id = "test-task-005".to_string();
        let shard1_start = Utc.with_ymd_and_hms(2025, 10, 1, 0, 0, 0).unwrap();
        let shard1_end = Utc.with_ymd_and_hms(2025, 10, 2, 23, 59, 59).unwrap();

        let mut checkpoint = CrawlCheckpoint::new_backward(task_id, shard1_start, shard1_end);

        let shard2_start = Utc.with_ymd_and_hms(2025, 10, 3, 0, 0, 0).unwrap();
        let shard2_end = Utc.with_ymd_and_hms(2025, 10, 4, 23, 59, 59).unwrap();
        checkpoint.complete_current_shard(shard2_start, shard2_end);

        let shard3_start = Utc.with_ymd_and_hms(2025, 10, 5, 0, 0, 0).unwrap();
        let shard3_end = Utc.with_ymd_and_hms(2025, 10, 7, 23, 59, 59).unwrap();
        checkpoint.complete_current_shard(shard3_start, shard3_end);

        assert_eq!(checkpoint.completed_shards.len(), 2);
        assert_eq!(checkpoint.completed_shards[0], (shard1_start, shard1_end));
        assert_eq!(checkpoint.completed_shards[1], (shard2_start, shard2_end));
        assert_eq!(checkpoint.shard_start_time, shard3_start);
        assert_eq!(checkpoint.shard_end_time, shard3_end);
    }
}
