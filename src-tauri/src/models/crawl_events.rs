//! 爬取事件模型
//!
//! 定义爬取过程中通过Tauri事件推送的三种事件:
//! - CrawlProgressEvent: 实时进度更新
//! - CrawlCompletedEvent: 完成通知
//! - CrawlErrorEvent: 错误通知

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 爬取进度事件
///
/// 每页爬取完成后推送,用于实时展示任务进度
///
/// 每个字段不可替代:
/// - task_id: 标识所属任务
/// - status: 当前状态 (HistoryCrawling/IncrementalCrawling)
/// - current_time_range: 当前正在爬取的时间分片
/// - current_page: 当前页码,展示细粒度进度
/// - crawled_count: 累计爬取数,展示总体进度
/// - timestamp: 事件时间,用于前端排序和去重
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrawlProgressEvent {
    pub task_id: String,
    pub status: CrawlStatus,
    pub current_time_range: TimeRange,
    pub current_page: u32,
    pub crawled_count: u64,
    pub timestamp: DateTime<Utc>,
}

impl CrawlProgressEvent {
    /// 创建新的进度事件
    pub fn new(
        task_id: String,
        status: CrawlStatus,
        current_time_range: TimeRange,
        current_page: u32,
        crawled_count: u64,
    ) -> Self {
        Self {
            task_id,
            status,
            current_time_range,
            current_page,
            crawled_count,
            timestamp: Utc::now(),
        }
    }
}

/// 爬取完成事件
///
/// 历史回溯或增量更新完成时推送
///
/// 每个字段不可替代:
/// - task_id: 标识所属任务
/// - final_status: 最终状态 (HistoryCompleted/IncrementalCrawling)
/// - total_crawled: 总爬取数,用于统计汇总
/// - duration: 耗时秒数,性能监控指标
/// - timestamp: 事件时间
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrawlCompletedEvent {
    pub task_id: String,
    pub final_status: CrawlStatus,
    pub total_crawled: u64,
    pub duration: u64,
    pub timestamp: DateTime<Utc>,
}

impl CrawlCompletedEvent {
    /// 创建新的完成事件
    pub fn new(
        task_id: String,
        final_status: CrawlStatus,
        total_crawled: u64,
        duration: u64,
    ) -> Self {
        Self {
            task_id,
            final_status,
            total_crawled,
            duration,
            timestamp: Utc::now(),
        }
    }
}

/// 爬取错误事件
///
/// 检测到验证码、网络错误或存储错误时推送
///
/// 每个字段不可替代:
/// - task_id: 标识所属任务
/// - error: 错误描述,用于用户展示
/// - error_code: 错误类型,用于程序化处理
/// - timestamp: 事件时间
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrawlErrorEvent {
    pub task_id: String,
    pub error: String,
    pub error_code: ErrorCode,
    pub timestamp: DateTime<Utc>,
}

impl CrawlErrorEvent {
    /// 创建新的错误事件
    pub fn new(task_id: String, error: String, error_code: ErrorCode) -> Self {
        Self {
            task_id,
            error,
            error_code,
            timestamp: Utc::now(),
        }
    }
}

/// 时间范围
///
/// ISO 8601格式的时间区间,表示当前正在爬取的时间分片
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeRange {
    /// 开始时间 (ISO 8601)
    pub start: DateTime<Utc>,

    /// 结束时间 (ISO 8601)
    pub end: DateTime<Utc>,
}

impl TimeRange {
    /// 创建新的时间范围
    pub fn new(start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        Self { start, end }
    }
}

/// 爬取状态
///
/// 用于事件中表示任务当前状态
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrawlStatus {
    /// 历史回溯中
    HistoryCrawling,

    /// 历史回溯完成
    HistoryCompleted,

    /// 增量更新中
    IncrementalCrawling,
}

/// 错误类型
///
/// 三种可恢复的错误类型,对应不同的处理策略
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    /// 检测到验证码 (需要人工处理)
    CaptchaDetected,

    /// 网络错误 (可以重试)
    NetworkError,

    /// 存储错误 (Redis异常)
    StorageError,
}
