//! 契约测试: trigger_manual_check
//!
//! 参考文档:
//! - specs/002-/contracts/trigger_manual_check.md (契约定义)
//! - specs/002-/data-model.md (DependencyCheckResult结构)
//!
//! 验证场景:
//! 1. 手动触发返回最新检测结果
//! 2. 强制重新检测,忽略缓存
//! 3. 发送dependency-check-progress事件
//! 4. 版本不匹配和部分依赖缺失场景
//! 5. 无输入参数验证
//! 6. 性能要求验证
//! 7. JSON序列化兼容性
//!
//! 注意: 本文件使用 Mock 实现验证契约,不依赖真实 Tauri 环境

use chrono::Utc;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;

// ============================================================================
// 数据结构 (基于data-model.md定义)
// ============================================================================

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CheckStatus {
    Satisfied,
    Missing,
    VersionMismatch,
    Corrupted,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DependencyCheckResult {
    pub dependency_id: String,
    pub checked_at: chrono::DateTime<Utc>,
    pub status: CheckStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detected_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_details: Option<String>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DependencyCheckProgress {
    pub current_index: usize,
    pub total_count: usize,
    pub dependency_id: String,
    pub dependency_name: String,
    pub status: CheckStatus,
}

// ============================================================================
// Mock事件收集器
// ============================================================================

#[derive(Clone)]
struct MockEventCollector {
    events: Arc<Mutex<Vec<DependencyCheckProgress>>>,
}

impl MockEventCollector {
    fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    async fn emit(&self, event: DependencyCheckProgress) {
        self.events.lock().await.push(event);
    }

    async fn get_events(&self) -> Vec<DependencyCheckProgress> {
        self.events.lock().await.clone()
    }

    async fn clear(&self) {
        self.events.lock().await.clear();
    }

    async fn count(&self) -> usize {
        self.events.lock().await.len()
    }
}

// ============================================================================
// Mock依赖定义和检测器
// ============================================================================

#[derive(Debug, Clone)]
struct MockDependency {
    id: String,
    name: String,
    mock_status: CheckStatus,
    mock_version: Option<String>,
    detection_duration_ms: u64,
}

#[derive(Debug, Clone)]
struct MockDependencyChecker {
    dependencies: Vec<MockDependency>,
    should_fail: bool,
    failure_message: String,
}

impl MockDependencyChecker {
    /// 创建默认的Mock检测器
    fn new() -> Self {
        Self {
            dependencies: vec![
                MockDependency {
                    id: "nodejs".to_string(),
                    name: "Node.js".to_string(),
                    mock_status: CheckStatus::Satisfied,
                    mock_version: Some("20.10.0".to_string()),
                    detection_duration_ms: 45,
                },
                MockDependency {
                    id: "redis".to_string(),
                    name: "Redis Server".to_string(),
                    mock_status: CheckStatus::Satisfied,
                    mock_version: Some("7.2.4".to_string()),
                    detection_duration_ms: 20,
                },
                MockDependency {
                    id: "playwright".to_string(),
                    name: "Playwright".to_string(),
                    mock_status: CheckStatus::Missing,
                    mock_version: None,
                    detection_duration_ms: 30,
                },
                MockDependency {
                    id: "pnpm".to_string(),
                    name: "pnpm".to_string(),
                    mock_status: CheckStatus::VersionMismatch,
                    mock_version: Some("8.0.0".to_string()),
                    detection_duration_ms: 25,
                },
            ],
            should_fail: false,
            failure_message: "Mock system error".to_string(),
        }
    }

    /// 设置失败模式
    fn set_fail_mode(&mut self, should_fail: bool, message: String) {
        self.should_fail = should_fail;
        self.failure_message = message;
    }

    /// 更新特定依赖的版本
    fn set_dependency_version(&mut self, dependency_id: &str, version: String) {
        if let Some(dep) = self.dependencies.iter_mut().find(|d| d.id == dependency_id) {
            dep.mock_version = Some(version);
        }
    }

    /// 更新特定依赖的状态
    fn set_dependency_status(&mut self, dependency_id: &str, status: CheckStatus) {
        if let Some(dep) = self.dependencies.iter_mut().find(|d| d.id == dependency_id) {
            dep.mock_status = status;
        }
    }

    /// 执行依赖检测
    async fn check_dependencies(&self, event_collector: &MockEventCollector) -> Result<Vec<DependencyCheckResult>, String> {
        if self.should_fail {
            return Err(self.failure_message.clone());
        }

        let total = self.dependencies.len();
        let mut results = Vec::new();

        for (index, dep) in self.dependencies.iter().enumerate() {
            // 模拟检测延迟
            tokio::time::sleep(tokio::time::Duration::from_millis(dep.detection_duration_ms)).await;

            let checked_at = Utc::now();
            let result = DependencyCheckResult {
                dependency_id: dep.id.clone(),
                checked_at,
                status: dep.mock_status,
                detected_version: dep.mock_version.clone(),
                error_details: match dep.mock_status {
                    CheckStatus::Missing => Some(format!("{} not found in PATH", dep.name)),
                    CheckStatus::VersionMismatch => {
                        Some(format!("Version {} does not meet requirement >=8.10.0",
                            dep.mock_version.as_ref().unwrap_or(&"unknown".to_string())))
                    }
                    CheckStatus::Corrupted => Some("Executable found but failed to run".to_string()),
                    CheckStatus::Satisfied => None,
                },
                duration_ms: dep.detection_duration_ms,
            };

            // 发送进度事件
            let progress_event = DependencyCheckProgress {
                current_index: index + 1,
                total_count: total,
                dependency_id: dep.id.clone(),
                dependency_name: dep.name.clone(),
                status: dep.mock_status,
            };
            event_collector.emit(progress_event).await;

            results.push(result);
        }

        Ok(results)
    }
}

// ============================================================================
// Mock trigger_manual_check 实现 (未实现,测试应失败)
// ============================================================================

/// 模拟 trigger_manual_check 命令
///
/// 这个函数目前使用 todo!() 来确保测试失败，符合功能未实现的现状
/// 当功能实现后，应该替换为实际的调用
#[allow(dead_code)]
async fn trigger_manual_check(
    _event_collector: MockEventCollector,
) -> Result<Vec<DependencyCheckResult>, String> {
    // 功能未实现，返回错误
    todo!("trigger_manual_check command not implemented yet")

    // 预期的实现逻辑：
    // let checker = MockDependencyChecker::new();
    // checker.check_dependencies(&_event_collector).await
}

/// 完整的Mock实现（用于测试验证逻辑）
///
/// 这个函数展示了当 trigger_manual_check 实现后应该如何工作
/// 目前仅在注释中展示预期行为
#[allow(dead_code)]
async fn mock_trigger_manual_check(
    event_collector: MockEventCollector,
    checker: MockDependencyChecker,
) -> Result<Vec<DependencyCheckResult>, String> {
    checker.check_dependencies(&event_collector).await
}

// ============================================================================
// 测试用例
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试用例1: 手动触发返回最新检测结果
    ///
    /// 契约要求:
    /// - 返回Vec<DependencyCheckResult>,包含所有依赖
    /// - 每个结果的checked_at是最新时间戳(1秒内)
    /// - 所有必需字段存在且有效
    #[tokio::test]
    async fn test_manual_check_returns_fresh_results() {
        let collector = MockEventCollector::new();

        // 预期失败: 功能未实现
        let result = trigger_manual_check(collector.clone()).await;

        assert!(
            result.is_err(),
            "Expected unimplemented, trigger_manual_check command should fail with todo!()"
        );

        // ====================================================================
        // 以下是预期实现后的断言 (当前因todo!()而不会执行)
        // ====================================================================
        //
        // let results = result.unwrap();
        //
        // // 验证返回所有依赖(至少3个: nodejs, redis, playwright)
        // assert!(
        //     results.len() >= 3,
        //     "应该返回至少3个依赖检测结果"
        // );
        //
        // let now = Utc::now();
        //
        // for result in &results {
        //     // 必需字段验证
        //     assert!(!result.dependency_id.is_empty(), "dependency_id不应为空");
        //     assert!(result.duration_ms > 0, "duration_ms应该大于0");
        //
        //     // 时间戳验证: 应该是最近1秒内
        //     let duration = now.signed_duration_since(result.checked_at);
        //     assert!(
        //         duration.num_seconds() < 1,
        //         "检测时间应该在1秒内, 实际间隔: {}s",
        //         duration.num_seconds()
        //     );
        //
        //     // 状态验证
        //     match result.status {
        //         CheckStatus::Satisfied => {
        //             assert!(
        //                 result.detected_version.is_some(),
        //                 "Satisfied状态应该有版本号"
        //             );
        //             assert!(result.error_details.is_none(), "成功状态不应有错误");
        //         }
        //         CheckStatus::Missing | CheckStatus::VersionMismatch | CheckStatus::Corrupted => {
        //             assert!(
        //                 result.error_details.is_some(),
        //                 "失败状态应该有错误详情"
        //             );
        //         }
        //     }
        // }
    }

    /// 测试用例2: 事件流验证 - emit progress事件
    ///
    /// 契约要求:
    /// - 每完成一个依赖检测,emit dependency-check-progress事件
    /// - 事件包含current_index, total_count, dependency_id, status
    /// - 事件顺序正确(current_index从1递增到total_count)
    /// - 事件与结果数据一致性
    #[tokio::test]
    async fn test_manual_check_emits_progress_events() {
        let collector = MockEventCollector::new();

        // 测试实际未实现的函数
        let result = trigger_manual_check(collector.clone()).await;

        // 预期失败: 功能未实现
        assert!(
            result.is_err(),
            "Expected unimplemented, trigger_manual_check command should fail"
        );

        // 使用Mock实现验证事件流逻辑（仅验证测试框架）
        let checker = MockDependencyChecker::new();
        let mock_result = mock_trigger_manual_check(collector.clone(), checker).await;

        assert!(mock_result.is_ok(), "Mock实现应该成功");

        let results = mock_result.unwrap();
        let events = collector.get_events().await;

        // 验证事件数量与依赖数量一致
        assert_eq!(
            events.len(),
            results.len(),
            "事件数量应该与依赖数量一致"
        );

        // 验证事件顺序和结构
        for (index, event) in events.iter().enumerate() {
            assert_eq!(
                event.current_index,
                index + 1,
                "事件索引应该从1开始递增"
            );
            assert_eq!(event.total_count, results.len(), "total_count应该一致");
            assert!(!event.dependency_id.is_empty(), "依赖ID不应为空");
            assert!(!event.dependency_name.is_empty(), "依赖名称不应为空");

            // 验证事件中的status与结果一致
            let matching_result = results
                .iter()
                .find(|r| r.dependency_id == event.dependency_id)
                .expect("事件中的依赖ID应该在结果中存在");

            assert_eq!(
                event.status, matching_result.status,
                "事件中的status应该与结果一致"
            );
        }

        // 验证事件时序：事件应该在检测开始后立即发送
        for (_i, event) in events.iter().enumerate() {
            let matching_result = results
                .iter()
                .find(|r| r.dependency_id == event.dependency_id)
                .unwrap();

            // 事件时间应该早于或等于对应的检测完成时间
            // 由于Mock实现是同步的，这里主要验证逻辑一致性
            assert_eq!(event.status, matching_result.status);
        }
    }

    /// 测试用例3: 部分依赖缺失场景
    ///
    /// 契约要求:
    /// - 检测到部分依赖满足,部分缺失
    /// - Missing状态包含error_details说明缺失原因
    /// - 所有依赖都会被检测,不会因为某个失败而中断
    #[tokio::test]
    async fn test_manual_check_partial_missing() {
        let collector = MockEventCollector::new();

        let result = trigger_manual_check(collector.clone()).await;

        assert!(
            result.is_err(),
            "Expected unimplemented, trigger_manual_check command should fail"
        );

        // ====================================================================
        // 预期实现后的断言 (假设playwright缺失):
        // ====================================================================
        //
        // let results = result.unwrap();
        //
        // // 验证包含不同状态的结果
        // let satisfied_count = results
        //     .iter()
        //     .filter(|r| r.status == CheckStatus::Satisfied)
        //     .count();
        // let missing_count = results
        //     .iter()
        //     .filter(|r| r.status == CheckStatus::Missing)
        //     .count();
        //
        // assert!(satisfied_count > 0, "应该有至少一个依赖满足");
        // assert!(missing_count > 0, "应该有至少一个依赖缺失");
        //
        // // 验证缺失依赖的error_details
        // for result in &results {
        //     if result.status == CheckStatus::Missing {
        //         assert!(
        //             result.error_details.is_some(),
        //             "Missing状态应该有error_details"
        //         );
        //         assert!(
        //             result
        //                 .error_details
        //                 .as_ref()
        //                 .unwrap()
        //                 .contains("not found"),
        //             "错误信息应该说明未找到"
        //         );
        //         assert!(
        //             result.detected_version.is_none(),
        //             "缺失状态不应有版本号"
        //         );
        //     }
        // }
    }

    /// 测试用例4: 强制重新检测,忽略缓存
    ///
    /// 契约要求:
    /// - 即使有缓存,也必须重新检测
    /// - 两次检测的时间戳不同
    /// - 能检测到版本变化
    #[tokio::test]
    async fn test_manual_check_ignores_cache() {
        let collector = MockEventCollector::new();

        // 测试实际未实现的函数（第一次）
        let first_result = trigger_manual_check(collector.clone()).await;

        // 预期失败: 功能未实现
        assert!(
            first_result.is_err(),
            "Expected unimplemented, trigger_manual_check command should fail"
        );

        // 使用Mock实现验证缓存忽略逻辑
        let checker = MockDependencyChecker::new();

        // 第一次检测
        let first_mock_result = mock_trigger_manual_check(collector.clone(), checker.clone()).await;
        assert!(first_mock_result.is_ok());
        let first_results = first_mock_result.unwrap();
        let first_checked_at = first_results[0].checked_at;

        // 模拟版本变化
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        // 更新某个依赖的版本（模拟环境变化）
        let mut updated_checker = MockDependencyChecker::new();
        updated_checker.set_dependency_version("nodejs", "20.11.0".to_string());
        updated_checker.set_dependency_version("pnpm", "8.15.0".to_string());

        // 清空事件收集器
        collector.clear().await;

        // 第二次检测（应该强制重新检测）
        let second_mock_result = mock_trigger_manual_check(collector.clone(), updated_checker).await;
        assert!(second_mock_result.is_ok());
        let second_results = second_mock_result.unwrap();

        // 验证时间戳更新
        let second_checked_at = second_results[0].checked_at;
        assert!(
            second_checked_at > first_checked_at,
            "第二次检测的时间戳应该晚于第一次: first={}, second={}",
            first_checked_at,
            second_checked_at
        );

        // 验证版本变化被检测到
        let nodejs_result = second_results.iter()
            .find(|r| r.dependency_id == "nodejs")
            .expect("应该找到nodejs结果");
        assert_eq!(nodejs_result.detected_version, Some("20.11.0".to_string()));

        let pnpm_result = second_results.iter()
            .find(|r| r.dependency_id == "pnpm")
            .expect("应该找到pnpm结果");
        assert_eq!(pnpm_result.detected_version, Some("8.15.0".to_string()));

        // 验证重新发送事件
        let events = collector.get_events().await;
        assert_eq!(
            events.len(),
            second_results.len(),
            "第二次检测应该重新发送所有事件"
        );

        // 验证事件中的最新数据
        for event in &events {
            let matching_result = second_results.iter()
                .find(|r| r.dependency_id == event.dependency_id)
                .unwrap();

            // 事件中的版本应该与最终结果一致
            if let Some(result_version) = &matching_result.detected_version {
                if event.dependency_id == "nodejs" {
                    assert_eq!(matching_result.detected_version, Some("20.11.0".to_string()));
                } else if event.dependency_id == "pnpm" {
                    assert_eq!(matching_result.detected_version, Some("8.15.0".to_string()));
                }
            }
        }
    }

    /// 测试用例5: 版本不匹配检测
    ///
    /// 契约要求:
    /// - 检测到版本低于要求时,返回VersionMismatch状态
    /// - detected_version显示实际版本
    /// - error_details说明版本要求
    #[tokio::test]
    async fn test_manual_check_version_mismatch() {
        let collector = MockEventCollector::new();

        let result = trigger_manual_check(collector.clone()).await;

        assert!(
            result.is_err(),
            "Expected unimplemented, trigger_manual_check command should fail"
        );

        // ====================================================================
        // 预期实现后的断言 (模拟nodejs版本19.0.0,要求>=20.0.0):
        // ====================================================================
        //
        // let results = result.unwrap();
        //
        // // 查找版本不匹配的依赖
        // let version_mismatch_results: Vec<_> = results
        //     .iter()
        //     .filter(|r| r.status == CheckStatus::VersionMismatch)
        //     .collect();
        //
        // if !version_mismatch_results.is_empty() {
        //     for result in version_mismatch_results {
        //         assert_eq!(
        //             result.status,
        //             CheckStatus::VersionMismatch,
        //             "应该为VersionMismatch状态"
        //         );
        //         assert!(
        //             result.detected_version.is_some(),
        //             "应该检测到实际版本"
        //         );
        //         assert!(
        //             result.error_details.is_some(),
        //             "应该有错误详情"
        //         );
        //         assert!(
        //             result
        //                 .error_details
        //                 .as_ref()
        //                 .unwrap()
        //                 .contains("requires") ||
        //             result
        //                 .error_details
        //                 .as_ref()
        //                 .unwrap()
        //                 .contains(">="),
        //             "错误信息应该包含版本要求"
        //         );
        //     }
        // }
    }

    /// 测试用例6: 无输入参数验证
    ///
    /// 契约要求:
    /// - trigger_manual_check不需要任何输入参数
    /// - 自动检测所有已配置依赖
    #[tokio::test]
    async fn test_manual_check_no_parameters() {
        let collector = MockEventCollector::new();

        // 无参数调用
        let result = trigger_manual_check(collector.clone()).await;

        assert!(
            result.is_err(),
            "Expected unimplemented, trigger_manual_check command should fail"
        );

        // ====================================================================
        // 预期实现后的断言:
        // ====================================================================
        //
        // let results = result.unwrap();
        //
        // // 验证返回所有依赖(至少nodejs, redis, playwright)
        // assert!(
        //     results.len() >= 3,
        //     "应该检测所有已配置依赖,至少3个"
        // );
        //
        // // 验证包含预期依赖
        // let ids: Vec<&str> = results.iter().map(|r| r.dependency_id.as_str()).collect();
        // assert!(ids.contains(&"nodejs"), "应该包含nodejs");
        // assert!(ids.contains(&"redis"), "应该包含redis");
        // assert!(ids.contains(&"playwright"), "应该包含playwright");
    }

    /// 测试用例7: 性能要求验证
    ///
    /// 契约要求:
    /// - 单个依赖检测 < 500ms (P95)
    /// - 总体检测 < 2秒 (P95)
    /// - duration_ms字段记录实际耗时
    #[tokio::test]
    async fn test_manual_check_performance() {
        let collector = MockEventCollector::new();

        // 测试实际未实现的函数
        let start = Instant::now();
        let result = trigger_manual_check(collector.clone()).await;
        let _total_duration = start.elapsed();

        // 预期失败: 功能未实现
        assert!(
            result.is_err(),
            "Expected unimplemented, trigger_manual_check command should fail"
        );

        // 使用Mock实现验证性能要求
        let checker = MockDependencyChecker::new();

        let start = Instant::now();
        let mock_result = mock_trigger_manual_check(collector.clone(), checker).await;
        let total_duration = start.elapsed();

        assert!(mock_result.is_ok());
        let results = mock_result.unwrap();

        // 验证总体耗时 < 2秒
        assert!(
            total_duration.as_secs() < 2,
            "总体检测耗时应该 < 2秒, 实际: {:?}",
            total_duration
        );

        // 验证返回的duration_ms字段记录了合理的耗时
        for result in &results {
            assert!(
                result.duration_ms < 500,
                "依赖 {} 检测耗时应该 < 500ms, 实际: {}ms",
                result.dependency_id,
                result.duration_ms
            );
            assert!(result.duration_ms > 0, "duration_ms应该大于0");
        }

        // 验证时间戳准确性 - 所有检测应该在大致相同的时间窗口内完成
        let min_checked_at = results.iter().map(|r| r.checked_at).min().unwrap();
        let max_checked_at = results.iter().map(|r| r.checked_at).max().unwrap();
        let time_window = max_checked_at.signed_duration_since(min_checked_at);

        // 时间窗口应该小于总耗时（因为检测是并行或顺序执行的）
        assert!(
            time_window.num_milliseconds() <= total_duration.as_millis() as i64,
            "检测时间窗口应该小于等于总耗时, 实际窗口: {}ms, 总耗时: {}ms",
            time_window.num_milliseconds(),
            total_duration.as_millis()
        );

        // 验证时间戳在合理范围内（最近几秒内）
        let now = Utc::now();
        for result in &results {
            let duration_since_check = now.signed_duration_since(result.checked_at);
            assert!(
                duration_since_check.num_seconds() < 5,
                "检测时间应该在最近5秒内, 实际: {}s前",
                duration_since_check.num_seconds()
            );
        }

        // 验证事件发送的及时性
        let events = collector.get_events().await;
        assert_eq!(events.len(), results.len());

        // 检查事件和结果的时间戳顺序是否合理
        for (event, result) in events.iter().zip(results.iter()) {
            // 事件应该与对应的结果相关联
            assert_eq!(event.dependency_id, result.dependency_id);
            assert_eq!(event.status, result.status);
        }
    }

    /// 测试用例8: JSON序列化兼容性
    ///
    /// 契约要求:
    /// - DependencyCheckResult可以序列化为JSON
    /// - status枚举序列化为snake_case
    /// - 与前端TypeScript接口兼容
    #[test]
    fn test_result_json_serialization() {
        let result = DependencyCheckResult {
            dependency_id: "nodejs".to_string(),
            checked_at: Utc::now(),
            status: CheckStatus::Satisfied,
            detected_version: Some("20.10.0".to_string()),
            error_details: None,
            duration_ms: 45,
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("nodejs"));
        assert!(json.contains("satisfied"));
        assert!(json.contains("20.10.0"));

        // 验证可以反序列化
        let deserialized: DependencyCheckResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.dependency_id, result.dependency_id);
        assert_eq!(deserialized.status, result.status);
    }

    /// 测试用例9: CheckStatus枚举序列化格式
    ///
    /// 契约要求:
    /// - status序列化为snake_case字符串
    #[test]
    fn test_check_status_serialization() {
        assert_eq!(
            serde_json::to_string(&CheckStatus::Satisfied).unwrap(),
            r#""satisfied""#
        );
        assert_eq!(
            serde_json::to_string(&CheckStatus::Missing).unwrap(),
            r#""missing""#
        );
        assert_eq!(
            serde_json::to_string(&CheckStatus::VersionMismatch).unwrap(),
            r#""version_mismatch""#
        );
        assert_eq!(
            serde_json::to_string(&CheckStatus::Corrupted).unwrap(),
            r#""corrupted""#
        );
    }

    /// 测试用例10: 系统错误场景
    ///
    /// 契约要求:
    /// - 检测过程中发生系统级错误时返回错误
    /// - 错误信息应该有意义且便于调试
    #[tokio::test]
    async fn test_manual_check_system_error() {
        let collector = MockEventCollector::new();

        // 测试实际未实现的函数
        let result = trigger_manual_check(collector.clone()).await;

        // 预期失败: 功能未实现
        assert!(
            result.is_err(),
            "Expected unimplemented, trigger_manual_check command should fail"
        );

        // 使用Mock实现验证系统错误处理
        let mut checker = MockDependencyChecker::new();
        checker.set_fail_mode(true, "Mock system error: Unable to access PATH".to_string());

        let error_result = mock_trigger_manual_check(collector.clone(), checker).await;

        assert!(error_result.is_err());
        let error_message = error_result.unwrap_err();
        assert!(error_message.contains("Mock system error"));
        assert!(error_message.contains("PATH"));

        // 验证错误情况下没有发送事件
        let events = collector.get_events().await;
        assert_eq!(events.len(), 0, "系统错误时不应发送任何事件");
    }

    /// 测试用例11: 依赖检测完整性验证
    ///
    /// 契约要求:
    /// - 所有配置的依赖都应该被检测
    /// - 检测不应该因为某个依赖失败而停止
    #[tokio::test]
    async fn test_manual_check_completeness() {
        let collector = MockEventCollector::new();

        // 测试实际未实现的函数
        let result = trigger_manual_check(collector.clone()).await;

        // 预期失败: 功能未实现
        assert!(
            result.is_err(),
            "Expected unimplemented, trigger_manual_check command should fail"
        );

        // 使用Mock实现验证完整性
        let checker = MockDependencyChecker::new();
        let mock_result = mock_trigger_manual_check(collector.clone(), checker).await;

        assert!(mock_result.is_ok());
        let results = mock_result.unwrap();

        // 验证包含所有预期的依赖
        let expected_deps = vec!["nodejs", "redis", "playwright", "pnpm"];
        let found_deps: Vec<&str> = results.iter().map(|r| r.dependency_id.as_str()).collect();

        for expected_dep in &expected_deps {
            assert!(
                found_deps.contains(expected_dep),
                "应该包含依赖: {}",
                expected_dep
            );
        }

        // 验证每个依赖都有完整的结果
        for result in &results {
            assert!(!result.dependency_id.is_empty(), "依赖ID不应为空");
            assert!(result.duration_ms > 0, "应该记录检测耗时");

            // 验证状态与版本/错误信息的一致性
            match result.status {
                CheckStatus::Satisfied => {
                    assert!(
                        result.detected_version.is_some(),
                        "Satisfied状态应该有版本号: {}",
                        result.dependency_id
                    );
                    assert!(
                        result.error_details.is_none(),
                        "Satisfied状态不应该有错误信息: {}",
                        result.dependency_id
                    );
                }
                CheckStatus::Missing | CheckStatus::VersionMismatch | CheckStatus::Corrupted => {
                    assert!(
                        result.error_details.is_some(),
                        "失败状态应该有错误信息: {} ({})",
                        result.dependency_id,
                        result.error_details.as_ref().unwrap()
                    );
                }
            }
        }
    }

    /// 测试用例12: 事件流完整性和顺序验证
    ///
    /// 契约要求:
    /// - 事件应该按正确顺序发送
    /// - 事件应该包含所有必需字段
    /// - 事件总数应该与依赖数量一致
    #[tokio::test]
    async fn test_manual_check_event_flow_integrity() {
        let collector = MockEventCollector::new();

        // 测试实际未实现的函数
        let result = trigger_manual_check(collector.clone()).await;

        // 预期失败: 功能未实现
        assert!(
            result.is_err(),
            "Expected unimplemented, trigger_manual_check command should fail"
        );

        // 使用Mock实现验证事件流完整性
        let checker = MockDependencyChecker::new();
        let mock_result = mock_trigger_manual_check(collector.clone(), checker).await;

        assert!(mock_result.is_ok());
        let results = mock_result.unwrap();
        let events = collector.get_events().await;

        // 验证事件总数
        assert_eq!(
            events.len(),
            results.len(),
            "事件数量应该与结果数量一致"
        );

        // 验证事件顺序：current_index应该从1开始递增
        for (index, event) in events.iter().enumerate() {
            assert_eq!(
                event.current_index,
                index + 1,
                "事件索引应该从1开始递增, 期望: {}, 实际: {}",
                index + 1,
                event.current_index
            );
            assert_eq!(
                event.total_count,
                results.len(),
                "total_count应该与总结果数一致, 期望: {}, 实际: {}",
                results.len(),
                event.total_count
            );
        }

        // 验证每个事件的结构完整性
        for event in &events {
            assert!(!event.dependency_id.is_empty(), "事件不应包含空的依赖ID");
            assert!(!event.dependency_name.is_empty(), "事件不应包含空的依赖名称");

            // 验证依赖ID在结果中存在
            let matching_result = results.iter()
                .find(|r| r.dependency_id == event.dependency_id)
                .unwrap_or_else(|| panic!("事件中的依赖ID {} 在结果中不存在", event.dependency_id));

            // 验证状态一致性
            assert_eq!(
                event.status,
                matching_result.status,
                "事件状态应该与结果状态一致: {}",
                event.dependency_id
            );
        }

        // 验证事件顺序与检测顺序的一致性
        for (i, event) in events.iter().enumerate() {
            let _corresponding_result = results.iter()
                .find(|r| r.dependency_id == event.dependency_id)
                .unwrap();

            // 检查事件的顺序是否与结果的顺序匹配（虽然顺序可能不同，但每个依赖都应该有一个事件）
            assert_eq!(
                event.current_index - 1,
                i,
                "事件的current_index应该与其在事件数组中的索引一致"
            );
        }
    }

    /// 测试用例13: 并发安全性验证
    ///
    /// 契约要求:
    /// - 事件收集器应该是线程安全的
    /// - 多个并发检测不应该相互干扰
    #[tokio::test]
    async fn test_manual_check_concurrent_safety() {
        let collector1 = MockEventCollector::new();
        let collector2 = MockEventCollector::new();
        let collector3 = MockEventCollector::new();

        let checker1 = MockDependencyChecker::new();
        let checker2 = MockDependencyChecker::new();
        let checker3 = MockDependencyChecker::new();

        // 并发执行三次检测
        let result1 = mock_trigger_manual_check(collector1.clone(), checker1);
        let result2 = mock_trigger_manual_check(collector2.clone(), checker2);
        let result3 = mock_trigger_manual_check(collector3.clone(), checker3);

        let (r1, r2, r3) = tokio::join!(result1, result2, result3);

        // 验证所有检测都成功
        assert!(r1.is_ok());
        assert!(r2.is_ok());
        assert!(r3.is_ok());

        // 验证每个收集器都有正确数量的事件
        let events1 = collector1.get_events().await;
        let events2 = collector2.get_events().await;
        let events3 = collector3.get_events().await;

        assert_eq!(events1.len(), 4);
        assert_eq!(events2.len(), 4);
        assert_eq!(events3.len(), 4);

        // 验证每个事件收集器的事件顺序正确
        for (events, collector_name) in [
            (events1, "collector1"),
            (events2, "collector2"),
            (events3, "collector3"),
        ] {
            for (index, event) in events.iter().enumerate() {
                assert_eq!(
                    event.current_index,
                    index + 1,
                    "{}: 事件索引应该正确",
                    collector_name
                );
            }
        }
    }
}
