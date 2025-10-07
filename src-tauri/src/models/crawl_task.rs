//! 爬取任务模型
//!
//! 表示一次关键字爬取任务的完整生命周期

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 爬取任务
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlTask {
    pub id: String,
    pub keyword: String,
    pub event_start_time: DateTime<Utc>,
    pub status: CrawlStatus,
    pub min_post_time: Option<DateTime<Utc>>,
    pub max_post_time: Option<DateTime<Utc>>,
    pub crawled_count: u64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub failure_reason: Option<String>,
}

/// 爬取任务状态
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrawlStatus {
    Created,
    HistoryCrawling,
    HistoryCompleted,
    IncrementalCrawling,
    Paused,
    Failed,
}

impl CrawlTask {
    pub fn new(_keyword: String, _event_start_time: DateTime<Utc>) -> Self {
        todo!("Phase 3.3 - T018实现")
    }

    pub fn update_progress(&mut self, _post_time: DateTime<Utc>, _post_count: u64) {
        todo!("Phase 3.3 - T018实现")
    }

    pub fn transition_to(&mut self, _new_status: CrawlStatus) -> Result<(), String> {
        todo!("Phase 3.3 - T018实现")
    }

    pub fn mark_failed(&mut self, _reason: String) {
        todo!("Phase 3.3 - T018实现")
    }

    pub fn validate(&self) -> Result<(), String> {
        todo!("Phase 3.3 - T018实现")
    }

    pub fn redis_key(&self) -> String {
        todo!("Phase 3.3 - T018实现")
    }
}

impl CrawlStatus {
    pub fn can_transition_to(&self, _target: &CrawlStatus) -> bool {
        todo!("Phase 3.3 - T018实现")
    }

    pub fn as_str(&self) -> &'static str {
        todo!("Phase 3.3 - T018实现")
    }
}
