//! 爬取任务模型
//!
//! 表示一次关键字爬取任务的完整生命周期

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 爬取任务
///
/// 每个字段都不可替代:
/// - id: 唯一标识任务,支持并发管理多个任务
/// - keyword: 搜索关键字,决定爬取内容
/// - event_start_time: 历史回溯的起点,定义时间范围
/// - status: 状态机的当前状态,决定可执行的操作
/// - 时间统计: 支持断点续爬和增量更新
/// - 计数器: 实时进度展示
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlTask {
    /// 任务ID (UUID v4)
    pub id: String,

    /// 搜索关键字
    pub keyword: String,

    /// 事件开始时间 (历史回溯的终点)
    pub event_start_time: DateTime<Utc>,

    /// 任务状态
    pub status: CrawlStatus,

    /// 已爬取的最小帖子时间 (向下取整到小时)
    /// None表示尚未爬取任何帖子
    pub min_post_time: Option<DateTime<Utc>>,

    /// 已爬取的最大帖子时间 (向上取整到小时)
    /// None表示尚未爬取任何帖子
    pub max_post_time: Option<DateTime<Utc>>,

    /// 已爬取帖子总数
    pub crawled_count: u64,

    /// 任务创建时间
    pub created_at: DateTime<Utc>,

    /// 最后更新时间 (每次状态变化或爬取进度更新时刷新)
    pub updated_at: DateTime<Utc>,

    /// 失败原因 (仅当status=Failed时有值)
    pub failure_reason: Option<String>,
}

/// 爬取任务状态
///
/// 状态转换规则:
/// Created → HistoryCrawling → HistoryCompleted → IncrementalCrawling
///         ↘ Paused ↔ (恢复到上一个活跃状态)
///         ↘ Failed (终态,可手动重试)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrawlStatus {
    /// 已创建,未开始
    Created,

    /// 历史回溯中 (从现在到event_start_time)
    HistoryCrawling,

    /// 历史回溯完成
    HistoryCompleted,

    /// 增量更新中 (从max_post_time到现在)
    IncrementalCrawling,

    /// 已暂停 (用户主动暂停或检测到验证码)
    Paused,

    /// 失败 (网络错误/Redis错误等)
    Failed,
}

impl CrawlTask {
    /// 创建新任务
    pub fn new(keyword: String, event_start_time: DateTime<Utc>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            keyword,
            event_start_time,
            status: CrawlStatus::Created,
            min_post_time: None,
            max_post_time: None,
            crawled_count: 0,
            created_at: now,
            updated_at: now,
            failure_reason: None,
        }
    }

    /// 更新爬取进度 (调用时自动刷新updated_at)
    pub fn update_progress(&mut self, post_time: DateTime<Utc>, post_count: u64) {
        self.min_post_time = Some(
            self.min_post_time
                .map(|t| t.min(post_time))
                .unwrap_or(post_time),
        );
        self.max_post_time = Some(
            self.max_post_time
                .map(|t| t.max(post_time))
                .unwrap_or(post_time),
        );
        self.crawled_count += post_count;
        self.updated_at = Utc::now();
    }

    /// 状态转换 (带验证)
    pub fn transition_to(&mut self, new_status: CrawlStatus) -> Result<(), String> {
        if !self.status.can_transition_to(&new_status) {
            return Err(format!(
                "无效的状态转换: {} -> {}",
                self.status.as_str(),
                new_status.as_str()
            ));
        }
        self.status = new_status;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// 标记失败
    pub fn mark_failed(&mut self, reason: String) {
        self.status = CrawlStatus::Failed;
        self.failure_reason = Some(reason);
        self.updated_at = Utc::now();
    }

    /// 验证任务数据完整性
    pub fn validate(&self) -> Result<(), String> {
        // 1. 关键字不能为空
        if self.keyword.trim().is_empty() {
            return Err("关键字不能为空".to_string());
        }

        // 2. 事件开始时间不能晚于当前时间
        if self.event_start_time > Utc::now() {
            return Err("事件开始时间不能是未来时间".to_string());
        }

        // 3. min_post_time必须 <= max_post_time
        if let (Some(min), Some(max)) = (self.min_post_time, self.max_post_time) {
            if min > max {
                return Err("min_post_time不能大于max_post_time".to_string());
            }
        }

        // 4. 状态为Failed时必须有失败原因
        if self.status == CrawlStatus::Failed && self.failure_reason.is_none() {
            return Err("失败状态必须包含失败原因".to_string());
        }

        Ok(())
    }

    /// Redis存储键
    pub fn redis_key(&self) -> String {
        format!("crawl:task:{}", self.id)
    }
}

impl CrawlStatus {
    /// 检查是否可以转换到目标状态
    pub fn can_transition_to(&self, target: &CrawlStatus) -> bool {
        use CrawlStatus::*;
        matches!(
            (self, target),
            (Created, HistoryCrawling)
                | (HistoryCrawling, HistoryCompleted)
                | (HistoryCrawling, Paused)
                | (HistoryCrawling, Failed)
                | (HistoryCompleted, IncrementalCrawling)
                | (IncrementalCrawling, Paused)
                | (IncrementalCrawling, Failed)
                | (Paused, HistoryCrawling)
                | (Paused, IncrementalCrawling)
                | (Failed, HistoryCrawling) // 允许手动重试
        )
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Created => "Created",
            Self::HistoryCrawling => "HistoryCrawling",
            Self::HistoryCompleted => "HistoryCompleted",
            Self::IncrementalCrawling => "IncrementalCrawling",
            Self::Paused => "Paused",
            Self::Failed => "Failed",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_new_task_creation() {
        let keyword = "国庆".to_string();
        let event_start_time = Utc::now() - Duration::days(7);

        let task = CrawlTask::new(keyword.clone(), event_start_time);

        assert_eq!(task.keyword, keyword);
        assert_eq!(task.event_start_time, event_start_time);
        assert_eq!(task.status, CrawlStatus::Created);
        assert_eq!(task.crawled_count, 0);
        assert!(task.min_post_time.is_none());
        assert!(task.max_post_time.is_none());
        assert!(task.failure_reason.is_none());
        assert!(!task.id.is_empty());
    }

    #[test]
    fn test_update_progress() {
        let mut task = CrawlTask::new("测试".to_string(), Utc::now() - Duration::days(1));

        let post_time1 = Utc::now() - Duration::hours(5);
        task.update_progress(post_time1, 10);

        assert_eq!(task.crawled_count, 10);
        assert_eq!(task.min_post_time, Some(post_time1));
        assert_eq!(task.max_post_time, Some(post_time1));

        let post_time2 = Utc::now() - Duration::hours(3);
        task.update_progress(post_time2, 5);

        assert_eq!(task.crawled_count, 15);
        assert_eq!(task.min_post_time, Some(post_time1));
        assert_eq!(task.max_post_time, Some(post_time2));
    }

    #[test]
    fn test_valid_state_transitions() {
        let mut task = CrawlTask::new("测试".to_string(), Utc::now() - Duration::days(1));

        // Created -> HistoryCrawling
        assert!(task.transition_to(CrawlStatus::HistoryCrawling).is_ok());
        assert_eq!(task.status, CrawlStatus::HistoryCrawling);

        // HistoryCrawling -> HistoryCompleted
        assert!(task.transition_to(CrawlStatus::HistoryCompleted).is_ok());
        assert_eq!(task.status, CrawlStatus::HistoryCompleted);

        // HistoryCompleted -> IncrementalCrawling
        assert!(task.transition_to(CrawlStatus::IncrementalCrawling).is_ok());
        assert_eq!(task.status, CrawlStatus::IncrementalCrawling);
    }

    #[test]
    fn test_invalid_state_transitions() {
        let mut task = CrawlTask::new("测试".to_string(), Utc::now() - Duration::days(1));

        // Created -> HistoryCompleted (无效)
        let result = task.transition_to(CrawlStatus::HistoryCompleted);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("无效的状态转换"));

        // Created -> IncrementalCrawling (无效)
        let result = task.transition_to(CrawlStatus::IncrementalCrawling);
        assert!(result.is_err());
    }

    #[test]
    fn test_pause_and_resume() {
        let mut task = CrawlTask::new("测试".to_string(), Utc::now() - Duration::days(1));

        task.transition_to(CrawlStatus::HistoryCrawling).unwrap();

        // 暂停
        assert!(task.transition_to(CrawlStatus::Paused).is_ok());
        assert_eq!(task.status, CrawlStatus::Paused);

        // 恢复
        assert!(task.transition_to(CrawlStatus::HistoryCrawling).is_ok());
        assert_eq!(task.status, CrawlStatus::HistoryCrawling);
    }

    #[test]
    fn test_mark_failed() {
        let mut task = CrawlTask::new("测试".to_string(), Utc::now() - Duration::days(1));

        task.mark_failed("网络错误".to_string());

        assert_eq!(task.status, CrawlStatus::Failed);
        assert_eq!(task.failure_reason, Some("网络错误".to_string()));
    }

    #[test]
    fn test_retry_after_failure() {
        let mut task = CrawlTask::new("测试".to_string(), Utc::now() - Duration::days(1));

        task.transition_to(CrawlStatus::HistoryCrawling).unwrap();
        task.mark_failed("网络错误".to_string());

        // 失败后可以重试
        assert!(task.transition_to(CrawlStatus::HistoryCrawling).is_ok());
        assert_eq!(task.status, CrawlStatus::HistoryCrawling);
    }

    #[test]
    fn test_validate_empty_keyword() {
        let mut task = CrawlTask::new("测试".to_string(), Utc::now() - Duration::days(1));
        assert!(task.validate().is_ok());

        task.keyword = "".to_string();
        let result = task.validate();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "关键字不能为空");

        task.keyword = "   ".to_string();
        let result = task.validate();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "关键字不能为空");
    }

    #[test]
    fn test_validate_future_time() {
        let task = CrawlTask::new("测试".to_string(), Utc::now() + Duration::days(1));

        let result = task.validate();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "事件开始时间不能是未来时间");
    }

    #[test]
    fn test_validate_min_max_post_time() {
        let mut task = CrawlTask::new("测试".to_string(), Utc::now() - Duration::days(1));

        task.min_post_time = Some(Utc::now());
        task.max_post_time = Some(Utc::now() - Duration::hours(1));

        let result = task.validate();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "min_post_time不能大于max_post_time");
    }

    #[test]
    fn test_validate_failed_without_reason() {
        let mut task = CrawlTask::new("测试".to_string(), Utc::now() - Duration::days(1));

        task.status = CrawlStatus::Failed;
        task.failure_reason = None;

        let result = task.validate();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "失败状态必须包含失败原因");
    }

    #[test]
    fn test_redis_key() {
        let task = CrawlTask::new("测试".to_string(), Utc::now() - Duration::days(1));

        let key = task.redis_key();
        assert!(key.starts_with("crawl:task:"));
        assert!(key.contains(&task.id));
    }

    #[test]
    fn test_status_as_str() {
        assert_eq!(CrawlStatus::Created.as_str(), "Created");
        assert_eq!(CrawlStatus::HistoryCrawling.as_str(), "HistoryCrawling");
        assert_eq!(CrawlStatus::HistoryCompleted.as_str(), "HistoryCompleted");
        assert_eq!(
            CrawlStatus::IncrementalCrawling.as_str(),
            "IncrementalCrawling"
        );
        assert_eq!(CrawlStatus::Paused.as_str(), "Paused");
        assert_eq!(CrawlStatus::Failed.as_str(), "Failed");
    }
}
