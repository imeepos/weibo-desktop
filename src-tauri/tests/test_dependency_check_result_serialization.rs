//! DependencyCheckResult JSON序列化测试
//!
//! 验证DependencyCheckResult模型能够正确序列化和反序列化，
//! 确保与前端React组件的数据交换兼容性。

use serde_json;
use chrono::{DateTime, Utc};
use weibo_login::models::dependency::{DependencyCheckResult, CheckStatus};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dependency_check_result_satisfied_serialization() {
        let checked_at = DateTime::parse_from_rfc3339("2025-10-05T10:30:15.123Z")
            .unwrap()
            .with_timezone(&Utc);

        let result = DependencyCheckResult {
            dependency_id: "nodejs".to_string(),
            checked_at,
            status: CheckStatus::Satisfied,
            detected_version: Some("20.10.0".to_string()),
            error_details: None,
            duration_ms: 45,
        };

        // 序列化为JSON
        let json = serde_json::to_string_pretty(&result).expect("序列化失败");

        println!("序列化结果:\n{}", json);

        // 验证JSON包含必要字段
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("解析JSON失败");

        assert_eq!(parsed["dependency_id"], "nodejs");
        assert_eq!(parsed["status"], "satisfied");
        assert_eq!(parsed["detected_version"], "20.10.0");
        assert!(parsed["error_details"].is_null());
        assert_eq!(parsed["duration_ms"], 45);

        // 反序列化验证
        let deserialized: DependencyCheckResult = serde_json::from_str(&json)
            .expect("反序列化失败");

        assert_eq!(deserialized.dependency_id, result.dependency_id);
        assert_eq!(deserialized.status, result.status);
        assert_eq!(deserialized.detected_version, result.detected_version);
        assert_eq!(deserialized.duration_ms, result.duration_ms);
    }

    #[test]
    fn test_dependency_check_result_missing_serialization() {
        let checked_at = DateTime::parse_from_rfc3339("2025-10-05T10:31:20.456Z")
            .unwrap()
            .with_timezone(&Utc);

        let result = DependencyCheckResult {
            dependency_id: "redis".to_string(),
            checked_at,
            status: CheckStatus::Missing,
            detected_version: None,
            error_details: Some("Redis server not found in PATH".to_string()),
            duration_ms: 12,
        };

        let json = serde_json::to_string(&result).expect("序列化失败");

        // 验证JSON不包含None值字段
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("解析JSON失败");

        assert!(parsed.get("detected_version").is_none() || parsed["detected_version"].is_null());
        assert_eq!(parsed["error_details"], "Redis server not found in PATH");

        // 反序列化验证
        let deserialized: DependencyCheckResult = serde_json::from_str(&json)
            .expect("反序列化失败");

        assert_eq!(deserialized.status, CheckStatus::Missing);
        assert_eq!(deserialized.detected_version, None);
        assert_eq!(deserialized.error_details, Some("Redis server not found in PATH".to_string()));
    }

    #[test]
    fn test_check_status_serialization() {
        // 测试所有CheckStatus枚举值的序列化
        let statuses = vec![
            CheckStatus::Satisfied,
            CheckStatus::Missing,
            CheckStatus::VersionMismatch,
            CheckStatus::Corrupted,
        ];

        for status in statuses {
            let json = serde_json::to_string(&status).expect("序列化失败");
            let deserialized: CheckStatus = serde_json::from_str(&json)
                .expect("反序列化失败");

            assert_eq!(status, deserialized, "CheckStatus {:?} 序列化反序列化不一致", status);
        }
    }

    #[test]
    fn test_dependency_check_result_version_mismatch() {
        let checked_at = Utc::now();

        let result = DependencyCheckResult {
            dependency_id: "pnpm".to_string(),
            checked_at,
            status: CheckStatus::VersionMismatch,
            detected_version: Some("7.5.0".to_string()),
            error_details: Some("Required version >=8.0.0, found 7.5.0".to_string()),
            duration_ms: 23,
        };

        let json = serde_json::to_string_pretty(&result).expect("序列化失败");

        // 验证关键字段
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("解析JSON失败");
        assert_eq!(parsed["status"], "version_mismatch");
        assert_eq!(parsed["detected_version"], "7.5.0");
        assert_eq!(parsed["error_details"], "Required version >=8.0.0, found 7.5.0");

        // 完整往返测试
        let roundtrip: DependencyCheckResult = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip.dependency_id, "pnpm");
        assert_eq!(roundtrip.status, CheckStatus::VersionMismatch);
    }

    #[test]
    fn test_dependency_check_result_corrupted() {
        let checked_at = Utc::now();

        let result = DependencyCheckResult {
            dependency_id: "playwright".to_string(),
            checked_at,
            status: CheckStatus::Corrupted,
            detected_version: Some("1.40.0".to_string()),
            error_details: Some("Browser executable not found or corrupted".to_string()),
            duration_ms: 156,
        };

        let json = serde_json::to_string(&result).expect("序列化失败");
        let deserialized: DependencyCheckResult = serde_json::from_str(&json)
            .expect("反序列化失败");

        assert_eq!(deserialized.status, CheckStatus::Corrupted);
        assert_eq!(deserialized.error_details.unwrap(), "Browser executable not found or corrupted");
        assert_eq!(deserialized.duration_ms, 156);
    }

    #[test]
    fn test_empty_error_details_and_version() {
        let checked_at = Utc::now();

        let result = DependencyCheckResult {
            dependency_id: "test".to_string(),
            checked_at,
            status: CheckStatus::Satisfied,
            detected_version: None,
            error_details: None,
            duration_ms: 0,
        };

        let json = serde_json::to_string(&result).expect("序列化失败");

        // 验证None值字段不在JSON中或为null
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("解析JSON失败");

        // 检查这些字段是否存在且为null，或者完全不存在
        if let Some(version) = parsed.get("detected_version") {
            assert!(version.is_null());
        }

        if let Some(error) = parsed.get("error_details") {
            assert!(error.is_null());
        }
    }
}