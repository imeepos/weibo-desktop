# Data Model: 启动时依赖检测与自动安装

**生成日期**: 2025-10-05
**版本**: 1.0.0
**数据源**: [spec.md](./spec.md) Key Entities + [research.md](./research.md) 技术选型

---

## 概述

本文档定义启动时依赖检测功能的核心数据结构,遵循Tauri Desktop架构的Rust后端数据模型设计。所有结构体均支持序列化/反序列化(通过serde)以便与前端React组件通信。

---

## 核心实体

### 1. Dependency - 依赖项定义

代表应用运行所需的外部组件或资源的元数据配置。

```rust
use serde::{Deserialize, Serialize};

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

/// 依赖重要性级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DependencyLevel {
    /// 必需依赖:缺失时阻止应用启动
    Required,
    /// 可选依赖:缺失时显示警告但允许启动
    Optional,
}

/// 依赖检测方法类型
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
```

**字段设计理由**:
- `id`作为唯一标识,用于关联检测结果和安装任务
- `version_requirement`使用semver格式,与research.md中选定的`semver` crate对齐
- `level`枚举明确必需/可选语义,避免布尔值歧义
- `check_method`枚举支持3种检测方式,覆盖Node.js(可执行文件)/Redis(服务端口)/Playwright(文件)等场景
- `install_guide`支持Markdown格式,便于前端渲染富文本

---

### 2. DependencyCheckResult - 依赖检测结果

记录单次依赖检测的状态快照,支持检测历史追溯。

```rust
use chrono::{DateTime, Utc};

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

/// 依赖检测状态
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
```

**字段设计理由**:
- `checked_at`使用UTC时间戳,便于跨时区日志分析
- `detected_version`和`error_details`为Optional,遵循Rust最佳实践
- `duration_ms`用于性能监控,识别慢检测项
- `CheckStatus`枚举覆盖所有可能的检测结果,支持精确错误提示

---

### 3. InstallationTask - 安装任务

代表一次依赖自动安装操作的生命周期状态,支持进度追踪和日志记录。

```rust
use uuid::Uuid;

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

/// 安装任务状态
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

/// 安装错误类型(用于错误提示分类)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallErrorType {
    /// 网络错误(无法连接/下载超时)
    NetworkError,
    /// 权限错误(需要管理员权限)
    PermissionError,
    /// 磁盘空间不足
    DiskSpaceError,
    /// 版本冲突(已存在其他版本且无法覆盖)
    VersionConflictError,
    /// 未知错误
    UnknownError,
}
```

**字段设计理由**:
- `task_id`使用UUID保证全局唯一性,支持并发安装任务追踪
- 三个时间戳(`created_at`/`started_at`/`completed_at`)支持精确计时和性能分析
- `progress_percent`为u8限定范围0-100,避免无效值
- `install_log`为Vec动态增长,记录安装命令输出
- `error_type`枚举支持前端根据错误类型展示定制化提示

---

### 4. InstallationGuide - 安装指引

为需手动安装的依赖提供多语言安装指导信息(当前版本仅支持中文)。

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallationGuide {
    /// 关联的依赖项ID
    pub dependency_id: String,

    /// 依赖名称(用于展示)
    pub dependency_name: String,

    /// 指引标题
    pub title: String,

    /// 指引内容(Markdown格式步骤列表)
    pub content: String,

    /// 相关链接(官方下载页/文档)
    pub links: Vec<InstallationLink>,

    /// 适用的操作系统(空表示全平台通用)
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub target_os: Vec<String>,

    /// 语言版本(当前固定为"zh-CN")
    pub language: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallationLink {
    /// 链接文本
    pub text: String,
    /// 链接URL
    pub url: String,
}
```

**字段设计理由**:
- `content`为Markdown格式,与`Dependency.install_guide`一致
- `target_os`支持多平台指引(如Windows/macOS/Linux不同安装方式)
- `language`字段预留国际化扩展能力,当前固定"zh-CN"满足FR-014要求

---

## 状态转换图

### DependencyCheckResult状态流

```
[NotStarted]
    ↓ (开始检测)
[Checking]
    ↓ (检测完成)
    ├─→ [Satisfied]         # 找到且版本符合
    ├─→ [Missing]           # 未找到可执行文件/文件/服务
    ├─→ [VersionMismatch]   # 找到但版本低于要求
    └─→ [Corrupted]         # 找到但无法执行
```

### InstallationTask状态流

```
[Pending]
    ↓ (开始安装)
[Downloading]
    ↓ (下载完成)
[Installing]
    ↓ (安装命令执行)
    ├─→ [Success]   # 安装成功且重新检测通过
    └─→ [Failed]    # 网络/权限/磁盘/版本冲突错误
        ├─ NetworkError
        ├─ PermissionError
        ├─ DiskSpaceError
        ├─ VersionConflictError
        └─ UnknownError
```

---

## 数据关系

```
Dependency (1) ──< (N) DependencyCheckResult
    ↓ id              └─ dependency_id

Dependency (1) ──< (N) InstallationTask
    ↓ id              └─ dependency_id

Dependency (1) ─── (1) InstallationGuide
    ↓ id              └─ dependency_id
```

**说明**:
- 一个Dependency可以有多次检测结果(支持启动检测和用户手动触发检测)
- 一个Dependency可以有多次安装任务(重试场景)
- 一个Dependency对应一个安装指引(手动安装场景)

---

## 验证规则

### Dependency验证

| 字段 | 规则 |
|------|------|
| `id` | 非空,仅包含小写字母/数字/连字符 |
| `version_requirement` | 符合semver格式(由`semver` crate验证) |
| `install_priority` | 1-10范围 |
| `install_command` | 当`auto_installable=true`时必须非空 |
| `install_guide` | 当`auto_installable=false`时必须非空 |

### InstallationTask验证

| 字段 | 规则 |
|------|------|
| `progress_percent` | 0-100范围 |
| `error_message` | 当`status=Failed`时必须非空 |
| `error_type` | 当`status=Failed`时必须非空 |
| `completed_at` | 当`status=Success/Failed`时必须非空 |

---

## 序列化示例

### Dependency JSON示例

```json
{
  "id": "nodejs",
  "name": "Node.js",
  "version_requirement": ">=20.0.0",
  "description": "JavaScript运行时,用于执行前端构建和Playwright自动化脚本",
  "level": "required",
  "auto_installable": false,
  "install_priority": 1,
  "check_method": {
    "type": "executable",
    "name": "node",
    "version_args": ["--version"]
  },
  "install_guide": "## 安装Node.js\n\n1. 访问 [Node.js官网](https://nodejs.org/zh-cn/download/)\n2. 下载LTS版本(20.x)\n3. 运行安装程序并按提示完成安装\n4. 重启终端验证: `node --version`"
}
```

### DependencyCheckResult JSON示例

```json
{
  "dependency_id": "nodejs",
  "checked_at": "2025-10-05T10:30:15.123Z",
  "status": "satisfied",
  "detected_version": "20.10.0",
  "duration_ms": 45
}
```

### InstallationTask JSON示例

```json
{
  "task_id": "550e8400-e29b-41d4-a716-446655440000",
  "dependency_id": "pnpm",
  "created_at": "2025-10-05T10:31:00.000Z",
  "started_at": "2025-10-05T10:31:01.000Z",
  "completed_at": null,
  "status": "installing",
  "progress_percent": 65,
  "error_message": null,
  "install_log": [
    "Downloading pnpm@8.10.0...",
    "Extracting package...",
    "Installing to global directory..."
  ],
  "error_type": null
}
```

---

## 存储策略

### 依赖配置(Dependency)

**存储方式**: 静态配置,嵌入Rust代码或读取配置文件

**位置**: `src-tauri/src/config/dependencies.rs` 或 `src-tauri/dependencies.toml`

**理由**: 依赖清单相对固定,无需运行时动态修改,嵌入式配置简化部署

**示例(TOML格式)**:
```toml
[[dependencies]]
id = "nodejs"
name = "Node.js"
version_requirement = ">=20.0.0"
description = "JavaScript运行时"
level = "required"
auto_installable = false
install_priority = 1

[dependencies.check_method]
type = "executable"
name = "node"
version_args = ["--version"]
```

### 检测结果(DependencyCheckResult)

**存储方式**: 可选Redis缓存(与001-cookies保持一致) + 日志文件

**Redis Key设计**: `dep:check:{dependency_id}` → JSON序列化的CheckResult

**TTL**: 24小时(避免过期检测结果误导)

**理由**: 缓存检测结果避免重复检测,提升启动速度

### 安装任务(InstallationTask)

**存储方式**: 内存中(任务生命周期短) + 日志文件永久保留

**理由**: 安装任务为临时状态,无需持久化;日志文件提供历史追溯

### 日志持久化

**位置**: 应用数据目录 `/logs/dependency_check_YYYY-MM-DD.log`

**格式**: JSON Lines (每行一条日志)

**内容**: 所有检测结果和安装任务的完整状态变更

---

## 与Constitution对齐检查

### ✅ 存在即合理
- 每个字段都有明确用途,无冗余属性
- 枚举类型精确建模业务语义,避免魔法值

### ✅ 优雅即简约
- 结构体命名自文档化: `DependencyCheckResult`, `InstallationTask`
- 使用Rust类型系统(Option, enum)表达业务约束

### ✅ 性能即艺术
- 使用`#[serde(skip_serializing_if)]`减少JSON payload大小
- DateTime<Utc>直接支持高效序列化

### ✅ 错误处理如为人处世的哲学
- `InstallErrorType`枚举支持精确错误分类
- `error_details`字段提供人类可读的诊断信息

### ✅ 日志是思想的表达
- `install_log`记录安装命令的完整输出
- `duration_ms`支持性能分析

---

**数据模型版本**: 1.0.0
**最后更新**: 2025-10-05
**审查状态**: ✅ 通过Constitution Check
