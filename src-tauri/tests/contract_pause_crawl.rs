//! pause_crawl 契约测试
//!
//! 参考: specs/003-/contracts/pause_crawl.md
//!
//! 验证 pause_crawl 命令符合契约定义,包括:
//! - 任务不存在返回TASK_NOT_FOUND
//! - HistoryCrawling状态可暂停
//! - IncrementalCrawling状态可暂停
//! - Created状态无法暂停,返回INVALID_STATUS
//! - 暂停时保存检查点到Redis

mod common;

use chrono::Utc;
use common::MockRedisService;
use weibo_login::models::crawl_checkpoint::CrawlCheckpoint;
use weibo_login::models::crawl_task::{CrawlStatus, CrawlTask};

/// pause_crawl 响应
#[derive(Debug, Clone, PartialEq)]
struct PauseCrawlResponse {
    message: String,
    checkpoint: CheckpointInfo,
}

#[derive(Debug, Clone, PartialEq)]
struct CheckpointInfo {
    shard_start_time: String,
    shard_end_time: String,
    current_page: u32,
    crawled_count: u64,
}

/// pause_crawl 错误
#[derive(Debug)]
enum PauseCrawlError {
    TaskNotFound(String),
    InvalidStatus { status: String, message: String },
    StorageError(String),
}

/// Mock暂停爬取命令
///
/// 模拟 Tauri command 的行为,验证契约定义
async fn mock_pause_crawl(
    task_id: String,
    redis: &MockRedisService,
) -> Result<PauseCrawlResponse, PauseCrawlError> {
    // 1. 加载任务
    let task_key = format!("crawl:task:{}", task_id);
    let task_exists = redis
        .exists(&task_key)
        .await
        .map_err(|e| PauseCrawlError::StorageError(e))?;

    if !task_exists {
        return Err(PauseCrawlError::TaskNotFound(format!(
            "任务 {} 不存在",
            task_id
        )));
    }

    // 2. 获取任务状态
    let status_str = redis
        .hget(&task_key, "status")
        .await
        .map_err(|e| PauseCrawlError::StorageError(e))?
        .ok_or_else(|| PauseCrawlError::StorageError("任务状态缺失".to_string()))?;

    // 3. 状态检查
    match status_str.as_str() {
        "HistoryCrawling" | "IncrementalCrawling" => {
            // 允许暂停
        }
        _ => {
            return Err(PauseCrawlError::InvalidStatus {
                status: status_str.clone(),
                message: format!(
                    "任务状态 {} 无法暂停,仅支持HistoryCrawling/IncrementalCrawling",
                    status_str
                ),
            });
        }
    }

    // 4. 获取检查点信息
    let checkpoint_key = format!("crawl:checkpoint:{}", task_id);
    let shard_start_time = redis
        .hget(&checkpoint_key, "shard_start_time")
        .await
        .map_err(|e| PauseCrawlError::StorageError(e))?
        .unwrap_or_else(|| Utc::now().to_rfc3339());

    let shard_end_time = redis
        .hget(&checkpoint_key, "shard_end_time")
        .await
        .map_err(|e| PauseCrawlError::StorageError(e))?
        .unwrap_or_else(|| Utc::now().to_rfc3339());

    let current_page = redis
        .hget(&checkpoint_key, "current_page")
        .await
        .map_err(|e| PauseCrawlError::StorageError(e))?
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(1);

    // 获取已爬取数量
    let crawled_count = redis
        .hget(&task_key, "crawled_count")
        .await
        .map_err(|e| PauseCrawlError::StorageError(e))?
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);

    // 5. 状态转换到Paused
    redis
        .hset(&task_key, "status", "Paused".to_string())
        .await
        .map_err(|e| PauseCrawlError::StorageError(e))?;

    redis
        .hset(&task_key, "updated_at", Utc::now().timestamp().to_string())
        .await
        .map_err(|e| PauseCrawlError::StorageError(e))?;

    // 6. 保存检查点 (确保Redis中有检查点)
    redis
        .hset(
            &checkpoint_key,
            "shard_start_time",
            shard_start_time.clone(),
        )
        .await
        .map_err(|e| PauseCrawlError::StorageError(e))?;

    redis
        .hset(&checkpoint_key, "shard_end_time", shard_end_time.clone())
        .await
        .map_err(|e| PauseCrawlError::StorageError(e))?;

    redis
        .hset(&checkpoint_key, "current_page", current_page.to_string())
        .await
        .map_err(|e| PauseCrawlError::StorageError(e))?;

    redis
        .hset(
            &checkpoint_key,
            "saved_at",
            Utc::now().timestamp().to_string(),
        )
        .await
        .map_err(|e| PauseCrawlError::StorageError(e))?;

    // 7. 返回响应
    Ok(PauseCrawlResponse {
        message: "任务已暂停,可通过start_crawl恢复".to_string(),
        checkpoint: CheckpointInfo {
            shard_start_time,
            shard_end_time,
            current_page,
            crawled_count,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 辅助函数: 创建测试任务
    async fn create_test_task(
        redis: &MockRedisService,
        task_id: &str,
        status: CrawlStatus,
        crawled_count: u64,
    ) {
        let task_key = format!("crawl:task:{}", task_id);

        redis
            .hset(&task_key, "id", task_id.to_string())
            .await
            .unwrap();
        redis
            .hset(&task_key, "keyword", "测试关键字".to_string())
            .await
            .unwrap();
        redis
            .hset(
                &task_key,
                "status",
                match status {
                    CrawlStatus::Created => "Created",
                    CrawlStatus::HistoryCrawling => "HistoryCrawling",
                    CrawlStatus::HistoryCompleted => "HistoryCompleted",
                    CrawlStatus::IncrementalCrawling => "IncrementalCrawling",
                    CrawlStatus::Paused => "Paused",
                    CrawlStatus::Failed => "Failed",
                }
                .to_string(),
            )
            .await
            .unwrap();
        redis
            .hset(&task_key, "crawled_count", crawled_count.to_string())
            .await
            .unwrap();
        redis
            .hset(&task_key, "created_at", Utc::now().timestamp().to_string())
            .await
            .unwrap();
        redis
            .hset(&task_key, "updated_at", Utc::now().timestamp().to_string())
            .await
            .unwrap();
    }

    /// 辅助函数: 创建检查点
    async fn create_test_checkpoint(redis: &MockRedisService, task_id: &str, current_page: u32) {
        let checkpoint_key = format!("crawl:checkpoint:{}", task_id);
        let now = Utc::now();

        redis
            .hset(&checkpoint_key, "shard_start_time", now.to_rfc3339())
            .await
            .unwrap();
        redis
            .hset(&checkpoint_key, "shard_end_time", now.to_rfc3339())
            .await
            .unwrap();
        redis
            .hset(&checkpoint_key, "current_page", current_page.to_string())
            .await
            .unwrap();
    }

    /// 测试: 任务不存在返回TASK_NOT_FOUND
    ///
    /// 契约要求:
    /// - 当任务不存在时,返回错误码TASK_NOT_FOUND
    /// - 错误消息包含任务ID
    #[tokio::test]
    async fn test_pause_task_not_found() {
        let redis = MockRedisService::new();
        let task_id = "non-existent-task-id";

        let result = mock_pause_crawl(task_id.to_string(), &redis).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            PauseCrawlError::TaskNotFound(msg) => {
                assert!(msg.contains(task_id));
                assert!(msg.contains("不存在"));
            }
            _ => panic!("Expected TaskNotFound error"),
        }
    }

    /// 测试: HistoryCrawling状态可暂停
    ///
    /// 契约要求:
    /// - HistoryCrawling状态允许暂停
    /// - 暂停后状态转换为Paused
    /// - 返回检查点信息
    #[tokio::test]
    async fn test_pause_history_crawling() {
        let redis = MockRedisService::new();
        let task_id = "test-task-history";

        create_test_task(&redis, task_id, CrawlStatus::HistoryCrawling, 300).await;
        create_test_checkpoint(&redis, task_id, 15).await;

        let result = mock_pause_crawl(task_id.to_string(), &redis).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.message, "任务已暂停,可通过start_crawl恢复");
        assert_eq!(response.checkpoint.current_page, 15);
        assert_eq!(response.checkpoint.crawled_count, 300);

        // 验证状态已更新
        let task_key = format!("crawl:task:{}", task_id);
        let status = redis.hget(&task_key, "status").await.unwrap().unwrap();
        assert_eq!(status, "Paused");
    }

    /// 测试: IncrementalCrawling状态可暂停
    ///
    /// 契约要求:
    /// - IncrementalCrawling状态允许暂停
    /// - 暂停后状态转换为Paused
    /// - 保存检查点
    #[tokio::test]
    async fn test_pause_incremental_crawling() {
        let redis = MockRedisService::new();
        let task_id = "test-task-incremental";

        create_test_task(&redis, task_id, CrawlStatus::IncrementalCrawling, 500).await;
        create_test_checkpoint(&redis, task_id, 8).await;

        let result = mock_pause_crawl(task_id.to_string(), &redis).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.checkpoint.current_page, 8);
        assert_eq!(response.checkpoint.crawled_count, 500);

        // 验证检查点已保存
        let checkpoint_key = format!("crawl:checkpoint:{}", task_id);
        let saved_page = redis
            .hget(&checkpoint_key, "current_page")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(saved_page, "8");
    }

    /// 测试: Created状态无法暂停
    ///
    /// 契约要求:
    /// - Created状态不允许暂停
    /// - 返回错误码INVALID_STATUS
    /// - 错误消息说明仅支持HistoryCrawling/IncrementalCrawling
    #[tokio::test]
    async fn test_pause_created_status_invalid() {
        let redis = MockRedisService::new();
        let task_id = "test-task-created";

        create_test_task(&redis, task_id, CrawlStatus::Created, 0).await;

        let result = mock_pause_crawl(task_id.to_string(), &redis).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            PauseCrawlError::InvalidStatus { status, message } => {
                assert_eq!(status, "Created");
                assert!(message.contains("无法暂停"));
                assert!(message.contains("HistoryCrawling/IncrementalCrawling"));
            }
            _ => panic!("Expected InvalidStatus error"),
        }
    }

    /// 测试: Paused状态无法暂停
    ///
    /// 契约要求:
    /// - 已暂停的任务不能再次暂停
    /// - 返回INVALID_STATUS错误
    #[tokio::test]
    async fn test_pause_already_paused() {
        let redis = MockRedisService::new();
        let task_id = "test-task-paused";

        create_test_task(&redis, task_id, CrawlStatus::Paused, 200).await;

        let result = mock_pause_crawl(task_id.to_string(), &redis).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            PauseCrawlError::InvalidStatus { status, .. } => {
                assert_eq!(status, "Paused");
            }
            _ => panic!("Expected InvalidStatus error"),
        }
    }

    /// 测试: HistoryCompleted状态无法暂停
    ///
    /// 契约要求:
    /// - 已完成历史爬取的任务不允许暂停
    /// - 返回INVALID_STATUS错误
    #[tokio::test]
    async fn test_pause_history_completed_invalid() {
        let redis = MockRedisService::new();
        let task_id = "test-task-completed";

        create_test_task(&redis, task_id, CrawlStatus::HistoryCompleted, 1000).await;

        let result = mock_pause_crawl(task_id.to_string(), &redis).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            PauseCrawlError::InvalidStatus { status, .. } => {
                assert_eq!(status, "HistoryCompleted");
            }
            _ => panic!("Expected InvalidStatus error"),
        }
    }

    /// 测试: 暂停时保存检查点到Redis
    ///
    /// 契约要求:
    /// - 暂停时必须保存检查点
    /// - 检查点包含时间范围、页码、保存时间
    #[tokio::test]
    async fn test_pause_saves_checkpoint() {
        let redis = MockRedisService::new();
        let task_id = "test-task-checkpoint";

        create_test_task(&redis, task_id, CrawlStatus::HistoryCrawling, 150).await;
        create_test_checkpoint(&redis, task_id, 10).await;

        let result = mock_pause_crawl(task_id.to_string(), &redis).await;

        assert!(result.is_ok());

        // 验证检查点已保存
        let checkpoint_key = format!("crawl:checkpoint:{}", task_id);

        let shard_start = redis
            .hget(&checkpoint_key, "shard_start_time")
            .await
            .unwrap();
        assert!(shard_start.is_some());

        let shard_end = redis.hget(&checkpoint_key, "shard_end_time").await.unwrap();
        assert!(shard_end.is_some());

        let page = redis.hget(&checkpoint_key, "current_page").await.unwrap();
        assert_eq!(page, Some("10".to_string()));

        let saved_at = redis.hget(&checkpoint_key, "saved_at").await.unwrap();
        assert!(saved_at.is_some());
    }

    /// 测试: Redis存储失败
    ///
    /// 契约要求:
    /// - Redis操作失败时返回STORAGE_ERROR
    #[tokio::test]
    async fn test_pause_redis_storage_error() {
        let redis = MockRedisService::new();
        let task_id = "test-task-storage-error";

        create_test_task(&redis, task_id, CrawlStatus::HistoryCrawling, 100).await;
        redis.set_fail_mode(true).await;

        let result = mock_pause_crawl(task_id.to_string(), &redis).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            PauseCrawlError::StorageError(msg) => {
                assert!(msg.contains("Redis"));
            }
            _ => panic!("Expected StorageError"),
        }
    }

    /// 测试: 检查点包含完整信息
    ///
    /// 契约要求:
    /// - 检查点响应包含: shard_start_time, shard_end_time, current_page, crawled_count
    #[tokio::test]
    async fn test_pause_checkpoint_complete_info() {
        let redis = MockRedisService::new();
        let task_id = "test-task-complete-checkpoint";

        create_test_task(&redis, task_id, CrawlStatus::HistoryCrawling, 250).await;
        create_test_checkpoint(&redis, task_id, 20).await;

        let result = mock_pause_crawl(task_id.to_string(), &redis).await;

        assert!(result.is_ok());
        let response = result.unwrap();

        // 验证检查点完整性
        assert!(!response.checkpoint.shard_start_time.is_empty());
        assert!(!response.checkpoint.shard_end_time.is_empty());
        assert_eq!(response.checkpoint.current_page, 20);
        assert_eq!(response.checkpoint.crawled_count, 250);
    }
}
