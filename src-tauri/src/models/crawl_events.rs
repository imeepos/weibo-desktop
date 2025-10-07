//! 爬取事件模型

use serde::{Deserialize, Serialize};

/// 爬取进度事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlProgressEvent {
    pub task_id: String,
    pub status: String,
    pub current_time_range: TimeRange,
    pub current_page: u32,
    pub crawled_count: u64,
    pub timestamp: String,
}

/// 爬取完成事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlCompletedEvent {
    pub task_id: String,
    pub final_status: String,
    pub total_crawled: u64,
    pub duration: u64,
    pub timestamp: String,
}

/// 爬取错误事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlErrorEvent {
    pub task_id: String,
    pub error: String,
    pub error_code: String,
    pub timestamp: String,
}

/// 时间范围
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    pub start: String,
    pub end: String,
}
