//! 爬取检查点模型

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 爬取检查点
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrawlDirection {
    Backward,
    Forward,
}

impl CrawlCheckpoint {
    pub fn new_backward(
        _task_id: String,
        _shard_start_time: DateTime<Utc>,
        _shard_end_time: DateTime<Utc>,
    ) -> Self {
        todo!("Phase 3.3 - T020实现")
    }

    pub fn new_forward(_task_id: String, _shard_start_time: DateTime<Utc>) -> Self {
        todo!("Phase 3.3 - T020实现")
    }

    pub fn advance_page(&mut self) {
        todo!("Phase 3.3 - T020实现")
    }

    pub fn complete_current_shard(&mut self, _next_start: DateTime<Utc>, _next_end: DateTime<Utc>) {
        todo!("Phase 3.3 - T020实现")
    }

    pub fn redis_key(&self) -> String {
        todo!("Phase 3.3 - T020实现")
    }
}
