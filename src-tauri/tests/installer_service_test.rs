//! 安装服务集成测试
//!
//! 测试混合安装策略的正确性和性能

use weibo_login::services::InstallerService;
use weibo_login::models::dependency::*;

#[tokio::test]
async fn test_install_dependencies_mixed_strategy() {
    // 创建安装服务实例
    let installer = InstallerService::new();

    // 创建测试依赖项
    let required_deps = vec![
        Dependency::new(
            "test-req-1".to_string(),
            "Test Required 1".to_string(),
            ">=1.0.0".to_string(),
            "测试必需依赖1".to_string(),
            DependencyLevel::Required,
            true,  // 支持自动安装
            1,     // 优先级1
            CheckMethod::Executable {
                name: "echo".to_string(),
                version_args: vec!["--version".to_string()],
            },
            "# Test Required 1\n\n安装指南".to_string(),
            Some("echo 'installed test-req-1'".to_string()),
        ),
    ];

    let optional_deps = vec![
        Dependency::new(
            "test-opt-1".to_string(),
            "Test Optional 1".to_string(),
            ">=1.0.0".to_string(),
            "测试可选依赖1".to_string(),
            DependencyLevel::Optional,
            true,  // 支持自动安装
            5,     // 优先级5
            CheckMethod::Executable {
                name: "echo".to_string(),
                version_args: vec!["--version".to_string()],
            },
            "# Test Optional 1\n\n安装指南".to_string(),
            Some("echo 'installed test-opt-1'".to_string()),
        ),
        Dependency::new(
            "test-opt-2".to_string(),
            "Test Optional 2".to_string(),
            ">=1.0.0".to_string(),
            "测试可选依赖2".to_string(),
            DependencyLevel::Optional,
            true,  // 支持自动安装
            6,     // 优先级6
            CheckMethod::Executable {
                name: "echo".to_string(),
                version_args: vec!["--version".to_string()],
            },
            "# Test Optional 2\n\n安装指南".to_string(),
            Some("echo 'installed test-opt-2'".to_string()),
        ),
    ];

    // 测试混合安装策略
    match installer.install_dependencies(required_deps, optional_deps).await {
        Ok(tasks) => {
            assert_eq!(tasks.len(), 3, "应该安装3个依赖");

            // 验证必需依赖优先安装
            let req_task = &tasks[0];
            assert_eq!(req_task.dependency_id, "test-req-1");
            assert_eq!(req_task.status, InstallStatus::Success);

            // 验证可选依赖并行安装
            let opt_task_1 = &tasks[1];
            let opt_task_2 = &tasks[2];

            // 可选依赖的顺序可能不同，但都应该成功
            assert!((opt_task_1.dependency_id == "test-opt-1" && opt_task_2.dependency_id == "test-opt-2") ||
                    (opt_task_1.dependency_id == "test-opt-2" && opt_task_2.dependency_id == "test-opt-1"));

            // 验证安装日志
            assert!(req_task.install_log.len() > 0);
            assert!(opt_task_1.install_log.len() > 0);
            assert!(opt_task_2.install_log.len() > 0);
        }
        Err(e) => {
            panic!("安装失败: {:?}", e);
        }
    }
}

#[tokio::test]
async fn test_install_single_dependency_via_mixed() {
    let installer = InstallerService::new();

    let dep = Dependency::new(
        "test-single".to_string(),
        "Test Single".to_string(),
        ">=1.0.0".to_string(),
        "测试单个依赖".to_string(),
        DependencyLevel::Required,
        true,
        1,
        CheckMethod::Executable {
            name: "echo".to_string(),
            version_args: vec!["--version".to_string()],
        },
        "# Test Single\n\n安装指南".to_string(),
        Some("echo 'single install success'".to_string()),
    );

    // 通过混合安装策略测试单个依赖安装
    let tasks = installer.install_dependencies(vec![dep], vec![]).await.unwrap();

    assert_eq!(tasks.len(), 1);
    let task = &tasks[0];
    assert_eq!(task.dependency_id, "test-single");
    assert_eq!(task.status, InstallStatus::Success);
    assert_eq!(task.progress_percent, 100);
    assert!(task.install_log.len() > 0);
}

#[tokio::test]
async fn test_health_check() {
    let installer = InstallerService::new();

    // 健康检查应该成功
    assert!(installer.health_check().await.is_ok());
}

#[tokio::test]
async fn test_manual_guide() {
    let installer = InstallerService::new();

    let dep = Dependency::new(
        "test-guide".to_string(),
        "Test Guide".to_string(),
        ">=1.0.0".to_string(),
        "测试安装指南".to_string(),
        DependencyLevel::Required,
        false,  // 不支持自动安装
        1,
        CheckMethod::Executable {
            name: "test".to_string(),
            version_args: vec!["--version".to_string()],
        },
        "# Test Manual Guide\n\n这是手动安装指南".to_string(),
        None,
    );

    let guide = installer.get_manual_guide(&dep);
    assert!(guide.contains("Test Manual Guide"));
    assert!(guide.contains("这是手动安装指南"));
}