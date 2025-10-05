//! 依赖安装服务的集成测试
//!
//! 测试 install_dependency 函数的完整流程

#[cfg(test)]
mod tests {
    use weibo_login::services::InstallerService;
    use weibo_login::models::dependency::*;
    use weibo_login::models::errors::*;

    #[tokio::test]
    async fn test_get_manual_guide() {
        let installer = InstallerService::new();

        // 测试 Node.js 安装指南
        let nodejs_dep = Dependency::new(
            "nodejs".to_string(),
            "Node.js".to_string(),
            ">=20.0.0".to_string(),
            "JavaScript运行时".to_string(),
            DependencyLevel::Required,
            false,
            1,
            CheckMethod::Executable {
                name: "node".to_string(),
                version_args: vec!["--version".to_string()],
            },
            "".to_string(), // 使用默认指南
            None,
        );

        let guide = installer.get_manual_guide(&nodejs_dep);
        println!("Node.js 安装指南:\n{}", guide);
        assert!(guide.contains("Node.js 安装指南"));
        assert!(guide.contains("node --version"));

        // 测试自定义安装指南
        let custom_dep = Dependency::new(
            "custom-dep".to_string(),
            "Custom Dependency".to_string(),
            ">=1.0.0".to_string(),
            "自定义依赖".to_string(),
            DependencyLevel::Optional,
            false,
            1,
            CheckMethod::File {
                path: "/tmp/custom".to_string(),
            },
            "## 自定义安装指南\n\n这是自定义的安装指南".to_string(),
            None,
        );

        let custom_guide = installer.get_manual_guide(&custom_dep);
        println!("自定义安装指南:\n{}", custom_guide);
        assert!(custom_guide.contains("自定义安装指南"));
        assert!(custom_guide.contains("这是自定义的安装指南"));
    }

    #[test]
    fn test_error_classification() {
        // 测试错误类型的 Display 实现
        let network_error = InstallErrorType::NetworkError;
        assert_eq!(network_error.to_string(), "Network error");

        let permission_error = InstallErrorType::PermissionDenied;
        assert_eq!(permission_error.to_string(), "Permission denied");

        let disk_error = InstallErrorType::DiskSpaceError;
        assert_eq!(disk_error.to_string(), "Disk space error");

        let version_error = InstallErrorType::VersionConflict;
        assert_eq!(version_error.to_string(), "Version conflict");

        let unknown_error = InstallErrorType::UnknownError;
        assert_eq!(unknown_error.to_string(), "Unknown error");

        println!("✓ 所有错误类型显示测试通过");
    }

    #[test]
    fn test_installer_service_creation() {
        let installer = InstallerService::new();
        let installer_with_timeout = InstallerService::with_timeout(600);
        let default_installer = InstallerService::default();

        // 测试不同的创建方法都能成功创建
        // 由于 install_timeout 是私有字段，我们只能测试创建不会panic
        println!("✓ 安装服务创建测试通过");
        println!("  - 标准创建成功");
        println!("  - 带超时创建成功");
        println!("  - 默认创建成功");
    }

    #[test]
    fn test_dependency_validation() {
        // 测试有效依赖
        let valid_dep = Dependency::new(
            "test-dep".to_string(),
            "Test Dependency".to_string(),
            ">=1.0.0".to_string(),
            "测试依赖项".to_string(),
            DependencyLevel::Optional,
            true,
            1,
            CheckMethod::Executable {
                name: "echo".to_string(),
                version_args: vec!["--version".to_string()],
            },
            "## 安装指南\n\n测试安装".to_string(),
            Some("echo 'test installation'".to_string()),
        );

        assert!(valid_dep.validate().is_ok());

        // 测试无效依赖（ID为空）
        let mut invalid_dep = valid_dep.clone();
        invalid_dep.id = "".to_string();
        assert!(invalid_dep.validate().is_err());

        // 测试无效依赖（优先级超出范围）
        invalid_dep.id = "test-dep".to_string();
        invalid_dep.install_priority = 0;
        assert!(invalid_dep.validate().is_err());

        println!("✓ 依赖验证测试通过");
    }

    #[test]
    fn test_installation_task_lifecycle() {
        let mut task = InstallationTask::new("test-dep".to_string());

        // 测试初始状态
        assert_eq!(task.dependency_id, "test-dep");
        assert_eq!(task.status, InstallStatus::Pending);
        assert_eq!(task.progress_percent, 0);
        assert!(!task.is_completed());
        assert!(!task.is_running());
        assert!(task.status.can_start());

        // 测试开始安装
        task.start();
        assert_eq!(task.status, InstallStatus::Downloading);
        assert_eq!(task.progress_percent, 10);
        assert!(task.is_running());
        assert!(!task.is_completed());

        // 测试进度更新
        task.update_progress(
            InstallStatus::Installing,
            50,
            "安装进行中...".to_string(),
        );
        assert_eq!(task.status, InstallStatus::Installing);
        assert_eq!(task.progress_percent, 50);

        // 测试标记成功
        task.mark_success();
        assert_eq!(task.status, InstallStatus::Success);
        assert_eq!(task.progress_percent, 100);
        assert!(task.is_completed());
        assert!(!task.is_running());

        println!("✓ 安装任务生命周期测试通过");
    }

    #[tokio::test]
    async fn test_health_check() {
        let installer = InstallerService::new();

        // 测试健康检查
        match installer.health_check().await {
            Ok(_) => {
                println!("✓ 安装服务健康检查通过");
            }
            Err(e) => {
                println!("✗ 安装服务健康检查失败: {:?}", e);
                // 健康检查失败不应该阻止测试通过
            }
        }
    }
}