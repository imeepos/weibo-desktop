//! 依赖管理Tauri命令
//!
//! 提供前端调用的依赖管理接口:
//! - check_dependencies: 检测所有依赖
//! - install_dependency: 安装单个依赖
//! - query_dependency_status: 查询依赖状态
//! - trigger_manual_check: 手动触发检测

use crate::models::dependency::*;
use crate::models::errors::DependencyError;
use crate::services::{ConfigService, DependencyChecker, InstallerService};
use tracing::{error, info, warn};

/// 获取预定义的依赖项列表
fn get_predefined_dependencies() -> Vec<Dependency> {
    // 从 .env 读取 Redis 配置,用于环境检测
    let (redis_host, redis_port) = match ConfigService::load_redis_config() {
        Ok(config) => {
            info!(
                "从配置文件读取 Redis 检测地址: {}:{}",
                config.host, config.port
            );
            (config.host, config.port)
        }
        Err(e) => {
            warn!("无法读取 Redis 配置,使用默认值 localhost:6379: {}", e);
            ("localhost".to_string(), 6379)
        }
    };

    vec![
        Dependency::new(
            "nodejs".to_string(),
            "Node.js".to_string(),
            ">=20.0.0".to_string(),
            "JavaScript运行时,用于执行前端构建和Playwright自动化脚本".to_string(),
            DependencyLevel::Required,
            false,
            1,
            CheckMethod::Executable {
                name: "node".to_string(),
                version_args: vec!["--version".to_string()],
            },
            "## 安装Node.js\n\n1. 访问 [Node.js官网](https://nodejs.org/)\n2. 下载LTS版本(20.x)\n3. 运行安装程序\n4. 验证: `node --version`".to_string(),
            None,
        ),
        Dependency::new(
            "pnpm".to_string(),
            "pnpm".to_string(),
            ">=8.0.0".to_string(),
            "快速、节省磁盘空间的包管理器".to_string(),
            DependencyLevel::Required,
            true,
            2,
            CheckMethod::Executable {
                name: "pnpm".to_string(),
                version_args: vec!["--version".to_string()],
            },
            "".to_string(),
            Some("npm install -g pnpm".to_string()),
        ),
        Dependency::new(
            "redis".to_string(),
            "Redis".to_string(),
            ">=7.0.0".to_string(),
            "内存数据库,用于存储会话和缓存".to_string(),
            DependencyLevel::Required,
            false,
            3,
            CheckMethod::Service {
                host: redis_host,
                port: redis_port,
            },
            "## 安装Redis\n\n请参考官方文档根据您的操作系统安装Redis".to_string(),
            None,
        ),
        Dependency::new(
            "playwright-browsers".to_string(),
            "Playwright浏览器".to_string(),
            ">=1.0.0".to_string(),
            "用于自动化测试的浏览器引擎".to_string(),
            DependencyLevel::Optional,
            true,
            4,
            CheckMethod::Executable {
                name: "playwright".to_string(),
                version_args: vec!["--version".to_string()],
            },
            "".to_string(),
            Some("npx playwright install".to_string()),
        ),
    ]
}

/// 检测所有依赖项
#[tauri::command]
pub async fn check_dependencies(
    app_handle: tauri::AppHandle,
    _state: tauri::State<'_, crate::state::AppState>,
) -> Result<Vec<DependencyCheckResult>, String> {
    info!("开始检测所有依赖项");

    let dependencies = get_predefined_dependencies();

    let results = DependencyChecker::check_all_dependencies(app_handle, dependencies)
        .await
        .map_err(|e| {
            error!("依赖检测失败: {}", e);
            format!("依赖检测失败: {}", e)
        })?;

    info!("依赖检测完成,共检测 {} 个依赖", results.len());
    Ok(results)
}

/// 安装单个依赖项
#[tauri::command]
pub async fn install_dependency(
    dependency_id: String,
    force: bool,
    app_handle: tauri::AppHandle,
    _state: tauri::State<'_, crate::state::AppState>,
) -> Result<InstallationTask, DependencyError> {
    info!("开始安装依赖: {} (force: {})", dependency_id, force);

    // 查找依赖项定义
    let dependencies = get_predefined_dependencies();
    let dependency = dependencies
        .iter()
        .find(|d| d.id == dependency_id)
        .ok_or_else(|| {
            error!("未找到依赖项: {}", dependency_id);
            DependencyError::CheckFailed(format!("未找到依赖项: {}", dependency_id))
        })?;

    // 调用安装服务
    let installer = InstallerService::new();
    let task = installer
        .install_dependency(app_handle, dependency, force)
        .await?;

    info!(
        "安装任务已创建: {} (task_id: {})",
        dependency_id, task.task_id
    );
    Ok(task)
}

/// 查询依赖状态 (实时检测)
#[tauri::command]
pub async fn query_dependency_status(
    dependency_id: Option<String>,
    _state: tauri::State<'_, crate::state::AppState>,
) -> Result<Vec<DependencyCheckResult>, String> {
    let dependencies = get_predefined_dependencies();
    let checker = DependencyChecker::new();

    let results = if let Some(id) = dependency_id {
        // 查询单个依赖
        dependencies
            .into_iter()
            .filter(|dep| dep.id == id)
            .filter_map(|dep| {
                futures::executor::block_on(async { checker.check_dependency(&dep).await.ok() })
            })
            .collect()
    } else {
        // 查询所有依赖 (并发检测)
        let mut results = Vec::new();
        for dep in dependencies {
            if let Ok(result) = checker.check_dependency(&dep).await {
                results.push(result);
            }
        }
        results.sort_by(|a, b| a.dependency_id.cmp(&b.dependency_id));
        results
    };

    Ok(results)
}

/// 手动触发依赖检测
#[tauri::command]
pub async fn trigger_manual_check(
    app_handle: tauri::AppHandle,
    _state: tauri::State<'_, crate::state::AppState>,
) -> Result<Vec<DependencyCheckResult>, DependencyError> {
    info!("用户手动触发依赖检测");

    let dependencies = get_predefined_dependencies();

    let results = DependencyChecker::check_all_dependencies(app_handle, dependencies)
        .await
        .map_err(|e| {
            error!("手动检测失败: {}", e);
            DependencyError::CheckFailed(format!("手动检测失败: {}", e))
        })?;

    let satisfied_count = results.iter().filter(|r| r.is_satisfied()).count();
    info!(
        "手动检测完成: 总计 {} 个依赖, 满足 {} 个",
        results.len(),
        satisfied_count
    );

    Ok(results)
}
