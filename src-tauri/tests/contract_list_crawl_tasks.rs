//! list_crawl_tasks 契约测试
//!
//! 参考: specs/003-/contracts/list_crawl_tasks.md
//!
//! 验证 list_crawl_tasks 命令符合契约定义:
//! - 无任务时返回空数组
//! - 返回所有任务摘要信息
//! - 按状态过滤正确
//! - 按创建时间排序正确
//! - 所有时间字段使用ISO 8601格式

mod common;

use chrono::{DateTime, Utc};
use common::MockRedisService;

/// 爬取任务摘要
#[derive(Debug, Clone, PartialEq)]
struct CrawlTaskSummary {
    task_id: String,
    keyword: String,
    status: String,
    event_start_time: String,
    crawled_count: u64,
    created_at: String,
    updated_at: String,
    failure_reason: Option<String>,
}

/// 列出任务的响应
#[derive(Debug, Clone, PartialEq)]
struct ListCrawlTasksResponse {
    tasks: Vec<CrawlTaskSummary>,
    total: usize,
}

/// 列出任务的请求
#[derive(Debug, Clone)]
struct ListCrawlTasksRequest {
    status: Option<String>,
    sort_by: Option<String>,
    sort_order: Option<String>,
}

/// 列出任务的错误
#[derive(Debug)]
#[allow(dead_code)]
enum ListCrawlTasksError {
    StorageError(String),
}

/// Mock列出爬取任务的核心逻辑
///
/// 模拟 Tauri command 的行为,不依赖真实的Redis
async fn mock_list_crawl_tasks(
    request: ListCrawlTasksRequest,
    redis: &MockRedisService,
) -> Result<ListCrawlTasksResponse, ListCrawlTasksError> {
    // 1. 获取所有任务键
    let all_keys = redis
        .keys("crawl:task:*")
        .await
        .map_err(|e| ListCrawlTasksError::StorageError(format!("查询任务列表失败: {}", e)))?;

    // 2. 批量读取任务数据
    let mut tasks = Vec::new();
    for key in all_keys {
        if let Ok(hash) = redis.hgetall(&key).await {
            // 解析任务数据
            let task_id = hash.get("id").cloned().unwrap_or_default();
            let keyword = hash.get("keyword").cloned().unwrap_or_default();
            let status = hash.get("status").cloned().unwrap_or_default();

            // 过滤状态
            if let Some(ref filter_status) = request.status {
                if &status != filter_status {
                    continue;
                }
            }

            // 时间戳转ISO 8601
            let event_start_time = hash
                .get("event_start_time")
                .and_then(|ts| ts.parse::<i64>().ok())
                .map(|ts| DateTime::from_timestamp(ts, 0))
                .flatten()
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_default();

            let created_at = hash
                .get("created_at")
                .and_then(|ts| ts.parse::<i64>().ok())
                .map(|ts| DateTime::from_timestamp(ts, 0))
                .flatten()
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_default();

            let updated_at = hash
                .get("updated_at")
                .and_then(|ts| ts.parse::<i64>().ok())
                .map(|ts| DateTime::from_timestamp(ts, 0))
                .flatten()
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_default();

            let crawled_count = hash
                .get("crawled_count")
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(0);

            let failure_reason = hash.get("failure_reason").and_then(|s| {
                if s.is_empty() {
                    None
                } else {
                    Some(s.clone())
                }
            });

            tasks.push(CrawlTaskSummary {
                task_id,
                keyword,
                status,
                event_start_time,
                crawled_count,
                created_at,
                updated_at,
                failure_reason,
            });
        }
    }

    // 3. 排序
    let sort_by = request.sort_by.as_deref().unwrap_or("createdAt");
    let sort_order = request.sort_order.as_deref().unwrap_or("desc");

    tasks.sort_by(|a, b| {
        let ordering = match sort_by {
            "createdAt" => a.created_at.cmp(&b.created_at),
            "updatedAt" => a.updated_at.cmp(&b.updated_at),
            "crawledCount" => a.crawled_count.cmp(&b.crawled_count),
            _ => a.created_at.cmp(&b.created_at),
        };

        if sort_order == "desc" {
            ordering.reverse()
        } else {
            ordering
        }
    });

    // 4. 返回响应
    let total = tasks.len();
    Ok(ListCrawlTasksResponse { tasks, total })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 创建测试任务数据
    async fn create_test_task(
        redis: &MockRedisService,
        task_id: &str,
        keyword: &str,
        status: &str,
        event_start_time: i64,
        crawled_count: u64,
        created_at: i64,
        updated_at: i64,
        failure_reason: Option<&str>,
    ) {
        let key = format!("crawl:task:{}", task_id);
        redis.hset(&key, "id", task_id.to_string()).await.unwrap();
        redis
            .hset(&key, "keyword", keyword.to_string())
            .await
            .unwrap();
        redis
            .hset(&key, "status", status.to_string())
            .await
            .unwrap();
        redis
            .hset(&key, "event_start_time", event_start_time.to_string())
            .await
            .unwrap();
        redis
            .hset(&key, "crawled_count", crawled_count.to_string())
            .await
            .unwrap();
        redis
            .hset(&key, "created_at", created_at.to_string())
            .await
            .unwrap();
        redis
            .hset(&key, "updated_at", updated_at.to_string())
            .await
            .unwrap();
        redis
            .hset(
                &key,
                "failure_reason",
                failure_reason.unwrap_or("").to_string(),
            )
            .await
            .unwrap();
    }

    /// 测试无任务时返回空数组
    ///
    /// 契约要求:
    /// - tasks数组为空
    /// - total为0
    #[tokio::test]
    async fn test_list_tasks_empty() {
        let redis = MockRedisService::new();

        let request = ListCrawlTasksRequest {
            status: None,
            sort_by: None,
            sort_order: None,
        };

        let result = mock_list_crawl_tasks(request, &redis).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.tasks.len(), 0);
        assert_eq!(response.total, 0);
    }

    /// 测试返回所有任务的摘要信息
    ///
    /// 契约要求:
    /// - 返回所有任务
    /// - 包含完整的摘要字段
    /// - 时间字段使用ISO 8601格式
    #[tokio::test]
    async fn test_list_all_tasks() {
        let redis = MockRedisService::new();

        // 创建两个测试任务
        create_test_task(
            &redis,
            "550e8400-e29b-41d4-a716-446655440000",
            "国庆",
            "HistoryCrawling",
            1696118400, // 2023-10-01T00:00:00Z
            1234,
            1696668000, // 2023-10-07T10:00:00Z
            1696677296, // 2023-10-07T12:34:56Z
            None,
        )
        .await;

        create_test_task(
            &redis,
            "660e8400-e29b-41d4-a716-446655440001",
            "中秋",
            "HistoryCompleted",
            1694736000, // 2023-09-15T00:00:00Z
            5678,
            1695196800, // 2023-09-20T08:00:00Z
            1695204000, // 2023-09-20T10:00:00Z
            None,
        )
        .await;

        let request = ListCrawlTasksRequest {
            status: None,
            sort_by: None,
            sort_order: None,
        };

        let result = mock_list_crawl_tasks(request, &redis).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.total, 2);
        assert_eq!(response.tasks.len(), 2);

        // 验证第一个任务 (按created_at降序,国庆最新)
        let task1 = &response.tasks[0];
        assert_eq!(task1.task_id, "550e8400-e29b-41d4-a716-446655440000");
        assert_eq!(task1.keyword, "国庆");
        assert_eq!(task1.status, "HistoryCrawling");
        assert_eq!(task1.event_start_time, "2023-10-01T00:00:00+00:00");
        assert_eq!(task1.crawled_count, 1234);
        assert_eq!(task1.created_at, "2023-10-07T10:00:00+00:00");
        assert_eq!(task1.updated_at, "2023-10-07T12:34:56+00:00");
        assert_eq!(task1.failure_reason, None);

        // 验证第二个任务
        let task2 = &response.tasks[1];
        assert_eq!(task2.task_id, "660e8400-e29b-41d4-a716-446655440001");
        assert_eq!(task2.keyword, "中秋");
        assert_eq!(task2.status, "HistoryCompleted");
    }

    /// 测试按状态过滤
    ///
    /// 契约要求:
    /// - 只返回匹配状态的任务
    /// - 其他任务被过滤掉
    #[tokio::test]
    async fn test_list_tasks_filter_by_status() {
        let redis = MockRedisService::new();

        // 创建不同状态的任务
        create_test_task(
            &redis,
            "task1",
            "国庆",
            "HistoryCrawling",
            1696118400,
            1000,
            1696668000,
            1696668000,
            None,
        )
        .await;

        create_test_task(
            &redis,
            "task2",
            "中秋",
            "Failed",
            1694736000,
            100,
            1695196800,
            1695196800,
            Some("网络请求失败: Connection timeout"),
        )
        .await;

        create_test_task(
            &redis,
            "task3",
            "春节",
            "Failed",
            1704067200,
            50,
            1707652800,
            1707652800,
            Some("Redis错误"),
        )
        .await;

        // 只查询Failed状态
        let request = ListCrawlTasksRequest {
            status: Some("Failed".to_string()),
            sort_by: None,
            sort_order: None,
        };

        let result = mock_list_crawl_tasks(request, &redis).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.total, 2);
        assert_eq!(response.tasks.len(), 2);

        // 验证所有任务都是Failed状态
        for task in &response.tasks {
            assert_eq!(task.status, "Failed");
            assert!(task.failure_reason.is_some());
        }

        // 验证第一个任务 (按created_at降序,春节最新)
        let task1 = &response.tasks[0];
        assert_eq!(task1.task_id, "task3");
        assert_eq!(task1.keyword, "春节");
        assert_eq!(task1.failure_reason, Some("Redis错误".to_string()));
    }

    /// 测试按创建时间排序 (升序)
    ///
    /// 契约要求:
    /// - 按created_at字段排序
    /// - 支持asc升序
    #[tokio::test]
    async fn test_list_tasks_sort_by_created_at_asc() {
        let redis = MockRedisService::new();

        // 创建三个任务,不同创建时间
        create_test_task(
            &redis,
            "task1",
            "国庆",
            "HistoryCrawling",
            1696118400,
            1000,
            1696668000, // 2023-10-07 10:00:00
            1696668000,
            None,
        )
        .await;

        create_test_task(
            &redis,
            "task2",
            "中秋",
            "HistoryCompleted",
            1694736000,
            2000,
            1695196800, // 2023-09-20 08:00:00
            1695196800,
            None,
        )
        .await;

        create_test_task(
            &redis, "task3", "春节", "Created", 1704067200, 3000,
            1707652800, // 2024-02-10 10:00:00
            1707652800, None,
        )
        .await;

        let request = ListCrawlTasksRequest {
            status: None,
            sort_by: Some("createdAt".to_string()),
            sort_order: Some("asc".to_string()),
        };

        let result = mock_list_crawl_tasks(request, &redis).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.total, 3);

        // 验证按创建时间升序排列
        assert_eq!(response.tasks[0].task_id, "task2"); // 最早: 2023-09-20
        assert_eq!(response.tasks[1].task_id, "task1"); // 中间: 2023-10-07
        assert_eq!(response.tasks[2].task_id, "task3"); // 最晚: 2024-02-10
    }

    /// 测试按创建时间排序 (降序)
    ///
    /// 契约要求:
    /// - 按created_at字段排序
    /// - 支持desc降序 (默认排序)
    #[tokio::test]
    async fn test_list_tasks_sort_by_created_at_desc() {
        let redis = MockRedisService::new();

        create_test_task(
            &redis,
            "task1",
            "国庆",
            "HistoryCrawling",
            1696118400,
            1000,
            1696668000,
            1696668000,
            None,
        )
        .await;

        create_test_task(
            &redis,
            "task2",
            "中秋",
            "HistoryCompleted",
            1694736000,
            2000,
            1695196800,
            1695196800,
            None,
        )
        .await;

        let request = ListCrawlTasksRequest {
            status: None,
            sort_by: Some("createdAt".to_string()),
            sort_order: Some("desc".to_string()),
        };

        let result = mock_list_crawl_tasks(request, &redis).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.total, 2);

        // 验证按创建时间降序排列
        assert_eq!(response.tasks[0].task_id, "task1"); // 最新
        assert_eq!(response.tasks[1].task_id, "task2"); // 较旧
    }

    /// 测试所有时间字段使用ISO 8601格式
    ///
    /// 契约要求:
    /// - event_start_time使用ISO 8601
    /// - created_at使用ISO 8601
    /// - updated_at使用ISO 8601
    #[tokio::test]
    async fn test_list_tasks_iso8601_time_format() {
        let redis = MockRedisService::new();

        create_test_task(
            &redis,
            "task1",
            "国庆",
            "HistoryCrawling",
            1696118400, // 2023-10-01T00:00:00Z
            1234,
            1696668000, // 2023-10-07T10:00:00Z
            1696677296, // 2023-10-07T12:34:56Z
            None,
        )
        .await;

        let request = ListCrawlTasksRequest {
            status: None,
            sort_by: None,
            sort_order: None,
        };

        let result = mock_list_crawl_tasks(request, &redis).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.tasks.len(), 1);

        let task = &response.tasks[0];

        // 验证ISO 8601格式 (RFC3339)
        assert!(task.event_start_time.contains("T"));
        assert!(task.event_start_time.contains("Z") || task.event_start_time.contains("+00:00"));
        assert!(task.created_at.contains("T"));
        assert!(task.created_at.contains("Z") || task.created_at.contains("+00:00"));
        assert!(task.updated_at.contains("T"));
        assert!(task.updated_at.contains("Z") || task.updated_at.contains("+00:00"));

        // 验证可以解析为DateTime
        assert!(DateTime::parse_from_rfc3339(&task.event_start_time).is_ok());
        assert!(DateTime::parse_from_rfc3339(&task.created_at).is_ok());
        assert!(DateTime::parse_from_rfc3339(&task.updated_at).is_ok());
    }

    /// 测试Redis存储错误
    ///
    /// 契约要求:
    /// - Redis操作失败时返回StorageError
    #[tokio::test]
    async fn test_list_tasks_storage_error() {
        let redis = MockRedisService::new();
        redis.set_fail_mode(true).await;

        let request = ListCrawlTasksRequest {
            status: None,
            sort_by: None,
            sort_order: None,
        };

        let result = mock_list_crawl_tasks(request, &redis).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ListCrawlTasksError::StorageError(msg) => {
                assert!(msg.contains("查询任务列表失败"));
            }
        }
    }
}
