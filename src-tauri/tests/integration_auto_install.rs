//! 集成测试: 自动安装流程
//!
//! 测试完整流程:
//! 1. 检测到pnpm/Playwright缺失
//! 2. 自动安装依赖
//! 3. 安装成功后继续启动
//!
//! 遵循优雅即简约的原则: 每个测试都服务于明确的场景验证。

use weibo_login::models::dependency::*;
use weibo_login::models::errors::*;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use chrono;

#[cfg(test)]
mod tests {
    use super::*;

    /// Mock安装器服务，用于模拟安装流程
    #[derive(Clone)]
    struct MockInstallerService {
        /// 安装任务存储
        tasks: Arc<Mutex<Vec<InstallationTask>>>,
        /// 是否应该模拟安装成功
        should_succeed: Arc<Mutex<bool>>,
        /// 模拟安装进度回调
        progress_callback: Option<Arc<Mutex<dyn Fn(InstallationTask) + Send + Sync>>>,
    }

    impl MockInstallerService {
        /// 创建新的Mock安装器
        pub fn new() -> Self {
            Self {
                tasks: Arc::new(Mutex::new(Vec::new())),
                should_succeed: Arc::new(Mutex::new(true)),
                progress_callback: None,
            }
        }

        /// 设置安装成功/失败
        pub async fn set_install_result(&self, succeed: bool) {
            *self.should_succeed.lock().await = succeed;
        }

        /// 设置进度回调
        pub fn set_progress_callback<F>(&mut self, callback: F)
        where
            F: Fn(InstallationTask) + Send + Sync + 'static,
        {
            self.progress_callback = Some(Arc::new(Mutex::new(callback)));
        }

        /// 模拟安装流程
        pub async fn simulate_install(&self, dependency_id: String) -> Result<InstallationTask, InstallErrorType> {
            let mut task = InstallationTask::new(dependency_id.clone());

            // 检查是否应该失败
            if !*self.should_succeed.lock().await {
                task.start();
                task.update_progress(
                    InstallStatus::Installing,
                    35,
                    "权限不足，无法写入安装目录".to_string()
                );
                task.mark_failed(
                    InstallErrorType::PermissionDenied,
                    "Permission denied: cannot write to /usr/local/lib".to_string()
                );
                // 存储失败的任务
                self.tasks.lock().await.push(task.clone());
                return Err(InstallErrorType::PermissionDenied);
            }

            // 模拟成功安装流程
            task.start();

            // 调用进度回调（如果设置）
            if let Some(callback) = &self.progress_callback {
                let cb = callback.lock().await;
                cb(task.clone());
            }

            // 下载阶段 (15% -> 65%)
            tokio::time::sleep(Duration::from_millis(100)).await;
            task.update_progress(
                InstallStatus::Downloading,
                15,
                format!("Downloading {} v1.48.0...", dependency_id)
            );

            // 安装阶段 (65% -> 90%)
            tokio::time::sleep(Duration::from_millis(150)).await;
            task.update_progress(
                InstallStatus::Installing,
                65,
                "Installing browsers...".to_string()
            );

            // 完成阶段 (100%)
            tokio::time::sleep(Duration::from_millis(100)).await;
            task.update_progress(
                InstallStatus::Installing,
                90,
                "Finalizing installation...".to_string()
            );

            task.mark_success();

            // 更新存储的任务状态
            {
                let mut tasks = self.tasks.lock().await;
                if let Some(stored_task) = tasks.last_mut() {
                    *stored_task = task.clone();
                }
            }

            // 再次调用进度回调
            if let Some(callback) = &self.progress_callback {
                let cb = callback.lock().await;
                cb(task.clone());
            }

            Ok(task)
        }

        /// 获取所有任务
        pub async fn get_tasks(&self) -> Vec<InstallationTask> {
            self.tasks.lock().await.clone()
        }

        /// 清空任务
        pub async fn clear_tasks(&self) {
            self.tasks.lock().await.clear();
        }
    }

    /// Mock依赖检测器，用于模拟检测状态
    #[derive(Clone)]
    struct MockDependencyChecker {
        /// Redis检测结果
        redis_satisfied: Arc<Mutex<bool>>,
        /// Playwright检测结果
        playwright_satisfied: Arc<Mutex<bool>>,
    }

    impl MockDependencyChecker {
        pub fn new() -> Self {
            Self {
                redis_satisfied: Arc::new(Mutex::new(true)),
                playwright_satisfied: Arc::new(Mutex::new(false)), // 默认缺失，触发自动安装
            }
        }

        /// 设置Redis状态
        pub async fn set_redis_status(&self, satisfied: bool) {
            *self.redis_satisfied.lock().await = satisfied;
        }

        /// 设置Playwright状态
        pub async fn set_playwright_status(&self, satisfied: bool) {
            *self.playwright_satisfied.lock().await = satisfied;
        }

        /// 模拟检测所有依赖
        pub async fn check_all_dependencies(&self) -> Vec<DependencyCheckResult> {
            let redis_satisfied = *self.redis_satisfied.lock().await;
            let playwright_satisfied = *self.playwright_satisfied.lock().await;

            vec![
                DependencyCheckResult {
                    dependency_id: "redis".to_string(),
                    checked_at: chrono::Utc::now(),
                    status: if redis_satisfied {
                        CheckStatus::Satisfied
                    } else {
                        CheckStatus::Missing
                    },
                    detected_version: if redis_satisfied {
                        Some("7.2.4".to_string())
                    } else {
                        None
                    },
                    error_details: if redis_satisfied {
                        None
                    } else {
                        Some("Redis service not reachable at localhost:6379".to_string())
                    },
                    duration_ms: 45,
                },
                DependencyCheckResult {
                    dependency_id: "playwright".to_string(),
                    checked_at: chrono::Utc::now(),
                    status: if playwright_satisfied {
                        CheckStatus::Satisfied
                    } else {
                        CheckStatus::Missing
                    },
                    detected_version: if playwright_satisfied {
                        Some("1.48.0".to_string())
                    } else {
                        None
                    },
                    error_details: if playwright_satisfied {
                        None
                    } else {
                        Some("Playwright executable not found in node_modules".to_string())
                    },
                    duration_ms: 32,
                },
            ]
        }
    }

    /// 测试场景: 自动安装Playwright成功
    #[tokio::test]
    async fn test_auto_install_playwright_success() {
        // 准备Mock服务
        let mock_checker = MockDependencyChecker::new();
        let mock_installer = MockInstallerService::new();

        // 设置场景: Redis已满足，Playwright缺失
        mock_checker.set_redis_status(true).await;
        mock_checker.set_playwright_status(false).await;
        mock_installer.set_install_result(true).await;

        // 1. 执行依赖检测
        let check_results = mock_checker.check_all_dependencies().await;
        assert_eq!(check_results.len(), 2);

        // 验证Redis状态
        let redis_result = &check_results[0];
        assert_eq!(redis_result.dependency_id, "redis");
        assert_eq!(redis_result.status, CheckStatus::Satisfied);
        assert_eq!(redis_result.detected_version, Some("7.2.4".to_string()));

        // 验证Playwright状态
        let playwright_result = &check_results[1];
        assert_eq!(playwright_result.dependency_id, "playwright");
        assert_eq!(playwright_result.status, CheckStatus::Missing);
        assert_eq!(playwright_result.detected_version, None);
        assert!(playwright_result.error_details.is_some());

        // 2. 触发自动安装Playwright
        let install_result = mock_installer.simulate_install("playwright".to_string()).await;
        assert!(install_result.is_ok());

        let task = install_result.unwrap();
        assert_eq!(task.dependency_id, "playwright");
        assert_eq!(task.status, InstallStatus::Success);
        assert_eq!(task.progress_percent, 100);
        assert!(task.error_message.is_none());
        assert!(task.error_type.is_none());

        // 验证安装日志
        assert!(!task.install_log.is_empty());
        assert!(task.install_log.iter().any(|log| log.contains("Downloading")));
        assert!(task.install_log.iter().any(|log| log.contains("Installing")));
        assert!(task.install_log.iter().any(|log| log.contains("完成")));

        // 3. 验证安装完成后重新检测
        mock_checker.set_playwright_status(true).await;
        let recheck_results = mock_checker.check_all_dependencies().await;

        let playwright_recheck = &recheck_results[1];
        assert_eq!(playwright_recheck.status, CheckStatus::Satisfied);
        assert_eq!(playwright_recheck.detected_version, Some("1.48.0".to_string()));
        assert!(playwright_recheck.error_details.is_none());

        // 4. 验证最终状态：所有依赖满足
        for result in &recheck_results {
            assert_eq!(result.status, CheckStatus::Satisfied);
            assert!(result.detected_version.is_some());
            assert!(result.error_details.is_none());
        }
    }

    /// 测试场景: 自动安装因权限不足失败
    #[tokio::test]
    async fn test_auto_install_playwright_permission_error() {
        // 准备Mock服务
        let mock_checker = MockDependencyChecker::new();
        let mock_installer = MockInstallerService::new();

        // 设置场景: Redis已满足，Playwright缺失
        mock_checker.set_redis_status(true).await;
        mock_checker.set_playwright_status(false).await;
        mock_installer.set_install_result(false).await; // 设置安装失败

        // 1. 执行依赖检测
        let check_results = mock_checker.check_all_dependencies().await;
        let playwright_result = &check_results[1];
        assert_eq!(playwright_result.status, CheckStatus::Missing);

        // 2. 尝试自动安装Playwright
        let install_result = mock_installer.simulate_install("playwright".to_string()).await;
        assert!(install_result.is_err());

        // 验证错误类型
        let error_type = install_result.unwrap_err();
        assert_eq!(error_type, InstallErrorType::PermissionDenied);

        // 3. 验证失败的任务状态
        let tasks = mock_installer.get_tasks().await;
        assert_eq!(tasks.len(), 1);

        let failed_task = &tasks[0];
        assert_eq!(failed_task.dependency_id, "playwright");
        assert_eq!(failed_task.status, InstallStatus::Failed);
        assert_eq!(failed_task.progress_percent, 35);
        assert!(failed_task.error_message.is_some());
        assert!(failed_task.error_type.is_some());
        assert_eq!(failed_task.error_type.clone().unwrap(), InstallErrorType::PermissionDenied);

        // 验证错误消息
        let error_msg = failed_task.error_message.as_ref().unwrap();
        assert!(error_msg.contains("Permission denied"));

        // 验证安装日志包含权限错误
        assert!(failed_task.install_log.iter().any(|log| log.contains("权限不足")));
    }

    /// 测试场景: 安装进度事件监听
    #[tokio::test]
    async fn test_installation_progress_events() {
        // 准备Mock服务
        let mut mock_installer = MockInstallerService::new();
        mock_installer.set_install_result(true).await;

        // 用于收集进度事件的容器
        let progress_events = Arc::new(Mutex::new(Vec::<InstallationTask>::new()));
        let progress_events_clone = progress_events.clone();

        // 设置进度回调
        mock_installer.set_progress_callback(move |task| {
            let events = progress_events_clone.clone();
            tokio::spawn(async move {
                events.lock().await.push(task);
            });
        });

        // 执行安装
        let install_result = mock_installer.simulate_install("playwright".to_string()).await;
        assert!(install_result.is_ok());

        // 等待所有进度事件处理完成
        tokio::time::sleep(Duration::from_millis(50)).await;

        // 验证进度事件
        let events = progress_events.lock().await;
        assert!(!events.is_empty());

        // 验证事件序列包含关键进度点
        let has_starting = events.iter().any(|task| task.status == InstallStatus::Downloading);
        let has_success = events.iter().any(|task| task.status == InstallStatus::Success);

        assert!(has_starting, "应该有开始下载的进度事件");
        assert!(has_success, "应该有安装成功的进度事件");

        // 验证进度是递增的
        let mut last_progress = 0;
        for event in events.iter() {
            assert!(event.progress_percent >= last_progress,
                "进度应该是递增的: {} -> {}", last_progress, event.progress_percent);
            last_progress = event.progress_percent;
        }
    }

    /// 测试场景: 安装任务生命周期验证
    #[tokio::test]
    async fn test_installation_task_lifecycle() {
        let task = InstallationTask::new("test-dependency".to_string());

        // 验证初始状态
        assert!(!task.task_id.to_string().is_empty());
        assert_eq!(task.dependency_id, "test-dependency");
        assert_eq!(task.status, InstallStatus::Pending);
        assert_eq!(task.progress_percent, 0);
        assert!(task.started_at.is_none());
        assert!(task.completed_at.is_none());
        assert!(task.error_message.is_none());
        assert!(task.error_type.is_none());
        assert_eq!(task.install_log.len(), 1); // 初始日志
        assert!(task.install_log[0].contains("Task created"));

        // 验证开始状态
        let mut mutable_task = task;
        mutable_task.start();
        assert_eq!(mutable_task.status, InstallStatus::Downloading);
        assert_eq!(mutable_task.progress_percent, 10);
        assert!(mutable_task.started_at.is_some());
        assert!(mutable_task.completed_at.is_none());
        assert!(mutable_task.install_log.iter().any(|log| log.contains("开始下载")));

        // 验证进度更新
        mutable_task.update_progress(
            InstallStatus::Installing,
            65,
            "Installing browsers...".to_string()
        );
        assert_eq!(mutable_task.status, InstallStatus::Installing);
        assert_eq!(mutable_task.progress_percent, 65);
        assert!(mutable_task.install_log.iter().any(|log| log.contains("Installing browsers")));

        // 验证成功完成
        mutable_task.mark_success();
        assert_eq!(mutable_task.status, InstallStatus::Success);
        assert_eq!(mutable_task.progress_percent, 100);
        assert!(mutable_task.completed_at.is_some());
        assert!(mutable_task.error_message.is_none());
        assert!(mutable_task.error_type.is_none());
        assert!(mutable_task.install_log.iter().any(|log| log.contains("安装完成")));
    }

    /// 测试场景: 完整的自动安装流程模拟
    #[tokio::test]
    async fn test_complete_auto_install_flow_simulation() {
        // 准备Mock服务
        let mock_checker = MockDependencyChecker::new();
        let mock_installer = MockInstallerService::new();

        // 设置初始场景: Redis已满足，Playwright缺失
        mock_checker.set_redis_status(true).await;
        mock_checker.set_playwright_status(false).await;
        mock_installer.set_install_result(true).await;

        // 阶段1: 启动时依赖检测
        println!("🔍 阶段1: 执行依赖检测...");
        let initial_check = mock_checker.check_all_dependencies().await;

        // 验证检测结果
        let missing_deps: Vec<_> = initial_check.iter()
            .filter(|r| r.status != CheckStatus::Satisfied)
            .collect();
        assert_eq!(missing_deps.len(), 1);
        assert_eq!(missing_deps[0].dependency_id, "playwright");

        // 阶段2: 触发自动安装
        println!("🔧 阶段2: 触发自动安装...");
        for result in &initial_check {
            if result.status != CheckStatus::Satisfied && result.dependency_id == "playwright" {
                let install_result = mock_installer.simulate_install(result.dependency_id.clone()).await;
                assert!(install_result.is_ok(), "Playwright安装应该成功");

                let task = install_result.unwrap();
                println!("✅ 安装任务完成: {} - {:?}", task.dependency_id, task.status);
                assert_eq!(task.status, InstallStatus::Success);
            }
        }

        // 阶段3: 安装后重新检测
        println!("🔍 阶段3: 安装后重新检测...");
        mock_checker.set_playwright_status(true).await;
        let final_check = mock_checker.check_all_dependencies().await;

        // 验证最终状态
        for result in &final_check {
            assert_eq!(result.status, CheckStatus::Satisfied,
                "所有依赖都应该满足: {} -> {:?}", result.dependency_id, result.status);
            assert!(result.detected_version.is_some(),
                "应该检测到版本号: {}", result.dependency_id);
        }

        println!("🎉 完整自动安装流程测试通过！");
    }

    /// 测试场景: 验证安装耗时和性能要求
    #[tokio::test]
    async fn test_installation_performance_requirements() {
        let mock_installer = MockInstallerService::new();
        mock_installer.set_install_result(true).await;

        let start_time = std::time::Instant::now();

        // 执行安装
        let result = mock_installer.simulate_install("playwright".to_string()).await;

        let elapsed = start_time.elapsed();

        // 验证安装成功
        assert!(result.is_ok());

        // 验证性能要求 (模拟安装应该在500ms内完成)
        assert!(elapsed.as_millis() < 500,
            "安装耗时应该 < 500ms，实际: {}ms", elapsed.as_millis());

        let task = result.unwrap();
        // 验证时间戳顺序: created_at <= started_at <= completed_at
        if let (Some(started), Some(completed)) = (&task.started_at, &task.completed_at) {
            assert!(task.created_at <= *started, "created_at 应该 <= started_at");
            assert!(*started <= *completed, "started_at 应该 <= completed_at");
        }

        println!("✅ 安装性能测试通过，耗时: {}ms", elapsed.as_millis());
    }
}
