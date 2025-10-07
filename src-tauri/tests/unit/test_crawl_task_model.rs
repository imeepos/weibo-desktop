//! CrawlTask模型单元测试
//!
//! 覆盖范围:
//! 1. 所有状态转换路径(合法和非法)
//! 2. 验证规则
//! 3. 边界情况

use chrono::{Duration, Utc};
use uuid::Uuid;
use weibo_login::models::crawl_task::{CrawlStatus, CrawlTask};

// ============================================================================
// 1. 状态转换测试 - 合法路径
// ============================================================================

#[test]
fn test_transition_created_to_history_crawling() {
    let mut task = CrawlTask::new("国庆".to_string(), Utc::now() - Duration::days(7));

    let result = task.transition_to(CrawlStatus::HistoryCrawling);

    assert!(result.is_ok());
    assert_eq!(task.status, CrawlStatus::HistoryCrawling);
}

#[test]
fn test_transition_history_crawling_to_history_completed() {
    let mut task = CrawlTask::new("国庆".to_string(), Utc::now() - Duration::days(7));
    task.transition_to(CrawlStatus::HistoryCrawling).unwrap();

    let result = task.transition_to(CrawlStatus::HistoryCompleted);

    assert!(result.is_ok());
    assert_eq!(task.status, CrawlStatus::HistoryCompleted);
}

#[test]
fn test_transition_history_crawling_to_paused() {
    let mut task = CrawlTask::new("国庆".to_string(), Utc::now() - Duration::days(7));
    task.transition_to(CrawlStatus::HistoryCrawling).unwrap();

    let result = task.transition_to(CrawlStatus::Paused);

    assert!(result.is_ok());
    assert_eq!(task.status, CrawlStatus::Paused);
}

#[test]
fn test_transition_history_crawling_to_failed() {
    let mut task = CrawlTask::new("国庆".to_string(), Utc::now() - Duration::days(7));
    task.transition_to(CrawlStatus::HistoryCrawling).unwrap();

    let result = task.transition_to(CrawlStatus::Failed);

    assert!(result.is_ok());
    assert_eq!(task.status, CrawlStatus::Failed);
}

#[test]
fn test_transition_history_completed_to_incremental_crawling() {
    let mut task = CrawlTask::new("国庆".to_string(), Utc::now() - Duration::days(7));
    task.transition_to(CrawlStatus::HistoryCrawling).unwrap();
    task.transition_to(CrawlStatus::HistoryCompleted).unwrap();

    let result = task.transition_to(CrawlStatus::IncrementalCrawling);

    assert!(result.is_ok());
    assert_eq!(task.status, CrawlStatus::IncrementalCrawling);
}

#[test]
fn test_transition_incremental_crawling_to_paused() {
    let mut task = CrawlTask::new("国庆".to_string(), Utc::now() - Duration::days(7));
    task.transition_to(CrawlStatus::HistoryCrawling).unwrap();
    task.transition_to(CrawlStatus::HistoryCompleted).unwrap();
    task.transition_to(CrawlStatus::IncrementalCrawling).unwrap();

    let result = task.transition_to(CrawlStatus::Paused);

    assert!(result.is_ok());
    assert_eq!(task.status, CrawlStatus::Paused);
}

#[test]
fn test_transition_incremental_crawling_to_failed() {
    let mut task = CrawlTask::new("国庆".to_string(), Utc::now() - Duration::days(7));
    task.transition_to(CrawlStatus::HistoryCrawling).unwrap();
    task.transition_to(CrawlStatus::HistoryCompleted).unwrap();
    task.transition_to(CrawlStatus::IncrementalCrawling).unwrap();

    let result = task.transition_to(CrawlStatus::Failed);

    assert!(result.is_ok());
    assert_eq!(task.status, CrawlStatus::Failed);
}

#[test]
fn test_transition_paused_to_history_crawling() {
    let mut task = CrawlTask::new("国庆".to_string(), Utc::now() - Duration::days(7));
    task.transition_to(CrawlStatus::HistoryCrawling).unwrap();
    task.transition_to(CrawlStatus::Paused).unwrap();

    let result = task.transition_to(CrawlStatus::HistoryCrawling);

    assert!(result.is_ok());
    assert_eq!(task.status, CrawlStatus::HistoryCrawling);
}

#[test]
fn test_transition_paused_to_incremental_crawling() {
    let mut task = CrawlTask::new("国庆".to_string(), Utc::now() - Duration::days(7));
    task.transition_to(CrawlStatus::HistoryCrawling).unwrap();
    task.transition_to(CrawlStatus::HistoryCompleted).unwrap();
    task.transition_to(CrawlStatus::IncrementalCrawling).unwrap();
    task.transition_to(CrawlStatus::Paused).unwrap();

    let result = task.transition_to(CrawlStatus::IncrementalCrawling);

    assert!(result.is_ok());
    assert_eq!(task.status, CrawlStatus::IncrementalCrawling);
}

#[test]
fn test_transition_failed_to_history_crawling() {
    let mut task = CrawlTask::new("国庆".to_string(), Utc::now() - Duration::days(7));
    task.mark_failed("网络错误".to_string());

    let result = task.transition_to(CrawlStatus::HistoryCrawling);

    assert!(result.is_ok());
    assert_eq!(task.status, CrawlStatus::HistoryCrawling);
}

// ============================================================================
// 2. 状态转换测试 - 非法路径
// ============================================================================

#[test]
fn test_illegal_transition_created_to_history_completed() {
    let mut task = CrawlTask::new("国庆".to_string(), Utc::now() - Duration::days(7));

    let result = task.transition_to(CrawlStatus::HistoryCompleted);

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("无效的状态转换"));
    assert_eq!(task.status, CrawlStatus::Created);
}

#[test]
fn test_illegal_transition_created_to_incremental_crawling() {
    let mut task = CrawlTask::new("国庆".to_string(), Utc::now() - Duration::days(7));

    let result = task.transition_to(CrawlStatus::IncrementalCrawling);

    assert!(result.is_err());
    assert_eq!(task.status, CrawlStatus::Created);
}

#[test]
fn test_illegal_transition_created_to_paused() {
    let mut task = CrawlTask::new("国庆".to_string(), Utc::now() - Duration::days(7));

    let result = task.transition_to(CrawlStatus::Paused);

    assert!(result.is_err());
    assert_eq!(task.status, CrawlStatus::Created);
}

#[test]
fn test_illegal_transition_created_to_failed() {
    let mut task = CrawlTask::new("国庆".to_string(), Utc::now() - Duration::days(7));

    let result = task.transition_to(CrawlStatus::Failed);

    assert!(result.is_err());
    assert_eq!(task.status, CrawlStatus::Created);
}

#[test]
fn test_illegal_transition_history_completed_to_history_crawling() {
    let mut task = CrawlTask::new("国庆".to_string(), Utc::now() - Duration::days(7));
    task.transition_to(CrawlStatus::HistoryCrawling).unwrap();
    task.transition_to(CrawlStatus::HistoryCompleted).unwrap();

    let result = task.transition_to(CrawlStatus::HistoryCrawling);

    assert!(result.is_err());
    assert_eq!(task.status, CrawlStatus::HistoryCompleted);
}

#[test]
fn test_illegal_transition_history_completed_to_paused() {
    let mut task = CrawlTask::new("国庆".to_string(), Utc::now() - Duration::days(7));
    task.transition_to(CrawlStatus::HistoryCrawling).unwrap();
    task.transition_to(CrawlStatus::HistoryCompleted).unwrap();

    let result = task.transition_to(CrawlStatus::Paused);

    assert!(result.is_err());
    assert_eq!(task.status, CrawlStatus::HistoryCompleted);
}

#[test]
fn test_illegal_transition_incremental_crawling_to_history_crawling() {
    let mut task = CrawlTask::new("国庆".to_string(), Utc::now() - Duration::days(7));
    task.transition_to(CrawlStatus::HistoryCrawling).unwrap();
    task.transition_to(CrawlStatus::HistoryCompleted).unwrap();
    task.transition_to(CrawlStatus::IncrementalCrawling).unwrap();

    let result = task.transition_to(CrawlStatus::HistoryCrawling);

    assert!(result.is_err());
    assert_eq!(task.status, CrawlStatus::IncrementalCrawling);
}

#[test]
fn test_illegal_transition_paused_to_failed() {
    let mut task = CrawlTask::new("国庆".to_string(), Utc::now() - Duration::days(7));
    task.transition_to(CrawlStatus::HistoryCrawling).unwrap();
    task.transition_to(CrawlStatus::Paused).unwrap();

    let result = task.transition_to(CrawlStatus::Failed);

    assert!(result.is_err());
    assert_eq!(task.status, CrawlStatus::Paused);
}

#[test]
fn test_illegal_transition_paused_to_created() {
    let mut task = CrawlTask::new("国庆".to_string(), Utc::now() - Duration::days(7));
    task.transition_to(CrawlStatus::HistoryCrawling).unwrap();
    task.transition_to(CrawlStatus::Paused).unwrap();

    let result = task.transition_to(CrawlStatus::Created);

    assert!(result.is_err());
    assert_eq!(task.status, CrawlStatus::Paused);
}

#[test]
fn test_illegal_transition_failed_to_paused() {
    let mut task = CrawlTask::new("国庆".to_string(), Utc::now() - Duration::days(7));
    task.mark_failed("网络错误".to_string());

    let result = task.transition_to(CrawlStatus::Paused);

    assert!(result.is_err());
    assert_eq!(task.status, CrawlStatus::Failed);
}

// ============================================================================
// 3. 验证规则测试
// ============================================================================

#[test]
fn test_validate_empty_keyword() {
    let mut task = CrawlTask::new("国庆".to_string(), Utc::now() - Duration::days(7));
    assert!(task.validate().is_ok());

    task.keyword = "".to_string();
    let result = task.validate();
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "关键字不能为空");
}

#[test]
fn test_validate_whitespace_keyword() {
    let mut task = CrawlTask::new("国庆".to_string(), Utc::now() - Duration::days(7));

    task.keyword = "   ".to_string();
    let result = task.validate();

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "关键字不能为空");
}

#[test]
fn test_validate_future_event_start_time() {
    let task = CrawlTask::new("国庆".to_string(), Utc::now() + Duration::days(1));

    let result = task.validate();

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "事件开始时间不能是未来时间");
}

#[test]
fn test_validate_min_post_time_greater_than_max() {
    let mut task = CrawlTask::new("国庆".to_string(), Utc::now() - Duration::days(7));

    task.min_post_time = Some(Utc::now());
    task.max_post_time = Some(Utc::now() - Duration::hours(1));

    let result = task.validate();

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "min_post_time不能大于max_post_time");
}

#[test]
fn test_validate_min_post_time_equal_to_max() {
    let mut task = CrawlTask::new("国庆".to_string(), Utc::now() - Duration::days(7));

    let same_time = Utc::now() - Duration::hours(5);
    task.min_post_time = Some(same_time);
    task.max_post_time = Some(same_time);

    let result = task.validate();

    assert!(result.is_ok());
}

#[test]
fn test_validate_failed_without_reason() {
    let mut task = CrawlTask::new("国庆".to_string(), Utc::now() - Duration::days(7));

    task.status = CrawlStatus::Failed;
    task.failure_reason = None;

    let result = task.validate();

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "失败状态必须包含失败原因");
}

#[test]
fn test_validate_failed_with_empty_reason() {
    let mut task = CrawlTask::new("国庆".to_string(), Utc::now() - Duration::days(7));

    task.status = CrawlStatus::Failed;
    task.failure_reason = Some("".to_string());

    let result = task.validate();

    assert!(result.is_ok());
}

// ============================================================================
// 4. 边界情况测试
// ============================================================================

#[test]
fn test_new_task_with_very_long_keyword() {
    let long_keyword = "国".repeat(1000);
    let task = CrawlTask::new(long_keyword.clone(), Utc::now() - Duration::days(7));

    assert_eq!(task.keyword, long_keyword);
    assert!(task.validate().is_ok());
}

#[test]
fn test_new_task_with_very_old_event_start_time() {
    let very_old_time = Utc::now() - Duration::days(3650);
    let task = CrawlTask::new("国庆".to_string(), very_old_time);

    assert_eq!(task.event_start_time, very_old_time);
    assert!(task.validate().is_ok());
}

#[test]
fn test_update_progress_with_same_time_multiple_times() {
    let mut task = CrawlTask::new("国庆".to_string(), Utc::now() - Duration::days(7));

    let post_time = Utc::now() - Duration::hours(5);
    task.update_progress(post_time, 10);
    task.update_progress(post_time, 5);
    task.update_progress(post_time, 3);

    assert_eq!(task.crawled_count, 18);
    assert_eq!(task.min_post_time, Some(post_time));
    assert_eq!(task.max_post_time, Some(post_time));
}

#[test]
fn test_update_progress_with_decreasing_times() {
    let mut task = CrawlTask::new("国庆".to_string(), Utc::now() - Duration::days(7));

    let time1 = Utc::now() - Duration::hours(1);
    let time2 = Utc::now() - Duration::hours(3);
    let time3 = Utc::now() - Duration::hours(5);

    task.update_progress(time1, 10);
    task.update_progress(time2, 5);
    task.update_progress(time3, 3);

    assert_eq!(task.crawled_count, 18);
    assert_eq!(task.min_post_time, Some(time3));
    assert_eq!(task.max_post_time, Some(time1));
}

#[test]
fn test_update_progress_with_zero_count() {
    let mut task = CrawlTask::new("国庆".to_string(), Utc::now() - Duration::days(7));

    let post_time = Utc::now() - Duration::hours(5);
    task.update_progress(post_time, 0);

    assert_eq!(task.crawled_count, 0);
    assert_eq!(task.min_post_time, Some(post_time));
    assert_eq!(task.max_post_time, Some(post_time));
}

#[test]
fn test_mark_failed_updates_updated_at() {
    let mut task = CrawlTask::new("国庆".to_string(), Utc::now() - Duration::days(7));
    let initial_updated_at = task.updated_at;

    std::thread::sleep(std::time::Duration::from_millis(10));
    task.mark_failed("网络错误".to_string());

    assert!(task.updated_at > initial_updated_at);
    assert_eq!(task.status, CrawlStatus::Failed);
    assert_eq!(task.failure_reason, Some("网络错误".to_string()));
}

#[test]
fn test_transition_updates_updated_at() {
    let mut task = CrawlTask::new("国庆".to_string(), Utc::now() - Duration::days(7));
    let initial_updated_at = task.updated_at;

    std::thread::sleep(std::time::Duration::from_millis(10));
    task.transition_to(CrawlStatus::HistoryCrawling).unwrap();

    assert!(task.updated_at > initial_updated_at);
}

#[test]
fn test_redis_key_format() {
    let task = CrawlTask::new("国庆".to_string(), Utc::now() - Duration::days(7));

    let key = task.redis_key();

    assert!(key.starts_with("crawl:task:"));
    assert!(key.contains(&task.id));
    assert_eq!(key, format!("crawl:task:{}", task.id));
}

#[test]
fn test_task_id_is_valid_uuid() {
    let task = CrawlTask::new("国庆".to_string(), Utc::now() - Duration::days(7));

    let uuid_result = Uuid::parse_str(&task.id);

    assert!(uuid_result.is_ok());
}

#[test]
fn test_task_id_is_unique() {
    let task1 = CrawlTask::new("国庆".to_string(), Utc::now() - Duration::days(7));
    let task2 = CrawlTask::new("国庆".to_string(), Utc::now() - Duration::days(7));

    assert_ne!(task1.id, task2.id);
}

#[test]
fn test_new_task_initial_state() {
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
    assert_eq!(task.created_at, task.updated_at);
}

// ============================================================================
// 5. 状态机can_transition_to方法测试
// ============================================================================

#[test]
fn test_can_transition_to_from_created() {
    assert!(CrawlStatus::Created.can_transition_to(&CrawlStatus::HistoryCrawling));
    assert!(!CrawlStatus::Created.can_transition_to(&CrawlStatus::HistoryCompleted));
    assert!(!CrawlStatus::Created.can_transition_to(&CrawlStatus::IncrementalCrawling));
    assert!(!CrawlStatus::Created.can_transition_to(&CrawlStatus::Paused));
    assert!(!CrawlStatus::Created.can_transition_to(&CrawlStatus::Failed));
}

#[test]
fn test_can_transition_to_from_history_crawling() {
    assert!(CrawlStatus::HistoryCrawling.can_transition_to(&CrawlStatus::HistoryCompleted));
    assert!(CrawlStatus::HistoryCrawling.can_transition_to(&CrawlStatus::Paused));
    assert!(CrawlStatus::HistoryCrawling.can_transition_to(&CrawlStatus::Failed));
    assert!(!CrawlStatus::HistoryCrawling.can_transition_to(&CrawlStatus::Created));
    assert!(!CrawlStatus::HistoryCrawling.can_transition_to(&CrawlStatus::IncrementalCrawling));
}

#[test]
fn test_can_transition_to_from_history_completed() {
    assert!(CrawlStatus::HistoryCompleted.can_transition_to(&CrawlStatus::IncrementalCrawling));
    assert!(!CrawlStatus::HistoryCompleted.can_transition_to(&CrawlStatus::Created));
    assert!(!CrawlStatus::HistoryCompleted.can_transition_to(&CrawlStatus::HistoryCrawling));
    assert!(!CrawlStatus::HistoryCompleted.can_transition_to(&CrawlStatus::Paused));
    assert!(!CrawlStatus::HistoryCompleted.can_transition_to(&CrawlStatus::Failed));
}

#[test]
fn test_can_transition_to_from_incremental_crawling() {
    assert!(CrawlStatus::IncrementalCrawling.can_transition_to(&CrawlStatus::Paused));
    assert!(CrawlStatus::IncrementalCrawling.can_transition_to(&CrawlStatus::Failed));
    assert!(!CrawlStatus::IncrementalCrawling.can_transition_to(&CrawlStatus::Created));
    assert!(!CrawlStatus::IncrementalCrawling.can_transition_to(&CrawlStatus::HistoryCrawling));
    assert!(!CrawlStatus::IncrementalCrawling.can_transition_to(&CrawlStatus::HistoryCompleted));
}

#[test]
fn test_can_transition_to_from_paused() {
    assert!(CrawlStatus::Paused.can_transition_to(&CrawlStatus::HistoryCrawling));
    assert!(CrawlStatus::Paused.can_transition_to(&CrawlStatus::IncrementalCrawling));
    assert!(!CrawlStatus::Paused.can_transition_to(&CrawlStatus::Created));
    assert!(!CrawlStatus::Paused.can_transition_to(&CrawlStatus::HistoryCompleted));
    assert!(!CrawlStatus::Paused.can_transition_to(&CrawlStatus::Failed));
}

#[test]
fn test_can_transition_to_from_failed() {
    assert!(CrawlStatus::Failed.can_transition_to(&CrawlStatus::HistoryCrawling));
    assert!(!CrawlStatus::Failed.can_transition_to(&CrawlStatus::Created));
    assert!(!CrawlStatus::Failed.can_transition_to(&CrawlStatus::HistoryCompleted));
    assert!(!CrawlStatus::Failed.can_transition_to(&CrawlStatus::IncrementalCrawling));
    assert!(!CrawlStatus::Failed.can_transition_to(&CrawlStatus::Paused));
}

// ============================================================================
// 6. CrawlStatus::as_str方法测试
// ============================================================================

#[test]
fn test_status_as_str() {
    assert_eq!(CrawlStatus::Created.as_str(), "Created");
    assert_eq!(CrawlStatus::HistoryCrawling.as_str(), "HistoryCrawling");
    assert_eq!(CrawlStatus::HistoryCompleted.as_str(), "HistoryCompleted");
    assert_eq!(CrawlStatus::IncrementalCrawling.as_str(), "IncrementalCrawling");
    assert_eq!(CrawlStatus::Paused.as_str(), "Paused");
    assert_eq!(CrawlStatus::Failed.as_str(), "Failed");
}
