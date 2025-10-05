use weibo_login::services::dependency_checker::*;
use weibo_login::models::dependency::*;
use weibo_login::models::errors::*;

/// 测试并发依赖检测功能
#[cfg(test)]
mod tests {
    use super::*;

    /// 创建测试依赖项列表
    pub fn create_test_dependencies() -> Vec<Dependency> {
        vec![
            Dependency::new(
                "nodejs".to_string(),
                "Node.js".to_string(),
                ">=20.0.0".to_string(),
                "JavaScript运行时环境".to_string(),
                DependencyLevel::Required,
                false,
                1,
                CheckMethod::Executable {
                    name: "node".to_string(),
                    version_args: vec!["--version".to_string()],
                },
                "请访问 https://nodejs.org 下载安装".to_string(),
                None,
            ),
            Dependency::new(
                "redis".to_string(),
                "Redis Server".to_string(),
                ">=7.0.0".to_string(),
                "内存数据库服务".to_string(),
                DependencyLevel::Required,
                false,
                2,
                CheckMethod::Service {
                    host: "localhost".to_string(),
                    port: 6379,
                },
                "请启动 Redis 服务: redis-server".to_string(),
                None,
            ),
            Dependency::new(
                "test-file".to_string(),
                "Test File".to_string(),
                "*".to_string(),
                "测试文件存在性".to_string(),
                DependencyLevel::Optional,
                false,
                5,
                CheckMethod::File {
                    path: "/tmp/test.txt".to_string(),
                },
                "创建测试文件: touch /tmp/test.txt".to_string(),
                None,
            ),
        ]
    }

    #[tokio::test]
    async fn test_validate_version_function() {
        // 测试版本验证功能
        assert!(DependencyChecker::validate_version("20.10.0", ">=20.0.0"));
        assert!(DependencyChecker::validate_version("1.2.3", "^1.2.3"));
        assert!(DependencyChecker::validate_version("1.2.4", "~1.2.3"));
        assert!(!DependencyChecker::validate_version("1.2.3", ">=2.0.0"));
    }

    #[tokio::test]
    async fn test_parse_version_from_output() {
        // 测试版本解析功能
        assert_eq!(
            DependencyChecker::parse_version_from_output("v20.10.0"),
            Some("20.10.0".to_string())
        );
        assert_eq!(
            DependencyChecker::parse_version_from_output("node v20.10.0"),
            Some("20.10.0".to_string())
        );
        assert_eq!(
            DependencyChecker::parse_version_from_output("pnpm 8.15.0"),
            Some("8.15.0".to_string())
        );
        assert_eq!(
            DependencyChecker::parse_version_from_output("Redis server v=7.2.3"),
            Some("7.2.3".to_string())
        );
    }

    #[tokio::test]
    async fn test_single_dependency_check() {
        let checker = DependencyChecker::new();
        let deps = create_test_dependencies();

        // 测试单个依赖检测
        for dep in &deps {
            let result = checker.check_dependency(dep).await;
            println!("依赖检测结果: {:?}", result);

            // 检查结果结构
            match result {
                Ok(check_result) => {
                    assert!(!check_result.dependency_id.is_empty());
                    assert!(check_result.duration_ms >= 0);
                    println!("✓ 依赖 {} 检测完成: {}", dep.name, check_result.status.description());
                }
                Err(e) => {
                    println!("✗ 依赖 {} 检测失败: {}", dep.name, e);
                }
            }
        }
    }

    #[tokio::test]
    async fn test_check_executable_dependency() {
        let result = DependencyChecker::check_executable_dependency(
            "test-node",
            "node",
            &["--version".to_string()],
        ).await;

        match result {
            Ok(check_result) => {
                println!("Node.js 检测结果: {:?}", check_result);
                assert_eq!(check_result.dependency_id, "test-node");
            }
            Err(e) => {
                println!("Node.js 检测失败: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_check_service_dependency() {
        let result = DependencyChecker::check_service_dependency(
            "test-redis",
            "localhost",
            6379,
        ).await;

        match result {
            Ok(check_result) => {
                println!("Redis 检测结果: {:?}", check_result);
                assert_eq!(check_result.dependency_id, "test-redis");
            }
            Err(e) => {
                println!("Redis 检测失败: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_check_file_dependency() {
        let result = DependencyChecker::check_file_dependency(
            "test-file",
            "/tmp/test.txt",
        ).await;

        match result {
            Ok(check_result) => {
                println!("文件检测结果: {:?}", check_result);
                assert_eq!(check_result.dependency_id, "test-file");
            }
            Err(e) => {
                println!("文件检测失败: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_dependency_validation() {
        let deps = create_test_dependencies();

        // 测试依赖项验证
        for dep in &deps {
            assert!(dep.validate().is_ok(), "依赖项 {} 验证失败", dep.name);
        }

        // 测试无效依赖项
        let invalid_dep = Dependency::new(
            "invalid@name".to_string(), // 包含无效字符
            "Test".to_string(),
            "1.0.0".to_string(),
            "测试".to_string(),
            DependencyLevel::Required,
            false,
            1,
            CheckMethod::File {
                path: "/test".to_string(),
            },
            "测试".to_string(),
            None,
        );

        assert!(invalid_dep.validate().is_err());
    }

    #[tokio::test]
    async fn test_dependency_check_result_creation() {
        // 测试成功结果
        let success_result = DependencyCheckResult::success(
            "test-dep".to_string(),
            Some("1.0.0".to_string()),
            100,
        );

        assert!(success_result.is_satisfied());
        assert!(!success_result.is_failed());
        assert_eq!(success_result.detected_version, Some("1.0.0".to_string()));
        assert_eq!(success_result.duration_ms, 100);

        // 测试失败结果
        let failure_result = DependencyCheckResult::failure(
            "test-dep".to_string(),
            CheckStatus::Missing,
            "未找到依赖".to_string(),
            50,
        );

        assert!(!failure_result.is_satisfied());
        assert!(failure_result.is_failed());
        assert_eq!(failure_result.status, CheckStatus::Missing);
        assert_eq!(failure_result.error_details, Some("未找到依赖".to_string()));
    }

    #[tokio::test]
    async fn test_installation_task() {
        let mut task = InstallationTask::new("test-dep".to_string());

        assert!(!task.is_completed());
        assert!(!task.is_running());
        assert!(task.status.can_start());

        task.start();
        assert!(task.is_running());
        assert_eq!(task.progress_percent, 10);

        task.mark_success();
        assert!(task.is_completed());
        assert_eq!(task.progress_percent, 100);

        // 测试任务失败
        let mut failed_task = InstallationTask::new("test-dep-2".to_string());
        failed_task.mark_failed(
            InstallErrorType::NetworkError,
            "网络连接失败".to_string(),
        );

        assert!(failed_task.is_completed());
        assert!(failed_task.error_message.is_some());
        assert!(failed_task.error_type.is_some());
    }

    /// 性能测试：测试并发检测性能
    #[tokio::test]
    async fn test_concurrent_check_performance() {
        let start_time = std::time::Instant::now();

        let deps = create_test_dependencies();
        let checker = DependencyChecker::new();

        // 并发检测所有依赖
        let mut handles = Vec::new();
        for dep in deps {
            let handle = tokio::spawn(async move {
                let checker = DependencyChecker::new();
                checker.check_dependency(&dep).await
            });
            handles.push(handle);
        }

        // 等待所有检测完成
        let mut success_count = 0;
        for handle in handles {
            match handle.await.unwrap() {
                Ok(_) => success_count += 1,
                Err(_) => {},
            }
        }

        let duration = start_time.elapsed();
        println!("并发检测 {} 个依赖耗时: {:?}", success_count, duration);

        // 并发检测应该相对快速
        assert!(duration.as_millis() < 5000, "并发检测耗时过长");
    }
}

/// 手动运行测试的辅助函数
pub async fn run_manual_dependency_test() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 开始手动依赖检测测试...");

    let deps = tests::create_test_dependencies();
    let checker = DependencyChecker::new();

    for dep in &deps {
        println!("📋 检测依赖: {}", dep.name);
        match checker.check_dependency(dep).await {
            Ok(result) => {
                println!("  ✅ 状态: {}", result.status.description());
                if let Some(version) = result.detected_version {
                    println!("  📦 版本: {}", version);
                }
                println!("  ⏱️  耗时: {}ms", result.duration_ms);
                if let Some(error) = result.error_details {
                    println!("  ❌ 错误: {}", error);
                }
            }
            Err(e) => {
                println!("  ❌ 检测失败: {}", e);
            }
        }
        println!();
    }

    println!("✨ 依赖检测测试完成!");
    Ok(())
}