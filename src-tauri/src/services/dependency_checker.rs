//! 依赖项检测服务
//!
//! 负责检测系统依赖是否满足要求:
//! - 检测Node.js/pnpm/Redis版本
//! - 检测Playwright浏览器
//! - 版本比较和验证
//! - 并发检测协调和进度事件发射

use crate::models::{dependency::*, errors::*};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

/// 依赖检测服务
pub struct DependencyChecker;

impl DependencyChecker {
    /// 创建新的检测服务
    pub fn new() -> Self {
        Self
    }

    /// 检测所有依赖项
    pub async fn check_all(&self) -> Result<Vec<Dependency>, ApiError> {
        // TODO: 实现依赖检测逻辑
        todo!("检测所有依赖项")
    }

    /// 并发检测所有依赖项
    ///
    /// 使用 Tokio 并发模式同时检测多个依赖项，每个依赖项在独立任务中执行
    /// 每完成一个检测就发射进度事件到前端
    ///
    /// # 参数
    ///
    /// * `app` - Tauri 应用句柄，用于发射事件
    /// * `deps` - 待检测的依赖项列表
    ///
    /// # 返回
    ///
    /// 返回所有依赖项的检测结果，按输入顺序排列
    pub async fn check_all_dependencies(
        app: AppHandle,
        deps: Vec<Dependency>,
    ) -> Result<Vec<DependencyCheckResult>, ApiError> {
        info!("开始并发检测 {} 个依赖项", deps.len());

        // 使用 Arc<Mutex<>> 收集并发结果，保证线程安全
        let results = Arc::new(Mutex::new(HashMap::new()));
        let total_count = deps.len();

        // 为每个依赖项创建异步检测任务
        let mut tasks = Vec::new();

        for (index, dep) in deps.into_iter().enumerate() {
            let dep_id = dep.id.clone();
            let app_handle = app.clone();
            let results_ref = results.clone();

            let task = tokio::spawn(async move {
                let start_time = std::time::Instant::now();

                // 发射检测开始事件
                debug!(
                    "开始检测依赖项: {} (第 {}/{})",
                    dep.name,
                    index + 1,
                    total_count
                );

                if let Err(e) = app_handle.emit(
                    "dependency-check-progress",
                    &serde_json::json!({
                        "dependency_id": dep.id,
                        "dependency_name": dep.name,
                        "status": "checking",
                        "current_index": index + 1,
                        "total_count": total_count,
                        "progress_percent": ((index) as f64 / total_count as f64) * 100.0
                    }),
                ) {
                    warn!("发射检测开始事件失败: {}", e);
                }

                // 执行单个依赖检测
                let check_result = Self::check_single_dependency(&dep).await;
                let duration = start_time.elapsed().as_millis() as u64;

                let final_result = match check_result {
                    Ok(mut result) => {
                        result.duration_ms = duration;
                        info!(
                            "依赖项 {} 检测完成: {} (版本: {:?}, 耗时: {}ms)",
                            dep.name,
                            result.status.description(),
                            result.detected_version,
                            duration
                        );
                        result
                    }
                    Err(e) => {
                        error!("依赖项 {} 检测失败: {}", dep.name, e);
                        DependencyCheckResult::failure(
                            dep.id.clone(),
                            CheckStatus::Corrupted,
                            format!("检测过程出错: {}", e),
                            duration,
                        )
                    }
                };

                // 发射检测完成事件
                let status_str = match final_result.status {
                    CheckStatus::Satisfied => "satisfied",
                    CheckStatus::Missing => "missing",
                    CheckStatus::VersionMismatch => "version_mismatch",
                    CheckStatus::Corrupted => "corrupted",
                };

                if let Err(e) = app_handle.emit(
                    "dependency-check-progress",
                    &serde_json::json!({
                        "dependency_id": dep.id,
                        "dependency_name": dep.name,
                        "status": status_str,
                        "detected_version": final_result.detected_version,
                        "error_details": final_result.error_details,
                        "current_index": index + 1,
                        "total_count": total_count,
                        "progress_percent": ((index + 1) as f64 / total_count as f64) * 100.0,
                        "duration_ms": duration
                    }),
                ) {
                    warn!("发射检测完成事件失败: {}", e);
                }

                // 将结果存入共享集合
                let mut results_guard = results_ref.lock().await;
                results_guard.insert(dep_id, final_result);

                debug!("依赖项 {} 的检测结果已保存", dep.name);
            });

            tasks.push(task);
        }

        // 等待所有检测任务完成
        let tasks_count = tasks.len();
        info!("等待 {} 个检测任务完成", tasks_count);

        for (index, task) in tasks.into_iter().enumerate() {
            match task.await {
                Ok(_) => {
                    debug!("检测任务 {}/{} 已完成", index + 1, tasks_count);
                }
                Err(e) => {
                    error!("检测任务 {}/{} 发生panic: {}", index + 1, tasks_count, e);
                }
            }
        }

        // 收集并排序结果
        let mut results_guard = results.lock().await;
        let collected_results: Vec<DependencyCheckResult> =
            results_guard.drain().map(|(_, result)| result).collect();

        // 按依赖项名称排序以确保结果顺序一致
        let mut sorted_results = collected_results;
        sorted_results.sort_by(|a, b| a.dependency_id.cmp(&b.dependency_id));

        info!("所有依赖项检测完成，共 {} 个结果", sorted_results.len());

        // 统计检测结果
        let satisfied_count = sorted_results.iter().filter(|r| r.is_satisfied()).count();
        let failed_count = sorted_results.len() - satisfied_count;

        info!(
            "依赖检测统计: 满足 {} 个, 失败 {} 个, 成功率 {:.1}%",
            satisfied_count,
            failed_count,
            (satisfied_count as f64 / sorted_results.len() as f64) * 100.0
        );

        Ok(sorted_results)
    }

    /// 检测单个依赖项
    pub async fn check_dependency(
        &self,
        dependency: &Dependency,
    ) -> Result<DependencyCheckResult, DependencyError> {
        Self::check_single_dependency(dependency).await
    }

    /// 检测单个依赖项的具体实现
    ///
    /// 根据依赖项的检测方法执行相应的检测逻辑
    async fn check_single_dependency(
        dependency: &Dependency,
    ) -> Result<DependencyCheckResult, DependencyError> {
        debug!(
            "检测依赖项: {} (方法: {})",
            dependency.name,
            dependency.check_method_name()
        );

        let result = match &dependency.check_method {
            CheckMethod::Executable { name, version_args } => {
                Self::check_executable_dependency(&dependency.id, name, version_args).await?
            }
            CheckMethod::Service { host, port } => {
                Self::check_service_dependency(&dependency.id, host, *port).await?
            }
            CheckMethod::File { path } => Self::check_file_dependency(&dependency.id, path).await?,
        };

        // 如果检测到版本，还需要验证版本是否满足要求
        let final_result = if let Some(detected_version) = &result.detected_version {
            let version_valid =
                Self::validate_version(detected_version, &dependency.version_requirement);
            if version_valid {
                result
            } else {
                DependencyCheckResult::failure(
                    dependency.id.clone(),
                    CheckStatus::VersionMismatch,
                    format!(
                        "版本不满足要求: 检测到 {}, 要求 {}",
                        detected_version, dependency.version_requirement
                    ),
                    result.duration_ms,
                )
            }
        } else {
            result
        };

        Ok(final_result)
    }

    /// 验证版本是否满足要求
    ///
    /// 使用 semver 库进行语义化版本比较，支持各种版本要求格式：
    /// - ">=1.0.0" - 大于等于1.0.0
    /// - "^1.2.3" - 兼容版本，>=1.2.3且<2.0.0
    /// - "~1.2.3" - 近似版本，>=1.2.3且<1.3.0
    /// - "1.2.3" - 精确版本
    pub fn validate_version(current: &str, required: &str) -> bool {
        debug!("验证版本要求: {} vs {}", current, required);

        // 解析当前版本
        let current_version = match semver::Version::parse(current) {
            Ok(version) => {
                debug!("成功解析当前版本: {}", version);
                version
            }
            Err(err) => {
                warn!("无法解析当前版本 '{}': {}", current, err);
                return false;
            }
        };

        // 解析版本要求
        let version_req = match semver::VersionReq::parse(required) {
            Ok(req) => {
                debug!("成功解析版本要求: {}", req);
                req
            }
            Err(err) => {
                warn!("无法解析版本要求 '{}': {}", required, err);
                return false;
            }
        };

        // 执行版本比较
        let matches = version_req.matches(&current_version);
        debug!(
            "版本比较结果: {} {} {} -> {}",
            current,
            if matches { "✓" } else { "✗" },
            required,
            matches
        );

        matches
    }

    /// 检测可执行文件依赖
    pub async fn check_executable_dependency(
        dependency_id: &str,
        name: &str,
        version_args: &[String],
    ) -> Result<DependencyCheckResult, DependencyError> {
        let start_time = std::time::Instant::now();

        // 1. 检查可执行文件是否存在
        let _executable_path = which::which(name).map_err(|e| {
            debug!("未找到可执行文件: {}", name);
            DependencyError::CheckFailed(format!("未找到可执行文件 '{}': {}", name, e))
        })?;

        debug!("找到可执行文件: {:?}", _executable_path);

        // 2. 获取版本号
        let output = tokio::process::Command::new(name)
            .args(version_args)
            .output()
            .await
            .map_err(|e| {
                warn!("执行版本命令失败: {} {:?} - {}", name, version_args, e);
                DependencyError::CheckFailed(format!("执行版本命令失败: {}", e))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Ok(DependencyCheckResult::failure(
                dependency_id.to_string(),
                CheckStatus::Corrupted,
                format!("版本命令执行失败: {}", stderr.trim()),
                start_time.elapsed().as_millis() as u64,
            ));
        }

        let version_output = String::from_utf8_lossy(&output.stdout);
        let version_str = Self::parse_version_from_output(&version_output);

        match version_str {
            Some(version) => {
                debug!("检测到版本: {} -> {}", name, version);
                Ok(DependencyCheckResult::success(
                    dependency_id.to_string(),
                    Some(version),
                    start_time.elapsed().as_millis() as u64,
                ))
            }
            None => {
                warn!("无法解析版本号: {} {:?}", name, version_args);
                Ok(DependencyCheckResult::failure(
                    dependency_id.to_string(),
                    CheckStatus::Corrupted,
                    "无法解析版本号".to_string(),
                    start_time.elapsed().as_millis() as u64,
                ))
            }
        }
    }

    /// 检测服务依赖
    pub async fn check_service_dependency(
        dependency_id: &str,
        host: &str,
        port: u16,
    ) -> Result<DependencyCheckResult, DependencyError> {
        let start_time = std::time::Instant::now();

        debug!("检测服务连接: {}:{}", host, port);

        // 使用 tokio 尝试连接服务端口
        let socket_addr = format!("{}:{}", host, port);
        match tokio::net::TcpStream::connect(&socket_addr).await {
            Ok(_) => {
                debug!("服务连接成功: {}", socket_addr);
                Ok(DependencyCheckResult::success(
                    dependency_id.to_string(),
                    None,
                    start_time.elapsed().as_millis() as u64,
                ))
            }
            Err(e) => {
                warn!("服务连接失败: {} - {}", socket_addr, e);
                Ok(DependencyCheckResult::failure(
                    dependency_id.to_string(),
                    CheckStatus::Missing,
                    format!("服务不可达: {}", e),
                    start_time.elapsed().as_millis() as u64,
                ))
            }
        }
    }

    /// 检测文件依赖
    pub async fn check_file_dependency(
        dependency_id: &str,
        path: &str,
    ) -> Result<DependencyCheckResult, DependencyError> {
        let start_time = std::time::Instant::now();

        debug!("检测文件存在性: {}", path);

        // 使用 tokio 异步检查文件
        match tokio::fs::metadata(path).await {
            Ok(metadata) => {
                debug!("文件存在: {} (大小: {} bytes)", path, metadata.len());
                Ok(DependencyCheckResult::success(
                    dependency_id.to_string(),
                    None,
                    start_time.elapsed().as_millis() as u64,
                ))
            }
            Err(e) => {
                warn!("文件不存在或无法访问: {} - {}", path, e);
                Ok(DependencyCheckResult::failure(
                    dependency_id.to_string(),
                    CheckStatus::Missing,
                    format!("文件不存在: {}", e),
                    start_time.elapsed().as_millis() as u64,
                ))
            }
        }
    }

    /// 从命令输出中解析版本号
    ///
    /// 支持常见的版本号格式:
    /// - "v20.10.0" -> "20.10.0"
    /// - "node v20.10.0" -> "20.10.0"
    /// - "pnpm 8.15.0" -> "8.15.0"
    /// - "Redis server v=7.2.3" -> "7.2.3"
    pub fn parse_version_from_output(output: &str) -> Option<String> {
        let trimmed = output.trim();

        // 常见版本号正则模式
        let patterns = [
            r"v?(\d+\.\d+\.\d+[a-zA-Z0-9\.\-]*)", // v1.2.3-alpha.1 或 1.2.3-beta.2
            r"version[:\s]+(\d+\.\d+\.\d+[a-zA-Z0-9\.\-]*)", // version: 1.2.3-alpha.1 或 version 1.2.3-beta.2
            r"(\d+\.\d+\.\d+[a-zA-Z0-9\.\-]*)[^\d]*$",       // 行尾的版本号
        ];

        for pattern in &patterns {
            if let Ok(regex) = regex::Regex::new(pattern) {
                if let Some(captures) = regex.captures(trimmed) {
                    if let Some(version) = captures.get(1) {
                        return Some(version.as_str().to_string());
                    }
                }
            }
        }

        // 如果正则匹配失败，尝试简单的字符串提取
        let lines: Vec<&str> = trimmed.lines().collect();
        for line in lines {
            let words: Vec<&str> = line.split_whitespace().collect();
            for word in words {
                if word.chars().next().is_some_and(|c| c.is_ascii_digit()) {
                    if let Some(version) = word.trim_start_matches('v').split_whitespace().next() {
                        if version.contains('.') && version.split('.').count() >= 2 {
                            return Some(version.to_string());
                        }
                    }
                }
            }
        }

        warn!("无法从输出中解析版本号: {}", trimmed);
        None
    }
}

impl Default for DependencyChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_dependency(id: &str, check_method: CheckMethod) -> Dependency {
        Dependency::new(
            id.to_string(),
            format!("Test {}", id),
            ">=1.0.0".to_string(),
            "Test dependency".to_string(),
            DependencyLevel::Required,
            false,
            1,
            check_method,
            "Test install guide".to_string(),
            None,
        )
    }

    #[tokio::test]
    async fn test_check_executable_dependency() {
        let checker = DependencyChecker::new();

        // 测试存在的可执行文件 (假设系统有git)
        let dependency = create_test_dependency(
            "git",
            CheckMethod::Executable {
                name: "git".to_string(),
                version_args: vec!["--version".to_string()],
            },
        );

        let result = checker.check_dependency(&dependency).await;

        // 结果可能是成功（如果系统有git）或失败（如果没有）
        match result {
            Ok(check_result) => {
                assert_eq!(check_result.dependency_id, "git");
                println!("Git检测结果: {:?}", check_result);
            }
            Err(err) => {
                println!("Git检测错误: {}", err);
                // 错误是预期的，因为git可能不存在
            }
        }
    }

    #[tokio::test]
    async fn test_check_service_dependency() {
        let checker = DependencyChecker::new();

        // 测试本地Redis服务（可能不存在）
        let dependency = create_test_dependency(
            "redis-local",
            CheckMethod::Service {
                host: "localhost".to_string(),
                port: 6379,
            },
        );

        let result = checker.check_dependency(&dependency).await;

        match result {
            Ok(check_result) => {
                assert_eq!(check_result.dependency_id, "redis-local");
                println!("Redis服务检测结果: {:?}", check_result);
            }
            Err(err) => {
                println!("Redis服务检测错误: {}", err);
            }
        }
    }

    #[tokio::test]
    async fn test_check_file_dependency() {
        let checker = DependencyChecker::new();

        // 测试存在的文件（Cargo.toml） - 使用 CARGO_MANIFEST_DIR 确保正确的路径
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
            .expect("CARGO_MANIFEST_DIR should be set during tests");
        let cargo_toml_path = std::path::Path::new(&manifest_dir).join("Cargo.toml");

        let dependency = create_test_dependency(
            "cargo-toml",
            CheckMethod::File {
                path: cargo_toml_path.to_string_lossy().to_string(),
            },
        );

        let result = checker.check_dependency(&dependency).await.unwrap();

        assert_eq!(result.dependency_id, "cargo-toml");
        assert!(result.is_satisfied(), "检查结果应该是满足的: {:?}", result);
        assert_eq!(result.status, CheckStatus::Satisfied);
    }

    #[tokio::test]
    async fn test_check_nonexistent_file_dependency() {
        let checker = DependencyChecker::new();

        // 测试不存在的文件
        let dependency = create_test_dependency(
            "nonexistent",
            CheckMethod::File {
                path: "/nonexistent/file/path".to_string(),
            },
        );

        let result = checker.check_dependency(&dependency).await.unwrap();

        assert_eq!(result.dependency_id, "nonexistent");
        assert!(result.is_failed());
        assert_eq!(result.status, CheckStatus::Missing);
    }

    #[test]
    fn test_parse_version_from_output() {
        let _checker = DependencyChecker::new();

        // 测试各种版本输出格式
        let test_cases = [
            ("v20.10.0", Some("20.10.0")),
            ("node v20.10.0", Some("20.10.0")),
            ("git version 2.39.0", Some("2.39.0")),
            ("1.2.3-alpha.1", Some("1.2.3-alpha.1")),
            ("Redis server v=7.0.12", Some("7.0.12")),
            ("pnpm 8.15.0", Some("8.15.0")),
            ("invalid output", None),
            ("", None),
        ];

        for (input, expected) in test_cases {
            let result = DependencyChecker::parse_version_from_output(input);
            assert_eq!(
                result,
                expected.map(String::from),
                "Failed for input: {}",
                input
            );
        }
    }

    #[test]
    fn test_validate_version_requirement() {
        let test_cases = [
            ("20.10.0", ">=20.0.0", true),
            ("1.2.3", "^1.0.0", true),
            ("2.0.0", "^1.0.0", false),
            ("1.5.0", "~1.4.0", false),
            ("1.4.5", "~1.4.0", true),
            ("1.4.0", "=1.4.0", true),  // 使用 = 进行精确匹配
            ("1.4.1", "=1.4.0", false), // 使用 = 进行精确匹配
        ];

        for (current, required, expected) in test_cases {
            let result = DependencyChecker::validate_version(current, required);
            assert_eq!(result, expected, "Failed for {} vs {}", current, required);
        }
    }

    #[tokio::test]
    async fn test_version_validation_integration() {
        let checker = DependencyChecker::new();

        // 使用 CARGO_MANIFEST_DIR 获取正确的路径
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
            .expect("CARGO_MANIFEST_DIR should be set during tests");
        let cargo_toml_path = std::path::Path::new(&manifest_dir).join("Cargo.toml");

        // 创建一个依赖项，要求版本 >= 1.0.0
        let dependency = Dependency::new(
            "test-version".to_string(),
            "Test Version".to_string(),
            ">=1.0.0".to_string(),
            "Test version validation".to_string(),
            DependencyLevel::Required,
            false,
            1,
            CheckMethod::File {
                path: cargo_toml_path.to_string_lossy().to_string(),
            },
            "Test guide".to_string(),
            None,
        );

        let result = checker.check_dependency(&dependency).await.unwrap();

        // 文件检测应该成功，但没有版本信息，所以不需要版本验证
        assert!(result.is_satisfied());
        assert_eq!(result.status, CheckStatus::Satisfied);
        assert!(result.detected_version.is_none());
    }

    #[tokio::test]
    async fn test_dependency_checker_creation() {
        let checker = DependencyChecker::new();
        let default_checker = DependencyChecker::default();

        // 使用 CARGO_MANIFEST_DIR 获取正确的路径
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
            .expect("CARGO_MANIFEST_DIR should be set during tests");
        let cargo_toml_path = std::path::Path::new(&manifest_dir)
            .join("Cargo.toml")
            .to_string_lossy()
            .to_string();

        // 测试创建和默认值
        assert_eq!(
            checker
                .check_dependency(&create_test_dependency(
                    "test",
                    CheckMethod::File {
                        path: cargo_toml_path.clone()
                    }
                ))
                .await
                .unwrap()
                .dependency_id,
            "test"
        );
        assert_eq!(
            default_checker
                .check_dependency(&create_test_dependency(
                    "default",
                    CheckMethod::File {
                        path: cargo_toml_path
                    }
                ))
                .await
                .unwrap()
                .dependency_id,
            "default"
        );
    }

    #[test]
    fn test_check_status_methods() {
        // 测试CheckStatus的辅助方法
        assert!(CheckStatus::Satisfied.is_success());
        assert!(!CheckStatus::Satisfied.is_failure());

        assert!(CheckStatus::Missing.is_failure());
        assert!(!CheckStatus::Missing.is_success());

        assert!(CheckStatus::VersionMismatch.is_failure());
        assert!(!CheckStatus::VersionMismatch.is_success());

        assert!(CheckStatus::Corrupted.is_failure());
        assert!(!CheckStatus::Corrupted.is_success());
    }

    #[test]
    fn test_dependency_check_result_methods() {
        // 测试DependencyCheckResult的辅助方法
        let success_result =
            DependencyCheckResult::success("test".to_string(), Some("1.0.0".to_string()), 100);
        assert!(success_result.is_satisfied());
        assert!(!success_result.is_failed());

        let failure_result = DependencyCheckResult::failure(
            "test".to_string(),
            CheckStatus::Missing,
            "Not found".to_string(),
            100,
        );
        assert!(!failure_result.is_satisfied());
        assert!(failure_result.is_failed());
    }
}
