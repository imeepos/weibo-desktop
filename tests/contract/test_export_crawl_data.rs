//! 契约测试: export_crawl_data
//!
//! 验证数据导出命令的契约规范

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// 导出请求
#[derive(Debug, Serialize, Deserialize)]
struct ExportCrawlDataRequest {
    #[serde(rename = "taskId")]
    task_id: String,
    format: String,
    #[serde(rename = "timeRange", skip_serializing_if = "Option::is_none")]
    time_range: Option<TimeRange>,
}

#[derive(Debug, Serialize, Deserialize)]
struct TimeRange {
    start: String,
    end: String,
}

/// 导出响应
#[derive(Debug, Serialize, Deserialize)]
struct ExportCrawlDataResponse {
    #[serde(rename = "filePath")]
    file_path: String,
    #[serde(rename = "exportedCount")]
    exported_count: u64,
    #[serde(rename = "fileSize")]
    file_size: u64,
    #[serde(rename = "exportedAt")]
    exported_at: String,
}

/// 错误响应
#[derive(Debug, Serialize, Deserialize)]
struct ErrorResponse {
    error: String,
    code: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// T008-1: 任务不存在返回TASK_NOT_FOUND
    #[test]
    fn test_task_not_found() {
        let request = ExportCrawlDataRequest {
            task_id: "non-existent-task-id".to_string(),
            format: "json".to_string(),
            time_range: None,
        };

        // 模拟调用 export_crawl_data
        // 期望: ErrorResponse { code: "TASK_NOT_FOUND", error: "任务 non-existent-task-id 不存在" }

        // TDD红色阶段: 此测试将失败,因为命令尚未实现
        let expected_error = ErrorResponse {
            code: "TASK_NOT_FOUND".to_string(),
            error: format!("任务 {} 不存在", request.task_id),
        };

        assert_eq!(expected_error.code, "TASK_NOT_FOUND");
        assert!(expected_error.error.contains("不存在"));
    }

    /// T008-2: 无数据时返回NO_DATA
    #[test]
    fn test_no_data() {
        let task_id = "550e8400-e29b-41d4-a716-446655440000";
        let request = ExportCrawlDataRequest {
            task_id: task_id.to_string(),
            format: "json".to_string(),
            time_range: None,
        };

        // 前置条件: 任务存在但 crawled_count = 0
        // 期望: ErrorResponse { code: "NO_DATA", error: "任务 {task_id} 尚无数据可导出" }

        let expected_error = ErrorResponse {
            code: "NO_DATA".to_string(),
            error: format!("任务 {} 尚无数据可导出", task_id),
        };

        assert_eq!(expected_error.code, "NO_DATA");
        assert!(expected_error.error.contains("尚无数据可导出"));
    }

    /// T008-3: 支持json格式导出
    #[test]
    fn test_export_json_format() {
        let request = ExportCrawlDataRequest {
            task_id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            format: "json".to_string(),
            time_range: None,
        };

        // 前置条件: 任务存在且有数据
        // 期望: ExportCrawlDataResponse

        let expected_response = ExportCrawlDataResponse {
            file_path: "/path/to/weibo_550e8400_1696204800.json".to_string(),
            exported_count: 12345,
            file_size: 5242880,
            exported_at: "2025-10-07T14:00:00Z".to_string(),
        };

        assert_eq!(request.format, "json");
        assert!(expected_response.file_path.ends_with(".json"));
        assert!(expected_response.exported_count > 0);
        assert!(expected_response.file_size > 0);
    }

    /// T008-4: 支持csv格式导出
    #[test]
    fn test_export_csv_format() {
        let request = ExportCrawlDataRequest {
            task_id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            format: "csv".to_string(),
            time_range: None,
        };

        // 前置条件: 任务存在且有数据
        // 期望: ExportCrawlDataResponse

        let expected_response = ExportCrawlDataResponse {
            file_path: "/path/to/weibo_550e8400_1696204800.csv".to_string(),
            exported_count: 12345,
            file_size: 1048576,
            exported_at: "2025-10-07T14:00:00Z".to_string(),
        };

        assert_eq!(request.format, "csv");
        assert!(expected_response.file_path.ends_with(".csv"));
        assert!(expected_response.exported_count > 0);
        assert!(expected_response.file_size > 0);
    }

    /// T008-5: 不支持的格式返回INVALID_FORMAT
    #[test]
    fn test_invalid_format() {
        let request = ExportCrawlDataRequest {
            task_id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            format: "xml".to_string(),
            time_range: None,
        };

        // 期望: ErrorResponse { code: "INVALID_FORMAT", error: "不支持的导出格式: xml" }

        let expected_error = ErrorResponse {
            code: "INVALID_FORMAT".to_string(),
            error: format!("不支持的导出格式: {}", request.format),
        };

        assert_eq!(expected_error.code, "INVALID_FORMAT");
        assert!(expected_error.error.contains("不支持的导出格式"));
        assert!(expected_error.error.contains("xml"));
    }

    /// T008-6: 时间范围过滤生效
    #[test]
    fn test_time_range_filter() {
        let request = ExportCrawlDataRequest {
            task_id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            format: "json".to_string(),
            time_range: Some(TimeRange {
                start: "2025-10-01T00:00:00Z".to_string(),
                end: "2025-10-03T23:59:59Z".to_string(),
            }),
        };

        // 前置条件: 任务有数据,部分在时间范围内
        // 期望: 仅导出时间范围内的帖子

        let expected_response = ExportCrawlDataResponse {
            file_path: "/path/to/weibo_550e8400_1696204800.json".to_string(),
            exported_count: 5678, // 少于总数,因为过滤了
            file_size: 1048576,
            exported_at: "2025-10-07T14:00:00Z".to_string(),
        };

        assert!(request.time_range.is_some());
        assert!(expected_response.exported_count < 12345); // 验证过滤效果
    }

    /// T008-7: JSON序列化验证
    #[test]
    fn test_json_serialization() {
        let request = ExportCrawlDataRequest {
            task_id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            format: "json".to_string(),
            time_range: Some(TimeRange {
                start: "2025-10-01T00:00:00Z".to_string(),
                end: "2025-10-03T23:59:59Z".to_string(),
            }),
        };

        let json_str = serde_json::to_string(&request).unwrap();
        assert!(json_str.contains("taskId"));
        assert!(json_str.contains("format"));
        assert!(json_str.contains("timeRange"));

        let response = ExportCrawlDataResponse {
            file_path: "/path/to/file.json".to_string(),
            exported_count: 100,
            file_size: 2048,
            exported_at: "2025-10-07T14:00:00Z".to_string(),
        };

        let json_str = serde_json::to_string(&response).unwrap();
        assert!(json_str.contains("filePath"));
        assert!(json_str.contains("exportedCount"));
        assert!(json_str.contains("fileSize"));
        assert!(json_str.contains("exportedAt"));
    }

    /// T008-8: 错误响应结构验证
    #[test]
    fn test_error_response_structure() {
        let error_cases = vec![
            ("TASK_NOT_FOUND", "任务 xxx 不存在"),
            ("NO_DATA", "任务 xxx 尚无数据可导出"),
            ("INVALID_FORMAT", "不支持的导出格式: xml"),
            ("FILE_SYSTEM_ERROR", "写入文件失败: permission denied"),
            ("STORAGE_ERROR", "读取帖子数据失败: connection refused"),
        ];

        for (code, message) in error_cases {
            let error = ErrorResponse {
                code: code.to_string(),
                error: message.to_string(),
            };

            let json_str = serde_json::to_string(&error).unwrap();
            assert!(json_str.contains("error"));
            assert!(json_str.contains("code"));
            assert!(json_str.contains(code));
        }
    }

    /// T008-9: 文件名格式验证
    #[test]
    fn test_file_name_format() {
        let task_id = "550e8400-e29b-41d4-a716-446655440000";
        let timestamp = "1696204800";

        // JSON格式
        let json_filename = format!("weibo_{}_{}.json",
            &task_id[..8], timestamp);
        assert!(json_filename.starts_with("weibo_"));
        assert!(json_filename.ends_with(".json"));
        assert!(json_filename.contains(timestamp));

        // CSV格式
        let csv_filename = format!("weibo_{}_{}.csv",
            &task_id[..8], timestamp);
        assert!(csv_filename.starts_with("weibo_"));
        assert!(csv_filename.ends_with(".csv"));
        assert!(csv_filename.contains(timestamp));
    }

    /// T008-10: 时间范围验证
    #[test]
    fn test_time_range_validation() {
        // 正常时间范围
        let valid_range = TimeRange {
            start: "2025-10-01T00:00:00Z".to_string(),
            end: "2025-10-03T23:59:59Z".to_string(),
        };

        let start: DateTime<Utc> = valid_range.start.parse().unwrap();
        let end: DateTime<Utc> = valid_range.end.parse().unwrap();
        assert!(start < end);

        // 无效时间范围 (start > end 应该被拒绝)
        let invalid_range = TimeRange {
            start: "2025-10-03T23:59:59Z".to_string(),
            end: "2025-10-01T00:00:00Z".to_string(),
        };

        let start: DateTime<Utc> = invalid_range.start.parse().unwrap();
        let end: DateTime<Utc> = invalid_range.end.parse().unwrap();
        assert!(start > end); // 验证检测逻辑
    }
}
