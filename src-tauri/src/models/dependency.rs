//! 依赖项数据模型
//!
//! 定义系统依赖的核心数据结构,遵循 data-model.md 规范:
//! - `Dependency`: 单个依赖项的完整信息
//! - `DependencyLevel`: 依赖重要性级别 (Required/Optional)
//! - `CheckMethod`: 依赖检测方法 (Executable/Service/File)
//! - `DependencyCheckResult`: 依赖检测结果
//! - `InstallationTask`: 安装任务状态
//!
//! # 设计原则
//!
//! 遵循 Constitution 原则:
//! 1. **存在即合理**: 每个字段都有明确用途,无冗余属性
//! 2. **优雅即简约**: 结构体命名自文档化,使用枚举表达业务语义
//! 3. **性能即艺术**: 使用 serde 属性优化序列化性能
//! 4. **错误处理**: 所有错误都有明确分类和上下文信息
//! 5. **日志安全**: 避免在日志中记录敏感信息

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::models::errors::InstallErrorType;

/// 依赖重要性级别
///
/// 区分必需依赖和可选依赖,影响应用启动策略
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DependencyLevel {
    /// 必需依赖: 缺失时阻止应用启动
    Required,
    /// 可选依赖: 缺失时显示警告但允许启动
    Optional,
}

/// 依赖检测方法类型
///
/// 支持3种检测方式,覆盖不同的依赖类型检测场景
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CheckMethod {
    /// 检测可执行文件(PATH中查找)
    Executable {
        /// 可执行文件名称(如: "node", "redis-server")
        name: String,
        /// 获取版本号的命令参数(如: ["--version"])
        version_args: Vec<String>,
    },
    /// 检测服务端口监听
    Service {
        /// 主机地址(默认"localhost")
        host: String,
        /// 端口号(如: 6379 for Redis)
        port: u16,
    },
    /// 检测文件或目录存在性
    File {
        /// 文件绝对路径或相对于项目根的路径
        path: String,
    },
}

/// 依赖项定义
///
/// 代表应用运行所需的外部组件或资源的元数据配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    /// 依赖项唯一标识符(如: "nodejs", "redis", "playwright")
    pub id: String,

    /// 人类可读的依赖名称(如: "Node.js", "Redis Server")
    pub name: String,

    /// 版本要求(semver格式,如: ">=20.0.0", "^7.0.0")
    pub version_requirement: String,

    /// 依赖用途说明(用于安装指引展示)
    pub description: String,

    /// 重要性级别
    pub level: DependencyLevel,

    /// 是否支持自动在线安装
    pub auto_installable: bool,

    /// 安装优先级(1-10,数字越小优先级越高,仅影响串行安装顺序)
    pub install_priority: u8,

    /// 检测方法类型
    pub check_method: CheckMethod,

    /// 手动安装指引(Markdown格式,当auto_installable=false时使用)
    pub install_guide: String,

    /// 自动安装命令(当auto_installable=true时使用,如: "npm install -g pnpm")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub install_command: Option<String>,
}

impl Dependency {
    /// 创建新的依赖项定义
    ///
    /// # 参数
    ///
    /// * `id` - 依赖项唯一标识符
    /// * `name` - 人类可读的依赖名称
    /// * `version_requirement` - 版本要求(semver格式)
    /// * `description` - 依赖用途说明
    /// * `level` - 重要性级别
    /// * `auto_installable` - 是否支持自动安装
    /// * `install_priority` - 安装优先级(1-10)
    /// * `check_method` - 检测方法
    /// * `install_guide` - 手动安装指引
    /// * `install_command` - 自动安装命令(可选)
    ///
    /// # 返回
    ///
    /// 返回一个新的Dependency实例
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: String,
        name: String,
        version_requirement: String,
        description: String,
        level: DependencyLevel,
        auto_installable: bool,
        install_priority: u8,
        check_method: CheckMethod,
        install_guide: String,
        install_command: Option<String>,
    ) -> Self {
        Self {
            id,
            name,
            version_requirement,
            description,
            level,
            auto_installable,
            install_priority,
            check_method,
            install_guide,
            install_command,
        }
    }

    /// 验证依赖项配置的有效性
    ///
    /// # 返回
    ///
    /// 返回Result,成功时返回(),失败时返回错误信息
    pub fn validate(&self) -> Result<(), String> {
        // 验证ID格式
        if self.id.is_empty() {
            return Err("依赖项ID不能为空".to_string());
        }

        // 验证ID只包含小写字母、数字和连字符
        if !self.id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
            return Err("依赖项ID只能包含小写字母、数字和连字符".to_string());
        }

        // 验证名称不为空
        if self.name.is_empty() {
            return Err("依赖项名称不能为空".to_string());
        }

        // 验证版本要求不为空
        if self.version_requirement.is_empty() {
            return Err("版本要求不能为空".to_string());
        }

        // 验证安装优先级范围
        if !(1..=10).contains(&self.install_priority) {
            return Err("安装优先级必须在1-10范围内".to_string());
        }

        // 验证自动安装相关字段
        if self.auto_installable {
            if self.install_command.is_none() || self.install_command.as_ref().unwrap().is_empty() {
                return Err("自动安装依赖必须提供install_command".to_string());
            }
        } else if self.install_guide.is_empty() {
            return Err("手动安装依赖必须提供install_guide".to_string());
        }

        Ok(())
    }

    /// 检查是否为必需依赖
    pub fn is_required(&self) -> bool {
        matches!(self.level, DependencyLevel::Required)
    }

    /// 检查是否为可选依赖
    pub fn is_optional(&self) -> bool {
        matches!(self.level, DependencyLevel::Optional)
    }

    /// 获取检测方法类型名称
    pub fn check_method_name(&self) -> &'static str {
        match self.check_method {
            CheckMethod::Executable { .. } => "executable",
            CheckMethod::Service { .. } => "service",
            CheckMethod::File { .. } => "file",
        }
    }
}

/// 依赖检测状态
///
/// 覆盖所有可能的检测结果,支持精确错误提示
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckStatus {
    /// 满足要求(版本符合 or 存在性检测通过)
    Satisfied,
    /// 缺失(未找到可执行文件/文件/服务)
    Missing,
    /// 版本不匹配(检测到但版本低于要求)
    VersionMismatch,
    /// 损坏(找到但无法执行/无法获取版本)
    Corrupted,
}

impl CheckStatus {
    /// 检查是否为成功状态
    pub fn is_success(&self) -> bool {
        matches!(self, CheckStatus::Satisfied)
    }

    /// 检查是否为失败状态
    pub fn is_failure(&self) -> bool {
        matches!(self, CheckStatus::Missing | CheckStatus::VersionMismatch | CheckStatus::Corrupted)
    }

    /// 获取状态描述
    pub fn description(&self) -> &'static str {
        match self {
            CheckStatus::Satisfied => "已满足",
            CheckStatus::Missing => "缺失",
            CheckStatus::VersionMismatch => "版本不匹配",
            CheckStatus::Corrupted => "损坏",
        }
    }
}

/// 依赖检测结果
///
/// 记录单次依赖检测的状态快照,支持检测历史追溯
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyCheckResult {
    /// 关联的依赖项ID
    pub dependency_id: String,

    /// 检测开始时间(UTC)
    pub checked_at: DateTime<Utc>,

    /// 检测状态
    pub status: CheckStatus,

    /// 检测到的版本号(如果成功检测到)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detected_version: Option<String>,

    /// 错误详情(status为失败状态时提供)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_details: Option<String>,

    /// 检测耗时(毫秒)
    pub duration_ms: u64,
}

impl DependencyCheckResult {
    /// 创建新的检测结果
    pub fn new(
        dependency_id: String,
        status: CheckStatus,
        detected_version: Option<String>,
        error_details: Option<String>,
        duration_ms: u64,
    ) -> Self {
        Self {
            dependency_id,
            checked_at: Utc::now(),
            status,
            detected_version,
            error_details,
            duration_ms,
        }
    }

    /// 创建成功结果
    pub fn success(dependency_id: String, detected_version: Option<String>, duration_ms: u64) -> Self {
        Self::new(dependency_id, CheckStatus::Satisfied, detected_version, None, duration_ms)
    }

    /// 创建失败结果
    pub fn failure(
        dependency_id: String,
        status: CheckStatus,
        error_details: String,
        duration_ms: u64,
    ) -> Self {
        Self::new(dependency_id, status, None, Some(error_details), duration_ms)
    }

    /// 检查是否为成功检测
    pub fn is_satisfied(&self) -> bool {
        self.status.is_success()
    }

    /// 检查是否为失败检测
    pub fn is_failed(&self) -> bool {
        self.status.is_failure()
    }
}

/// 安装任务状态
///
/// 支持安装任务的完整生命周期追踪
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallStatus {
    /// 待处理(任务已创建但未开始)
    Pending,
    /// 下载中(正在下载依赖包)
    Downloading,
    /// 安装中(正在执行安装命令)
    Installing,
    /// 成功(安装完成且验证通过)
    Success,
    /// 失败(安装过程出错)
    Failed,
}

impl InstallStatus {
    /// 检查任务是否已完成
    pub fn is_completed(&self) -> bool {
        matches!(self, InstallStatus::Success | InstallStatus::Failed)
    }

    /// 检查任务是否正在运行
    pub fn is_running(&self) -> bool {
        matches!(self, InstallStatus::Downloading | InstallStatus::Installing)
    }

    /// 检查任务是否可以开始
    pub fn can_start(&self) -> bool {
        matches!(self, InstallStatus::Pending)
    }
}

/// 安装任务
///
/// 代表一次依赖自动安装操作的生命周期状态,支持进度追踪和日志记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallationTask {
    /// 任务唯一ID(UUID v4)
    pub task_id: Uuid,

    /// 关联的依赖项ID
    pub dependency_id: String,

    /// 任务创建时间(UTC)
    pub created_at: DateTime<Utc>,

    /// 开始安装时间(状态转为InProgress时设置)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<DateTime<Utc>>,

    /// 完成时间(成功或失败时设置)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,

    /// 安装状态
    pub status: InstallStatus,

    /// 安装进度百分比(0-100)
    pub progress_percent: u8,

    /// 错误消息(status=Failed时提供)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,

    /// 安装日志条目(按时间顺序)
    pub install_log: Vec<String>,

    /// 错误类型(失败时分类)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_type: Option<InstallErrorType>,
}

impl InstallationTask {
    /// 创建新的安装任务
    pub fn new(dependency_id: String) -> Self {
        let now = Utc::now();
        Self {
            task_id: Uuid::new_v4(),
            dependency_id: dependency_id.clone(),
            created_at: now,
            started_at: None,
            completed_at: None,
            status: InstallStatus::Pending,
            progress_percent: 0,
            error_message: None,
            install_log: vec![format!("Task created for {}", dependency_id)],
            error_type: None,
        }
    }

    /// 开始安装
    pub fn start(&mut self) {
        if self.status.can_start() {
            self.started_at = Some(Utc::now());
            self.status = InstallStatus::Downloading;
            self.progress_percent = 10;
            self.install_log.push("开始下载...".to_string());
        }
    }

    /// 更新进度
    pub fn update_progress(&mut self, status: InstallStatus, progress: u8, log_entry: String) {
        self.status = status;
        self.progress_percent = progress;
        self.install_log.push(log_entry);
    }

    /// 标记为成功
    pub fn mark_success(&mut self) {
        self.completed_at = Some(Utc::now());
        self.status = InstallStatus::Success;
        self.progress_percent = 100;
        self.install_log.push("安装完成".to_string());
    }

    /// 标记为失败
    pub fn mark_failed(&mut self, error_type: InstallErrorType, error_message: String) {
        self.completed_at = Some(Utc::now());
        self.status = InstallStatus::Failed;
        self.error_type = Some(error_type);
        self.install_log.push(format!("安装失败: {}", error_message));
        self.error_message = Some(error_message);
    }

    /// 验证进度值是否在有效范围内 (0-100)
    pub fn validate_progress(progress: u8) -> Result<u8, String> {
        if progress <= 100 {
            Ok(progress)
        } else {
            Err(format!("进度值 {} 超出有效范围 0-100", progress))
        }
    }

    /// 安全地更新进度，自动验证范围
    pub fn update_progress_safe(&mut self, status: InstallStatus, progress: u8, log_entry: String) -> Result<(), String> {
        let validated_progress = Self::validate_progress(progress)?;
        self.status = status;
        self.progress_percent = validated_progress;
        self.install_log.push(log_entry);
        Ok(())
    }

    /// 检查任务是否已完成（成功或失败）
    pub fn is_completed(&self) -> bool {
        self.status.is_completed()
    }

    /// 检查任务是否正在运行中
    pub fn is_running(&self) -> bool {
        self.status.is_running()
    }

    /// 获取任务运行时长（毫秒）
    pub fn get_duration_ms(&self) -> Option<u64> {
        let start = self.started_at?;
        let end = self.completed_at.unwrap_or_else(Utc::now);
        Some(end.signed_duration_since(start).num_milliseconds() as u64)
    }

    /// 添加日志条目
    pub fn add_log(&mut self, message: String) {
        self.install_log.push(message);
    }

    /// 获取最后一个日志条目
    pub fn last_log(&self) -> Option<&String> {
        self.install_log.last()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dependency_creation() {
        let dependency = Dependency::new(
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
            "## 安装Node.js\n\n1. 访问官网下载".to_string(),
            None,
        );

        assert_eq!(dependency.id, "nodejs");
        assert_eq!(dependency.name, "Node.js");
        assert!(dependency.is_required());
        assert!(!dependency.is_optional());
    }

    #[test]
    fn test_dependency_validation() {
        let valid_dependency = Dependency::new(
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
            "## 安装Node.js\n\n1. 访问官网下载".to_string(),
            None,
        );

        assert!(valid_dependency.validate().is_ok());

        // 测试无效ID
        let mut invalid_dependency = valid_dependency.clone();
        invalid_dependency.id = "Invalid ID".to_string();
        assert!(invalid_dependency.validate().is_err());

        // 测试空名称
        invalid_dependency.id = "nodejs".to_string();
        invalid_dependency.name = "".to_string();
        assert!(invalid_dependency.validate().is_err());

        // 测试无效优先级
        invalid_dependency.name = "Node.js".to_string();
        invalid_dependency.install_priority = 0;
        assert!(invalid_dependency.validate().is_err());
    }

    #[test]
    fn test_check_status() {
        assert!(CheckStatus::Satisfied.is_success());
        assert!(!CheckStatus::Satisfied.is_failure());

        assert!(CheckStatus::Missing.is_failure());
        assert!(!CheckStatus::Missing.is_success());

        assert_eq!(CheckStatus::Satisfied.description(), "已满足");
        assert_eq!(CheckStatus::Missing.description(), "缺失");
    }

    #[test]
    fn test_dependency_check_result() {
        let result = DependencyCheckResult::success(
            "nodejs".to_string(),
            Some("20.10.0".to_string()),
            45,
        );

        assert!(result.is_satisfied());
        assert!(!result.is_failed());
        assert_eq!(result.detected_version, Some("20.10.0".to_string()));
    }

    #[test]
    fn test_installation_task() {
        let mut task = InstallationTask::new("nodejs".to_string());

        assert!(!task.is_completed());
        assert!(!task.is_running());
        assert!(task.status.can_start());

        task.start();
        assert!(task.is_running());
        assert_eq!(task.progress_percent, 10);

        task.mark_success();
        assert!(task.is_completed());
        assert_eq!(task.progress_percent, 100);
    }

    #[test]
    fn test_progress_validation() {
        assert!(InstallationTask::validate_progress(0).is_ok());
        assert!(InstallationTask::validate_progress(100).is_ok());
        assert!(InstallationTask::validate_progress(50).is_ok());
        assert!(InstallationTask::validate_progress(101).is_err());
    }
}