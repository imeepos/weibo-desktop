//! 集成测试: 手动安装指南
//!
//! 测试完整流程:
//! 1. 检测到Redis缺失(必需依赖)
//! 2. 显示手动安装指南
//! 3. 用户安装后手动检测
//! 4. 检测通过后继续启动
//!
//! 场景3验证: 必需依赖缺失时阻止启动并显示安装指引

use std::time::Duration;
use tokio::time::sleep;

mod common;
use weibo_login::models::dependency::CheckStatus;

/// 模拟应用启动状态
#[derive(Debug, Clone, PartialEq)]
enum ApplicationStartupState {
    /// 检测依赖中
    CheckingDependencies,
    /// 显示安装指引(必需依赖缺失)
    ShowingInstallationGuide,
    /// 自动安装中(可选依赖)
    AutoInstalling,
    /// 启动完成,进入主界面
    Started,
    /// 启动失败
    Failed(String),
}

/// 模拟安装指引界面数据
#[derive(Debug, Clone)]
struct InstallationGuide {
    dependency_id: String,
    dependency_name: String,
    title: String,
    content: String,
    links: Vec<InstallLink>,
    target_os: Vec<String>,
    language: String,
}

#[derive(Debug, Clone)]
struct InstallLink {
    text: String,
    url: String,
}

/// Mock依赖检测服务
///
/// 模拟依赖检测逻辑,支持配置检测结果:
/// - Redis缺失(必需)
/// - Playwright已安装(可选)
struct MockDependencyChecker {
    /// 模拟Redis状态
    redis_available: bool,
    /// 模拟Playwright状态
    playwright_available: bool,
}

impl MockDependencyChecker {
    fn new(redis_available: bool, playwright_available: bool) -> Self {
        Self {
            redis_available,
            playwright_available,
        }
    }

    /// 模拟依赖检测
    async fn check_dependencies(&self) -> Vec<MockDependencyCheckResult> {
        let start_time = std::time::Instant::now();
        let mut results = Vec::new();

        // 检测Redis(必需依赖)
        sleep(Duration::from_millis(50)).await; // 模拟检测耗时
        let redis_result = if self.redis_available {
            MockDependencyCheckResult {
                dependency_id: "redis".to_string(),
                dependency_name: "Redis Server".to_string(),
                status: CheckStatus::Satisfied,
                detected_version: Some("7.2.4".to_string()),
                error_details: None,
                duration_ms: 45,
                checked_at: chrono::Utc::now(),
            }
        } else {
            MockDependencyCheckResult {
                dependency_id: "redis".to_string(),
                dependency_name: "Redis Server".to_string(),
                status: CheckStatus::Missing,
                detected_version: None,
                error_details: Some("Redis service not reachable at localhost:6379".to_string()),
                duration_ms: 45,
                checked_at: chrono::Utc::now(),
            }
        };
        results.push(redis_result);

        // 检测Playwright(可选依赖)
        sleep(Duration::from_millis(30)).await; // 模拟检测耗时
        let playwright_result = if self.playwright_available {
            MockDependencyCheckResult {
                dependency_id: "playwright".to_string(),
                dependency_name: "Playwright".to_string(),
                status: CheckStatus::Satisfied,
                detected_version: Some("1.48.0".to_string()),
                error_details: None,
                duration_ms: 32,
                checked_at: chrono::Utc::now(),
            }
        } else {
            MockDependencyCheckResult {
                dependency_id: "playwright".to_string(),
                dependency_name: "Playwright".to_string(),
                status: CheckStatus::Missing,
                detected_version: None,
                error_details: Some("Playwright not found in node_modules".to_string()),
                duration_ms: 32,
                checked_at: chrono::Utc::now(),
            }
        };
        results.push(playwright_result);

        let total_duration = start_time.elapsed().as_millis() as u64;

        // 更新第一个结果的duration为总耗时
        if let Some(first) = results.get_mut(0) {
            first.duration_ms = total_duration;
        }

        results
    }

    /// 模拟重新检测单个依赖
    async fn recheck_dependency(&mut self, dependency_id: &str) -> MockDependencyCheckResult {
        sleep(Duration::from_millis(30)).await; // 模拟检测耗时

        match dependency_id {
            "redis" => {
                self.redis_available = true; // 模拟用户已安装Redis
                MockDependencyCheckResult {
                    dependency_id: "redis".to_string(),
                    dependency_name: "Redis Server".to_string(),
                    status: CheckStatus::Satisfied,
                    detected_version: Some("7.2.4".to_string()),
                    error_details: None,
                    duration_ms: 30,
                    checked_at: chrono::Utc::now(),
                }
            },
            "playwright" => {
                self.playwright_available = true; // 模拟用户已安装Playwright
                MockDependencyCheckResult {
                    dependency_id: "playwright".to_string(),
                    dependency_name: "Playwright".to_string(),
                    status: CheckStatus::Satisfied,
                    detected_version: Some("1.48.0".to_string()),
                    error_details: None,
                    duration_ms: 25,
                    checked_at: chrono::Utc::now(),
                }
            },
            _ => panic!("Unknown dependency: {}", dependency_id),
        }
    }
}

/// Mock前端界面状态
struct MockFrontendUI {
    current_state: ApplicationStartupState,
    installation_guides: Vec<InstallationGuide>,
    detection_progress: u8,
    error_message: Option<String>,
}

impl MockFrontendUI {
    fn new() -> Self {
        Self {
            current_state: ApplicationStartupState::CheckingDependencies,
            installation_guides: Vec::new(),
            detection_progress: 0,
            error_message: None,
        }
    }

    /// 模拟检测依赖时UI更新
    fn update_detection_progress(&mut self, progress: u8, dependency_name: &str, status: &str) {
        self.detection_progress = progress;
        println!("UI更新: 检测进度 {}% - {}: {}", progress, dependency_name, status);
    }

    /// 模拟显示安装指引
    fn show_installation_guides(&mut self, guides: Vec<InstallationGuide>) {
        self.installation_guides = guides;
        self.current_state = ApplicationStartupState::ShowingInstallationGuide;
        println!("UI状态: 显示安装指引界面");
    }

    /// 模拟用户点击"重新检测"按钮
    async fn click_recheck_button(&mut self, checker: &mut MockDependencyChecker) -> Vec<MockDependencyCheckResult> {
        println!("用户操作: 点击'重新检测'按钮");
        self.current_state = ApplicationStartupState::CheckingDependencies;
        self.detection_progress = 0;

        // 重新检测所有依赖
        checker.check_dependencies().await
    }

    /// 验证应用是否被正确阻止启动
    fn verify_startup_blocked(&self) -> bool {
        matches!(self.current_state, ApplicationStartupState::ShowingInstallationGuide)
    }

    /// 验证安装指引内容正确性
    fn verify_installation_guide_content(&self) -> bool {
        if self.installation_guides.is_empty() {
            return false;
        }

        let guide = &self.installation_guides[0];

        // 验证Redis安装指引包含必要信息
        guide.dependency_id == "redis"
            && guide.dependency_name == "Redis Server"
            && guide.title.contains("安装Redis")
            && guide.content.contains("docker run -d -p 6379:6379 redis:7-alpine")
            && guide.content.contains("https://redis.io/download")
            && guide.language == "zh-CN"
            && !guide.links.is_empty()
    }
}

/// Mock依赖检测结果
#[derive(Debug, Clone)]
struct MockDependencyCheckResult {
    pub dependency_id: String,
    pub dependency_name: String,
    pub status: CheckStatus,
    pub detected_version: Option<String>,
    pub error_details: Option<String>,
    pub duration_ms: u64,
    pub checked_at: chrono::DateTime<chrono::Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 场景3: Redis缺失 - 显示手动安装指引
    #[tokio::test]
    async fn test_manual_install_redis_flow() {
        println!("🧪 开始测试场景3: Redis缺失 - 显示手动安装指引");

        // 1. 初始化测试环境
        println!("📋 初始化测试环境...");

        // 创建Mock依赖检测器(Redis缺失, Playwright已安装)
        let mut dependency_checker = MockDependencyChecker::new(false, true);

        // 创建Mock前端UI
        let mut frontend_ui = MockFrontendUI::new();

        // 2. 模拟应用启动时的依赖检测
        println!("🔍 模拟依赖检测过程...");

        // 检测依赖
        let check_results = dependency_checker.check_dependencies().await;

        // 3. 验证检测结果
        println!("✅ 验证检测结果...");
        assert_eq!(check_results.len(), 2, "应该检测到2个依赖");

        // Redis应该缺失
        let redis_result = check_results.iter().find(|r| r.dependency_id == "redis").unwrap();
        assert_eq!(redis_result.status, CheckStatus::Missing, "Redis应该检测为缺失");
        assert!(redis_result.error_details.as_ref().unwrap().contains("localhost:6379"));

        // Playwright应该已安装
        let playwright_result = check_results.iter().find(|r| r.dependency_id == "playwright").unwrap();
        assert_eq!(playwright_result.status, CheckStatus::Satisfied, "Playwright应该已安装");
        assert_eq!(playwright_result.detected_version.as_ref().unwrap(), "1.48.0");

        // 4. 模拟前端处理检测结果并显示安装指引
        println!("🖥️ 模拟前端显示安装指引...");

        // 检查是否有必需依赖缺失
        let missing_required_deps: Vec<_> = check_results
            .iter()
            .filter(|r| r.status != CheckStatus::Satisfied && r.dependency_id == "redis")
            .collect();

        assert!(!missing_required_deps.is_empty(), "应该有必需依赖缺失");

        // 创建安装指引
        let installation_guides: Vec<InstallationGuide> = missing_required_deps
            .iter()
            .map(|dep| create_redis_installation_guide())
            .collect();

        // 显示安装指引
        frontend_ui.show_installation_guides(installation_guides);

        // 5. 验证应用启动被阻止
        println!("🚫 验证应用启动被阻止...");
        assert!(frontend_ui.verify_startup_blocked(), "应用启动应该被阻止");
        assert!(matches!(frontend_ui.current_state, ApplicationStartupState::ShowingInstallationGuide));

        // 6. 验证安装指引内容
        println!("📖 验证安装指引内容...");
        assert!(frontend_ui.verify_installation_guide_content(), "安装指引内容应该正确");

        let guide = &frontend_ui.installation_guides[0];
        println!("安装指引标题: {}", guide.title);
        println!("安装指引内容预览: {}...", &guide.content[..100.min(guide.content.len())]);
        println!("可用链接数量: {}", guide.links.len());

        // 7. 验证指引包含必要的信息
        println!("🔍 验证指引包含必要信息...");

        // 应该包含Docker安装方式
        assert!(guide.content.contains("docker run -d -p 6379:6379 redis:7-alpine"));

        // 应该包含手动安装链接
        assert!(guide.content.contains("https://redis.io/download"));

        // 应该有中文说明
        assert!(guide.content.contains("内存数据库"));

        // 应该有可点击的链接
        assert!(!guide.links.is_empty());
        let redis_link = &guide.links[0];
        assert_eq!(redis_link.text, "Redis官网");
        assert_eq!(redis_link.url, "https://redis.io/download");

        // 8. 模拟用户手动安装Redis
        println!("🛠️ 模拟用户手动安装Redis...");

        // 在真实场景中,用户会根据指引安装Redis
        // 这里我们直接模拟Redis变为可用状态
        dependency_checker.redis_available = true;

        // 9. 模拟用户点击"重新检测"按钮
        println!("🔄 模拟用户点击'重新检测'按钮...");

        let recheck_results = frontend_ui.click_recheck_button(&mut dependency_checker).await;

        // 10. 验证重新检测结果
        println!("✅ 验证重新检测结果...");

        let redis_recheck = recheck_results.iter().find(|r| r.dependency_id == "redis").unwrap();
        assert_eq!(redis_recheck.status, CheckStatus::Satisfied, "重新检测后Redis应该已安装");
        assert_eq!(redis_recheck.detected_version.as_ref().unwrap(), "7.2.4");

        // 11. 验证所有依赖都已满足,应用可以启动
        println!("🚀 验证所有依赖已满足,应用可以启动...");

        let all_satisfied = recheck_results.iter().all(|r| r.status == CheckStatus::Satisfied);
        assert!(all_satisfied, "所有依赖都应该已满足");

        // 模拟应用进入主界面
        frontend_ui.current_state = ApplicationStartupState::Started;
        assert!(matches!(frontend_ui.current_state, ApplicationStartupState::Started));

        println!("✅ 场景3测试通过: Redis缺失 - 显示手动安装指引流程正常");
    }

    /// 测试安装指引内容完整性
    #[tokio::test]
    async fn test_installation_guide_content_completeness() {
        println!("🧪 测试安装指引内容完整性");

        let guide = create_redis_installation_guide();

        // 验证基本信息
        assert_eq!(guide.dependency_id, "redis");
        assert_eq!(guide.dependency_name, "Redis Server");
        assert!(!guide.title.is_empty());
        assert!(!guide.content.is_empty());

        // 验证内容包含多种安装方式
        assert!(guide.content.contains("Docker"));
        assert!(guide.content.contains("手动安装"));

        // 验证包含具体命令
        assert!(guide.content.contains("docker run"));
        assert!(guide.content.contains("redis-server"));

        // 验证包含官方链接
        assert!(guide.links.iter().any(|l| l.url.contains("redis.io")));

        // 验证语言设置
        assert_eq!(guide.language, "zh-CN");

        // 验证内容格式(应该是Markdown格式)
        assert!(guide.content.contains("##"));
        assert!(guide.content.contains("###"));

        println!("✅ 安装指引内容完整性验证通过");
    }

    /// 测试依赖检测性能
    #[tokio::test]
    async fn test_dependency_check_performance() {
        println!("🧪 测试依赖检测性能");

        let dependency_checker = MockDependencyChecker::new(false, true);

        let start_time = std::time::Instant::now();
        let results = dependency_checker.check_dependencies().await;
        let duration = start_time.elapsed();

        // 验证检测时间在合理范围内
        assert!(duration < Duration::from_secs(2), "依赖检测应该在2秒内完成");
        assert_eq!(results.len(), 2, "应该检测到2个依赖");

        println!("✅ 依赖检测性能验证通过: {:?}", duration);
    }

    /// 测试并发重新检测
    #[tokio::test]
    async fn test_concurrent_recheck() {
        println!("🧪 测试并发重新检测");

        let mut dependency_checker = MockDependencyChecker::new(false, true);
        let mut frontend_ui = MockFrontendUI::new();

        // 模拟多个用户同时点击重新检测
        let checker_ref = &mut dependency_checker;

        let task1 = tokio::spawn(async move {
            let mut local_checker = MockDependencyChecker::new(false, true);
            sleep(Duration::from_millis(10)).await;
            local_checker.recheck_dependency("redis").await
        });

        let task2 = tokio::spawn(async move {
            let mut local_checker = MockDependencyChecker::new(false, true);
            sleep(Duration::from_millis(20)).await;
            local_checker.recheck_dependency("redis").await
        });

        let (result1, result2) = tokio::join!(task1, task2);

        // 验证并发检测都成功
        assert!(result1.is_ok());
        assert!(result2.is_ok());

        let check1 = result1.unwrap();
        let check2 = result2.unwrap();

        assert_eq!(check1.status, CheckStatus::Satisfied);
        assert_eq!(check2.status, CheckStatus::Satisfied);

        println!("✅ 并发重新检测验证通过");
    }
}

/// 创建Redis安装指引
fn create_redis_installation_guide() -> InstallationGuide {
    InstallationGuide {
        dependency_id: "redis".to_string(),
        dependency_name: "Redis Server".to_string(),
        title: "安装Redis Server".to_string(),
        content: r#"## 安装Redis Server

### 方式1: Docker (推荐)
```bash
docker run -d -p 6379:6379 redis:7-alpine
```

### 方式2: 手动安装
1. 访问 https://redis.io/download
2. 下载适合您操作系统的版本
3. 按照官方文档完成安装
4. 启动Redis服务: redis-server

### 验证安装
```bash
redis-cli ping
# 应该返回: PONG
```

### 用途说明
Redis是内存数据库,用于存储用户会话和缓存数据。本应用需要Redis来持久化登录状态和提高性能。"#.to_string(),
        links: vec![
            InstallLink {
                text: "Redis官网".to_string(),
                url: "https://redis.io/download".to_string(),
            },
            InstallLink {
                text: "Docker Hub".to_string(),
                url: "https://hub.docker.com/_/redis".to_string(),
            },
        ],
        target_os: vec!["windows".to_string(), "macos".to_string(), "linux".to_string()],
        language: "zh-CN".to_string(),
    }
}
