//! query_dependency_status 契约测试
//!
//! 参考: specs/002-/contracts/query_dependency_status.md
//!
//! 验证 query_dependency_status 命令符合契约定义,包括:
//! - 成功场景: 查询所有/单个依赖的缓存结果
//! - 边界场景: 空缓存、不存在的依赖ID
//! - 性能要求: 响应时间 < 50ms (纯缓存读取)
//!
//! 注意: 本文件使用 Mock 实现验证契约,不依赖真实 Tauri 环境

mod common;

use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

/// 依赖检测结果 (用于测试)
#[derive(Debug, Clone, PartialEq)]
struct DependencyCheckResult {
    dependency_id: String,
    checked_at: DateTime<Utc>,
    status: CheckStatus,
    detected_version: Option<String>,
    error_details: Option<String>,
    duration_ms: u64,
}

/// 依赖检测状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CheckStatus {
    Satisfied,
    Missing,
    VersionMismatch,
    Corrupted,
}

/// Mock应用状态 (模拟Tauri AppState)
struct MockAppState {
    /// 内存缓存 (dependency_id -> CheckResult)
    check_cache: Arc<RwLock<HashMap<String, DependencyCheckResult>>>,
}

impl MockAppState {
    fn new() -> Self {
        Self {
            check_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 插入缓存数据
    async fn insert_cache(&self, result: DependencyCheckResult) {
        let mut cache = self.check_cache.write().await;
        cache.insert(result.dependency_id.clone(), result);
    }

    /// 清空缓存
    async fn clear_cache(&self) {
        let mut cache = self.check_cache.write().await;
        cache.clear();
    }
}

/// Mock查询依赖状态的核心逻辑
async fn mock_query_dependency_status(
    dependency_id: Option<String>,
    state: &MockAppState,
) -> Result<Vec<DependencyCheckResult>, String> {
    let cache = state.check_cache.read().await;

    let results = match dependency_id {
        // 查询所有
        None => {
            let mut all_results: Vec<_> = cache.values().cloned().collect();
            // 按dependency_id字母序排序
            all_results.sort_by(|a, b| a.dependency_id.cmp(&b.dependency_id));
            all_results
        }
        // 查询单个
        Some(id) => cache
            .get(&id)
            .map(|r| vec![r.clone()])
            .unwrap_or_else(Vec::new),
    };

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 辅助函数: 创建测试用的检测结果
    fn create_test_result(
        dependency_id: &str,
        status: CheckStatus,
        version: Option<&str>,
    ) -> DependencyCheckResult {
        DependencyCheckResult {
            dependency_id: dependency_id.to_string(),
            checked_at: Utc::now(),
            status,
            detected_version: version.map(|v| v.to_string()),
            error_details: match status {
                CheckStatus::Missing => Some(format!("Dependency '{}' not found", dependency_id)),
                CheckStatus::VersionMismatch => Some("Version mismatch".to_string()),
                CheckStatus::Corrupted => Some("Corrupted installation".to_string()),
                CheckStatus::Satisfied => None,
            },
            duration_ms: 45,
        }
    }

    /// 测试查询所有依赖 (缓存命中)
    ///
    /// 契约要求:
    /// 1. 返回所有缓存的检测结果
    /// 2. 按dependency_id字母序排序
    /// 3. 响应时间 < 50ms
    #[tokio::test]
    async fn test_query_all_dependencies_from_cache() {
        let state = MockAppState::new();

        // 插入多个缓存结果 (故意乱序插入)
        state
            .insert_cache(create_test_result(
                "redis",
                CheckStatus::Missing,
                None,
            ))
            .await;
        state
            .insert_cache(create_test_result(
                "nodejs",
                CheckStatus::Satisfied,
                Some("20.10.0"),
            ))
            .await;
        state
            .insert_cache(create_test_result(
                "playwright",
                CheckStatus::Satisfied,
                Some("1.40.0"),
            ))
            .await;

        // 查询所有
        let start = Instant::now();
        let results = mock_query_dependency_status(None, &state).await;
        let duration = start.elapsed();

        assert!(results.is_ok());
        let results = results.unwrap();

        // 验证结果数量
        assert_eq!(results.len(), 3);

        // 验证按字母序排序: nodejs < playwright < redis
        assert_eq!(results[0].dependency_id, "nodejs");
        assert_eq!(results[0].status, CheckStatus::Satisfied);
        assert_eq!(results[0].detected_version, Some("20.10.0".to_string()));

        assert_eq!(results[1].dependency_id, "playwright");
        assert_eq!(results[1].status, CheckStatus::Satisfied);

        assert_eq!(results[2].dependency_id, "redis");
        assert_eq!(results[2].status, CheckStatus::Missing);
        assert!(results[2].error_details.is_some());

        // 验证性能要求
        assert!(duration.as_millis() < 50);
    }

    /// 测试查询单个依赖 (缓存命中)
    ///
    /// 契约要求:
    /// 1. 返回匹配dependency_id的结果(包装在Vec中)
    /// 2. 只返回一个元素
    #[tokio::test]
    async fn test_query_single_dependency() {
        let state = MockAppState::new();

        state
            .insert_cache(create_test_result(
                "nodejs",
                CheckStatus::Satisfied,
                Some("20.10.0"),
            ))
            .await;
        state
            .insert_cache(create_test_result(
                "redis",
                CheckStatus::Missing,
                None,
            ))
            .await;

        // 查询单个依赖
        let results = mock_query_dependency_status(Some("nodejs".to_string()), &state).await;

        assert!(results.is_ok());
        let results = results.unwrap();

        // 验证只返回一个结果
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].dependency_id, "nodejs");
        assert_eq!(results[0].status, CheckStatus::Satisfied);
        assert_eq!(results[0].detected_version, Some("20.10.0".to_string()));
    }

    /// 测试查询不存在的依赖
    ///
    /// 契约要求:
    /// 返回空Vec,不抛出错误
    #[tokio::test]
    async fn test_query_nonexistent_dependency() {
        let state = MockAppState::new();

        state
            .insert_cache(create_test_result(
                "nodejs",
                CheckStatus::Satisfied,
                Some("20.10.0"),
            ))
            .await;

        // 查询不存在的依赖
        let results =
            mock_query_dependency_status(Some("nonexistent".to_string()), &state).await;

        assert!(results.is_ok());
        let results = results.unwrap();

        // 验证返回空Vec
        assert_eq!(results.len(), 0);
    }

    /// 测试查询空缓存
    ///
    /// 契约要求:
    /// 缓存为空时返回空Vec,不抛出错误
    #[tokio::test]
    async fn test_query_empty_cache() {
        let state = MockAppState::new();

        // 查询所有 (空缓存)
        let results = mock_query_dependency_status(None, &state).await;

        assert!(results.is_ok());
        let results = results.unwrap();

        assert_eq!(results.len(), 0);
    }

    /// 测试查询单个依赖 (空缓存)
    #[tokio::test]
    async fn test_query_single_from_empty_cache() {
        let state = MockAppState::new();

        let results = mock_query_dependency_status(Some("nodejs".to_string()), &state).await;

        assert!(results.is_ok());
        let results = results.unwrap();

        assert_eq!(results.len(), 0);
    }

    /// 测试缓存更新后查询
    ///
    /// 验证缓存更新机制工作正常
    #[tokio::test]
    async fn test_query_after_cache_update() {
        let state = MockAppState::new();

        // 初始状态: nodejs缺失
        state
            .insert_cache(create_test_result(
                "nodejs",
                CheckStatus::Missing,
                None,
            ))
            .await;

        let results = mock_query_dependency_status(Some("nodejs".to_string()), &state).await;
        assert_eq!(results.unwrap()[0].status, CheckStatus::Missing);

        // 更新缓存: nodejs已安装
        state
            .insert_cache(create_test_result(
                "nodejs",
                CheckStatus::Satisfied,
                Some("20.10.0"),
            ))
            .await;

        let results = mock_query_dependency_status(Some("nodejs".to_string()), &state).await;
        assert_eq!(results.unwrap()[0].status, CheckStatus::Satisfied);
    }

    /// 测试性能要求
    ///
    /// 契约要求:
    /// 响应时间 < 50ms (P95)
    /// 在Mock环境下应该远低于这个值
    #[tokio::test]
    async fn test_query_performance() {
        let state = MockAppState::new();

        // 插入多个缓存结果
        for i in 0..10 {
            state
                .insert_cache(create_test_result(
                    &format!("dep_{}", i),
                    CheckStatus::Satisfied,
                    Some("1.0.0"),
                ))
                .await;
        }

        // 进行多次查询,测试平均性能
        let mut durations = Vec::new();
        for _ in 0..100 {
            let start = Instant::now();
            let result = mock_query_dependency_status(None, &state).await;
            let duration = start.elapsed();

            assert!(result.is_ok());
            durations.push(duration.as_millis());
        }

        // 计算P95
        durations.sort();
        let p95_index = (durations.len() as f64 * 0.95) as usize;
        let p95_duration = durations[p95_index];

        assert!(p95_duration < 50, "P95 duration: {}ms", p95_duration);
    }

    /// 测试并发查询
    ///
    /// 契约要求:
    /// 支持最多100个并发请求
    #[tokio::test]
    async fn test_query_concurrent() {
        use std::sync::Arc;

        let state = Arc::new(MockAppState::new());

        // 准备缓存数据
        for i in 0..10 {
            state
                .insert_cache(create_test_result(
                    &format!("dep_{}", i),
                    CheckStatus::Satisfied,
                    Some("1.0.0"),
                ))
                .await;
        }

        // 并发查询
        let mut tasks = Vec::new();
        for i in 0..100 {
            let state_clone = Arc::clone(&state);
            let dependency_id = if i % 3 == 0 {
                Some(format!("dep_{}", i % 10))
            } else {
                None
            };

            tasks.push(tokio::spawn(async move {
                mock_query_dependency_status(dependency_id, &state_clone).await
            }));
        }

        // 等待所有任务完成
        for task in tasks {
            let result = task.await.unwrap();
            assert!(result.is_ok());
        }
    }

    /// 测试不同检测状态的查询
    ///
    /// 验证所有CheckStatus类型都能正确查询
    #[tokio::test]
    async fn test_query_all_check_statuses() {
        let state = MockAppState::new();

        state
            .insert_cache(create_test_result(
                "dep_satisfied",
                CheckStatus::Satisfied,
                Some("1.0.0"),
            ))
            .await;
        state
            .insert_cache(create_test_result(
                "dep_missing",
                CheckStatus::Missing,
                None,
            ))
            .await;
        state
            .insert_cache(create_test_result(
                "dep_version_mismatch",
                CheckStatus::VersionMismatch,
                Some("0.9.0"),
            ))
            .await;
        state
            .insert_cache(create_test_result(
                "dep_corrupted",
                CheckStatus::Corrupted,
                None,
            ))
            .await;

        let results = mock_query_dependency_status(None, &state).await;

        assert!(results.is_ok());
        let results = results.unwrap();

        assert_eq!(results.len(), 4);

        // 验证每种状态都存在
        let statuses: Vec<_> = results.iter().map(|r| r.status).collect();
        assert!(statuses.contains(&CheckStatus::Satisfied));
        assert!(statuses.contains(&CheckStatus::Missing));
        assert!(statuses.contains(&CheckStatus::VersionMismatch));
        assert!(statuses.contains(&CheckStatus::Corrupted));
    }

    /// 测试缓存清空后查询
    #[tokio::test]
    async fn test_query_after_cache_clear() {
        let state = MockAppState::new();

        // 插入缓存
        state
            .insert_cache(create_test_result(
                "nodejs",
                CheckStatus::Satisfied,
                Some("20.10.0"),
            ))
            .await;

        // 验证缓存存在
        let results = mock_query_dependency_status(None, &state).await;
        assert_eq!(results.unwrap().len(), 1);

        // 清空缓存
        state.clear_cache().await;

        // 验证缓存为空
        let results = mock_query_dependency_status(None, &state).await;
        assert_eq!(results.unwrap().len(), 0);
    }

    /// 测试查询结果包含完整字段
    ///
    /// 验证返回的DependencyCheckResult包含所有必需字段
    #[tokio::test]
    async fn test_query_result_has_all_fields() {
        let state = MockAppState::new();

        state
            .insert_cache(create_test_result(
                "nodejs",
                CheckStatus::Satisfied,
                Some("20.10.0"),
            ))
            .await;

        let results = mock_query_dependency_status(Some("nodejs".to_string()), &state).await;

        assert!(results.is_ok());
        let result = &results.unwrap()[0];

        // 验证所有字段存在且正确
        assert_eq!(result.dependency_id, "nodejs");
        assert!(result.checked_at <= Utc::now());
        assert_eq!(result.status, CheckStatus::Satisfied);
        assert_eq!(result.detected_version, Some("20.10.0".to_string()));
        assert_eq!(result.error_details, None);
        assert_eq!(result.duration_ms, 45);
    }
}
