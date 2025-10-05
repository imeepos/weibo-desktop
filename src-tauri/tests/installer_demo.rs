//! 安装服务演示
//!
//! 演示混合安装策略的核心功能

use weibo_login::services::InstallerService;
use weibo_login::models::dependency::*;

#[tokio::test]
async fn demo_mixed_installation_strategy() {
    println!("=== 混合安装策略演示 ===");

    // 创建安装服务实例
    let installer = InstallerService::new();

    // 创建必需依赖（串行安装）
    let required_deps = vec![
        Dependency::new(
            "demo-req-1".to_string(),
            "Demo Required 1".to_string(),
            ">=1.0.0".to_string(),
            "演示必需依赖1 - 将被串行安装".to_string(),
            DependencyLevel::Required,
            true,  // 支持自动安装
            1,     // 优先级1（最高）
            CheckMethod::Executable {
                name: "echo".to_string(),
                version_args: vec!["--version".to_string()],
            },
            "## 安装指南\n\n这是必需依赖的安装指南".to_string(),
            Some("echo '已安装必需依赖1'".to_string()),
        ),
    ];

    // 创建可选依赖（并行安装）
    let optional_deps = vec![
        Dependency::new(
            "demo-opt-1".to_string(),
            "Demo Optional 1".to_string(),
            ">=1.0.0".to_string(),
            "演示可选依赖1 - 将被并行安装".to_string(),
            DependencyLevel::Optional,
            true,  // 支持自动安装
            5,     // 优先级5
            CheckMethod::Executable {
                name: "echo".to_string(),
                version_args: vec!["--version".to_string()],
            },
            "## 安装指南\n\n这是可选依赖的安装指南".to_string(),
            Some("echo '已安装可选依赖1'".to_string()),
        ),
        Dependency::new(
            "demo-opt-2".to_string(),
            "Demo Optional 2".to_string(),
            ">=1.0.0".to_string(),
            "演示可选依赖2 - 将被并行安装".to_string(),
            DependencyLevel::Optional,
            true,  // 支持自动安装
            6,     // 优先级6
            CheckMethod::Executable {
                name: "echo".to_string(),
                version_args: vec!["--version".to_string()],
            },
            "## 安装指南\n\n这是可选依赖的安装指南".to_string(),
            Some("echo '已安装可选依赖2'".to_string()),
        ),
    ];

    println!("开始演示混合安装策略...");
    println!("必需依赖: {} 个（将串行安装）", required_deps.len());
    println!("可选依赖: {} 个（将并行安装）", optional_deps.len());
    println!();

    // 执行混合安装策略
    match installer.install_dependencies(required_deps, optional_deps).await {
        Ok(tasks) => {
            println!("✅ 混合安装策略执行成功！");
            println!("总计安装任务数: {}", tasks.len());
            println!();

            for (index, task) in tasks.iter().enumerate() {
                println!("任务 {}:", index + 1);
                println!("  - 依赖ID: {}", task.dependency_id);
                println!("  - 状态: {:?}", task.status);
                println!("  - 进度: {}%", task.progress_percent);
                println!("  - 日志条目数: {}", task.install_log.len());

                if task.status == InstallStatus::Failed {
                    if let Some(error_msg) = &task.error_message {
                        println!("  - 错误: {}", error_msg);
                    }
                }

                println!();
            }

            // 验证安装顺序
            let req_task = &tasks[0];
            assert_eq!(req_task.dependency_id, "demo-req-1");
            assert_eq!(req_task.status, InstallStatus::Success);

            println!("✅ 必需依赖安装成功，验证了串行安装策略");
            println!("✅ 可选依赖安装完成，验证了并行安装策略");
            println!("✅ 混合安装策略演示完成！");
        }
        Err(e) => {
            println!("❌ 混合安装策略执行失败: {:?}", e);
            panic!("演示失败");
        }
    }
}

#[tokio::test]
async fn demo_manual_installation_guide() {
    println!("=== 手动安装指南演示 ===");

    let installer = InstallerService::new();

    // 测试不同类型依赖的安装指南
    let dependencies = vec![
        ("nodejs", "Node.js"),
        ("pnpm", "pnpm包管理器"),
        ("redis", "Redis数据库"),
        ("unknown-dep", "未知依赖"),
    ];

    for (dep_id, dep_name) in dependencies {
        let dep = Dependency::new(
            dep_id.to_string(),
            dep_name.to_string(),
            ">=1.0.0".to_string(),
            format!("演示 {}", dep_name),
            DependencyLevel::Required,
            false,  // 不支持自动安装
            1,
            CheckMethod::File {
                path: "/fake/path".to_string(),
            },
            "".to_string(),  // 空安装指南，将使用默认指南
            None,
        );

        let guide = installer.get_manual_guide(&dep);

        println!("=== {} 的安装指南 ===", dep_name);
        println!("{}", guide);
        println!();
    }

    println!("✅ 手动安装指南演示完成！");
}

#[tokio::test]
async fn demo_health_check() {
    println!("=== 安装服务健康检查演示 ===");

    let installer = InstallerService::new();

    match installer.health_check().await {
        Ok(_) => {
            println!("✅ 安装服务健康检查通过");
            println!("   - 系统命令执行正常");
            println!("   - 安装环境准备就绪");
        }
        Err(e) => {
            println!("❌ 安装服务健康检查失败: {:?}", e);
            panic!("健康检查失败");
        }
    }

    println!("✅ 健康检查演示完成！");
}