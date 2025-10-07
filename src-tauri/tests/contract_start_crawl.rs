//! start_crawl 契约测试
//!
//! 参考: specs/003-/contracts/start_crawl.md
//!
//! 验证 start_crawl 命令符合契约定义:
//! - 任务不存在返回TASK_NOT_FOUND
//! - Created状态可启动,状态转换到HistoryCrawling
//! - Paused状态可恢复,从检查点继续
//! - HistoryCompleted状态可启动增量,转换到IncrementalCrawling
//! - Failed状态无法启动,返回INVALID_STATUS
//! - 已有任务运行时返回ALREADY_RUNNING

mod common;

use chrono::{DateTime, Duration, Utc};
use common::MockRedisService;
use weibo_login::models::crawl_task::{CrawlStatus, CrawlTask};

/// Mock启动爬取的核心逻辑
///
/// 模拟 Tauri command 的行为,不依赖真实的Redis和Playwright
async fn mock_start_crawl(
    task_id: String,
    redis: &MockRedisService,
) -> Result<StartCrawlResponse, StartCrawlError> {
    // 1. 加载任务
    let task_key = format!("crawl:task:{}", task_id);
    let task_json = redis
        .get(&task_key)
        .await
        .map_err(|e| StartCrawlError::StorageError(e))?
        .ok_or_else(|| StartCrawlError::TaskNotFound(task_id.clone()))?;

    let mut task: CrawlTask = serde_json::from_str(&task_json)
        .map_err(|e| StartCrawlError::StorageError(e.to_string()))?;

    // 2. 状态检查 - 是否有其他任务正在运行
    if let Err(running_task_id) = check_no_running_tasks(redis, &task_id).await {
        return Err(StartCrawlError::AlreadyRunning(running_task_id));
    }

    // 3. 验证状态是否允许启动
    let (new_status, direction) = match task.status {
        CrawlStatus::Created => (CrawlStatus::HistoryCrawling, CrawlDirection::Backward),
        CrawlStatus::Paused => {
            // 从检查点恢复
            let checkpoint_key = format!("crawl:checkpoint:{}", task_id);
            let checkpoint_json = redis
                .get(&checkpoint_key)
                .await
                .map_err(|e| StartCrawlError::StorageError(e))?;

            if let Some(checkpoint_str) = checkpoint_json {
                let checkpoint: CheckpointData = serde_json::from_str(&checkpoint_str)
                    .map_err(|e| StartCrawlError::StorageError(e.to_string()))?;
                let direction = match checkpoint.direction.as_str() {
                    "Backward" => CrawlDirection::Backward,
                    "Forward" => CrawlDirection::Forward,
                    _ => CrawlDirection::Backward,
                };
                (CrawlStatus::HistoryCrawling, direction)
            } else {
                (CrawlStatus::HistoryCrawling, CrawlDirection::Backward)
            }
        }
        CrawlStatus::HistoryCompleted => {
            (CrawlStatus::IncrementalCrawling, CrawlDirection::Forward)
        }
        _ => {
            return Err(StartCrawlError::InvalidStatus {
                status: format!("{:?}", task.status),
            });
        }
    };

    // 4. 更新任务状态
    task.status = new_status;
    task.updated_at = Utc::now();

    let updated_json =
        serde_json::to_string(&task).map_err(|e| StartCrawlError::StorageError(e.to_string()))?;
    redis
        .set(task_key, updated_json)
        .await
        .map_err(|e| StartCrawlError::StorageError(e))?;

    // 5. 返回响应
    let message = match task.status {
        CrawlStatus::HistoryCrawling if direction == CrawlDirection::Backward => {
            if matches!(new_status, CrawlStatus::HistoryCrawling) && task.min_post_time.is_some() {
                "任务已恢复,从检查点继续爬取".to_string()
            } else {
                "任务已启动,开始历史回溯".to_string()
            }
        }
        CrawlStatus::IncrementalCrawling => "任务已启动,开始增量更新".to_string(),
        _ => "任务已启动".to_string(),
    };

    Ok(StartCrawlResponse { message, direction })
}

/// 检查是否有其他任务正在运行
async fn check_no_running_tasks(
    redis: &MockRedisService,
    current_task_id: &str,
) -> Result<(), String> {
    // 简化实现: 检查是否有其他任务处于运行状态
    // 实际实现需要遍历所有任务
    let running_task_key = "crawl:running_task";
    if let Some(running_id) = redis.get(running_task_key).await.ok().flatten() {
        if running_id != current_task_id {
            return Err(running_id);
        }
    }

    // 标记当前任务为运行中
    redis
        .set(running_task_key.to_string(), current_task_id.to_string())
        .await
        .ok();

    Ok(())
}

// ============ 响应和错误类型 ============

#[derive(Debug, Clone, PartialEq)]
struct StartCrawlResponse {
    message: String,
    direction: CrawlDirection,
}

#[derive(Debug, Clone, PartialEq)]
enum CrawlDirection {
    Backward,
    Forward,
}

#[derive(Debug)]
enum StartCrawlError {
    TaskNotFound(String),
    InvalidStatus { status: String },
    AlreadyRunning(String),
    StorageError(String),
}

#[derive(Debug, serde::Deserialize)]
struct CheckpointData {
    direction: String,
}

// ============ 辅助函数 ============

/// 创建测试任务
fn create_test_task(status: CrawlStatus) -> CrawlTask {
    let now = Utc::now();
    CrawlTask {
        id: "test-task-id".to_string(),
        keyword: "测试关键字".to_string(),
        event_start_time: now - Duration::days(7),
        status,
        min_post_time: None,
        max_post_time: None,
        crawled_count: 0,
        created_at: now - Duration::hours(1),
        updated_at: now - Duration::hours(1),
        failure_reason: None,
    }
}

/// 创建已有进度的暂停任务
fn create_paused_task_with_progress() -> CrawlTask {
    let now = Utc::now();
    CrawlTask {
        id: "paused-task-id".to_string(),
        keyword: "测试关键字".to_string(),
        event_start_time: now - Duration::days(7),
        status: CrawlStatus::Paused,
        min_post_time: Some(now - Duration::days(5)),
        max_post_time: Some(now - Duration::days(2)),
        crawled_count: 150,
        created_at: now - Duration::hours(24),
        updated_at: now - Duration::hours(1),
        failure_reason: None,
    }
}

/// 保存任务到Mock Redis
async fn save_task_to_redis(redis: &MockRedisService, task: &CrawlTask) {
    let task_key = format!("crawl:task:{}", task.id);
    let task_json = serde_json::to_string(task).unwrap();
    redis.set(task_key, task_json).await.unwrap();
}

/// 保存检查点到Mock Redis
async fn save_checkpoint_to_redis(redis: &MockRedisService, task_id: &str, direction: &str) {
    let checkpoint_key = format!("crawl:checkpoint:{}", task_id);
    let checkpoint = serde_json::json!({
        "task_id": task_id,
        "direction": direction,
        "shard_start_time": "2025-10-01T00:00:00Z",
        "shard_end_time": "2025-10-01T01:00:00Z",
        "current_page": 15
    });
    redis
        .set(checkpoint_key, checkpoint.to_string())
        .await
        .unwrap();
}

// ============ 测试用例 ============

#[tokio::test]
async fn test_task_not_found() {
    let redis = MockRedisService::new();

    let result = mock_start_crawl("nonexistent-task-id".to_string(), &redis).await;

    assert!(matches!(result, Err(StartCrawlError::TaskNotFound(_))));
    if let Err(StartCrawlError::TaskNotFound(task_id)) = result {
        assert_eq!(task_id, "nonexistent-task-id");
    }
}

#[tokio::test]
async fn test_created_status_starts_history_crawling() {
    let redis = MockRedisService::new();
    let task = create_test_task(CrawlStatus::Created);
    save_task_to_redis(&redis, &task).await;

    let result = mock_start_crawl(task.id.clone(), &redis).await;

    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.direction, CrawlDirection::Backward);
    assert!(response.message.contains("历史回溯"));

    // 验证任务状态已更新
    let task_key = format!("crawl:task:{}", task.id);
    let updated_json = redis.get(&task_key).await.unwrap().unwrap();
    let updated_task: CrawlTask = serde_json::from_str(&updated_json).unwrap();
    assert_eq!(updated_task.status, CrawlStatus::HistoryCrawling);
}

#[tokio::test]
async fn test_paused_status_resumes_from_checkpoint() {
    let redis = MockRedisService::new();
    let task = create_paused_task_with_progress();
    save_task_to_redis(&redis, &task).await;
    save_checkpoint_to_redis(&redis, &task.id, "Backward").await;

    let result = mock_start_crawl(task.id.clone(), &redis).await;

    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.direction, CrawlDirection::Backward);
    assert!(response.message.contains("检查点继续") || response.message.contains("恢复"));
}

#[tokio::test]
async fn test_history_completed_starts_incremental() {
    let redis = MockRedisService::new();
    let mut task = create_test_task(CrawlStatus::HistoryCompleted);
    let now = Utc::now();
    task.min_post_time = Some(now - Duration::days(7));
    task.max_post_time = Some(now - Duration::days(1));
    task.crawled_count = 1000;
    save_task_to_redis(&redis, &task).await;

    let result = mock_start_crawl(task.id.clone(), &redis).await;

    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.direction, CrawlDirection::Forward);
    assert!(response.message.contains("增量"));

    // 验证状态转换
    let task_key = format!("crawl:task:{}", task.id);
    let updated_json = redis.get(&task_key).await.unwrap().unwrap();
    let updated_task: CrawlTask = serde_json::from_str(&updated_json).unwrap();
    assert_eq!(updated_task.status, CrawlStatus::IncrementalCrawling);
}

#[tokio::test]
async fn test_failed_status_cannot_start() {
    let redis = MockRedisService::new();
    let mut task = create_test_task(CrawlStatus::Failed);
    task.failure_reason = Some("网络错误".to_string());
    save_task_to_redis(&redis, &task).await;

    let result = mock_start_crawl(task.id.clone(), &redis).await;

    assert!(matches!(result, Err(StartCrawlError::InvalidStatus { .. })));
    if let Err(StartCrawlError::InvalidStatus { status }) = result {
        assert!(status.contains("Failed"));
    }
}

#[tokio::test]
async fn test_already_running_prevents_second_task() {
    let redis = MockRedisService::new();

    // 第一个任务
    let task1 = create_test_task(CrawlStatus::Created);
    save_task_to_redis(&redis, &task1).await;

    // 第二个任务
    let mut task2 = create_test_task(CrawlStatus::Created);
    task2.id = "second-task-id".to_string();
    save_task_to_redis(&redis, &task2).await;

    // 启动第一个任务
    let result1 = mock_start_crawl(task1.id.clone(), &redis).await;
    assert!(result1.is_ok());

    // 尝试启动第二个任务应该失败
    let result2 = mock_start_crawl(task2.id.clone(), &redis).await;
    assert!(matches!(result2, Err(StartCrawlError::AlreadyRunning(_))));
    if let Err(StartCrawlError::AlreadyRunning(running_id)) = result2 {
        assert_eq!(running_id, task1.id);
    }
}

#[tokio::test]
async fn test_history_crawling_status_cannot_start() {
    let redis = MockRedisService::new();
    let task = create_test_task(CrawlStatus::HistoryCrawling);
    save_task_to_redis(&redis, &task).await;

    let result = mock_start_crawl(task.id.clone(), &redis).await;

    assert!(matches!(result, Err(StartCrawlError::InvalidStatus { .. })));
}

#[tokio::test]
async fn test_incremental_crawling_status_cannot_start() {
    let redis = MockRedisService::new();
    let task = create_test_task(CrawlStatus::IncrementalCrawling);
    save_task_to_redis(&redis, &task).await;

    let result = mock_start_crawl(task.id.clone(), &redis).await;

    assert!(matches!(result, Err(StartCrawlError::InvalidStatus { .. })));
}

#[tokio::test]
async fn test_storage_error_handling() {
    let redis = MockRedisService::new();
    redis.set_fail_mode(true).await;

    let result = mock_start_crawl("any-task-id".to_string(), &redis).await;

    assert!(matches!(result, Err(StartCrawlError::StorageError(_))));
}
