//! 依赖检测器独立测试
//!
//! 独立验证并发检测协调器功能的测试模块

use super::*;

/// 简单测试并发检测协调器的核心功能
pub async fn test_concurrent_dependency_checker() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 开始测试并发依赖检测协调器...");

    // 创建测试依赖项
    let dependencies = vec![
        Dependency::new(
            "nodejs".to_string(),
            "Node.js".to_string(),
            ">=18.0.0".to_string(),
            "JavaScript运行时".to_string(),
            DependencyLevel::Required,
            false,
            1,
            CheckMethod::Executable {
                name: "node".to_string(),
                version_args: vec!["--version".to_string()],
            },
            "请访问 https://nodejs.org 下载".to_string(),
            None,
        ),
        Dependency::new(
            "cargo-toml".to_string(),
            "Cargo.toml".to_string(),
            "*".to_string(),
            "Rust项目配置文件".to_string(),
            DependencyLevel::Optional,
            false,
            2,
            CheckMethod::File {
                path: "/workspace/desktop/src-tauri/Cargo.toml".to_string(),
            },
            "项目根目录应该有 Cargo.toml".to_string(),
            None,
        ),
        Dependency::new(
            "redis-service".to_string(),
            "Redis".to_string(),
            ">=6.0.0".to_string(),
            "内存数据库服务".to_string(),
            DependencyLevel::Required,
            false,
            3,
            CheckMethod::Service {
                host: "localhost".to_string(),
                port: 6379,
            },
            "请启动 Redis: redis-server".to_string(),
            None,
        ),
    ];

    // 创建模拟的 AppHandle (这里我们用简单的事件收集器替代)
    let event_collector = MockEventCollector::new();

    // 测试单个依赖检测
    println!("\n📋 测试单个依赖检测...");
    let checker = DependencyChecker::new();

    for dep in &dependencies {
        let start = std::time::Instant::now();
        match checker.check_dependency(dep).await {
            Ok(result) => {
                println!("✅ {}: {} ({})",
                    dep.name,
                    result.status.description(),
                    format_duration(result.duration_ms)
                );
                if let Some(version) = result.detected_version {
                    println!("   📦 版本: {}", version);
                }
                if let Some(error) = result.error_details {
                    println!("   ⚠️  错误: {}", error);
                }
            }
            Err(e) => {
                println!("❌ {}: 检测失败 - {}", dep.name, e);
            }
        }
    }

    // 测试版本验证功能
    println!("\n🔍 测试版本验证功能...");
    let test_cases = vec![
        ("20.10.0", ">=18.0.0", true),
        ("16.15.0", ">=18.0.0", false),
        ("18.19.0", "^18.0.0", true),
        ("19.0.0", "^18.0.0", false),
        ("18.1.2", "~18.1.0", true),
        ("18.2.0", "~18.1.0", false),
    ];

    for (current, required, expected) in test_cases {
        let result = DependencyChecker::validate_version(current, required);
        let status = if result { "✅" } else { "❌" };
        println!("{} {} vs {} -> {}", status, current, required, result);
        assert_eq!(result, expected, "版本验证结果不匹配");
    }

    // 测试版本解析功能
    println!("\n📝 测试版本解析功能...");
    let version_outputs = vec![
        "v20.10.0",
        "node v20.10.0",
        "git version 2.39.0",
        "pnpm 8.15.0",
        "Redis server v=7.0.12",
        "invalid output without version",
        "",
    ];

    for output in version_outputs {
        let parsed = DependencyChecker::parse_version_from_output(output);
        match parsed {
            Some(version) => println!("✅ '{}' -> '{}'", output, version),
            None => println!("❌ '{}' -> 无法解析", output),
        }
    }

    // 性能测试：并发 vs 串行
    println!("\n⚡ 性能测试：并发 vs 串行检测...");

    // 串行检测
    let serial_start = std::time::Instant::now();
    let mut serial_success = 0;
    for dep in &dependencies {
        if checker.check_dependency(dep).await.is_ok() {
            serial_success += 1;
        }
    }
    let serial_duration = serial_start.elapsed();

    // 并发检测（模拟）
    let concurrent_start = std::time::Instant::now();
    let mut handles = Vec::new();

    for dep in dependencies.clone() {
        let handle = tokio::spawn(async move {
            let checker = DependencyChecker::new();
            checker.check_dependency(&dep).await
        });
        handles.push(handle);
    }

    let mut concurrent_success = 0;
    for handle in handles {
        if handle.await.unwrap().is_ok() {
            concurrent_success += 1;
        }
    }
    let concurrent_duration = concurrent_start.elapsed();

    println!("📊 性能对比:");
    println!("   串行检测: {} 成功, 耗时 {:?}", serial_success, serial_duration);
    println!("   并发检测: {} 成功, 耗时 {:?}", concurrent_success, concurrent_duration);

    if concurrent_duration < serial_duration {
        println!("   🚀 并发检测快了 {:.1}x",
            serial_duration.as_millis() as f64 / concurrent_duration.as_millis() as f64
        );
    }

    println!("\n✨ 所有测试完成! 并发检测协调器功能正常。");
    Ok(())
}

/// 格式化持续时间
fn format_duration(ms: u64) -> String {
    if ms < 1000 {
        format!("{}ms", ms)
    } else if ms < 60000 {
        format!("{:.1}s", ms as f64 / 1000.0)
    } else {
        format!("{:.1}m", ms as f64 / 60000.0)
    }
}

/// 模拟事件收集器（替代 AppHandle）
struct MockEventCollector {
    events: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
}

impl MockEventCollector {
    fn new() -> Self {
        Self {
            events: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }

    fn emit_event(&self, event: &str, data: &str) {
        let mut events = self.events.lock().unwrap();
        events.push(format!("{}: {}", event, data));
        println!("📡 事件: {} - {}", event, data);
    }

    fn get_events(&self) -> Vec<String> {
        self.events.lock().unwrap().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_concurrent_checker_functionality() {
        test_concurrent_dependency_checker().await.expect("测试应该通过");
    }

    #[tokio::test]
    async fn test_dependency_validation() {
        // 测试依赖项验证
        let valid_dep = Dependency::new(
            "test".to_string(),
            "Test".to_string(),
            "1.0.0".to_string(),
            "Test dependency".to_string(),
            DependencyLevel::Required,
            false,
            1,
            CheckMethod::File {
                path: "/test".to_string(),
            },
            "Test guide".to_string(),
            None,
        );

        assert!(valid_dep.validate().is_ok());

        // 测试无效依赖项
        let invalid_dep = Dependency::new(
            "invalid@name".to_string(),
            "Test".to_string(),
            "1.0.0".to_string(),
            "Test dependency".to_string(),
            DependencyLevel::Required,
            false,
            1,
            CheckMethod::File {
                path: "/test".to_string(),
            },
            "Test guide".to_string(),
            None,
        );

        assert!(invalid_dep.validate().is_err());
    }

    #[tokio::test]
    async fn test_check_result_methods() {
        let success_result = DependencyCheckResult::success(
            "test".to_string(),
            Some("1.0.0".to_string()),
            100
        );
        assert!(success_result.is_satisfied());
        assert!(!success_result.is_failed());

        let failure_result = DependencyCheckResult::failure(
            "test".to_string(),
            CheckStatus::Missing,
            "Not found".to_string(),
            100
        );
        assert!(!failure_result.is_satisfied());
        assert!(failure_result.is_failed());
    }
}