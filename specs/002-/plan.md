
# Implementation Plan: 启动时依赖检测与自动安装

**Branch**: `002-` | **Date**: 2025-10-05 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/workspace/desktop/specs/002-/spec.md`

## Execution Flow (/plan command scope)
```
1. Load feature spec from Input path
   → ✅ Loaded successfully
2. Fill Technical Context (scan for NEEDS CLARIFICATION)
   → ✅ No NEEDS CLARIFICATION markers found (all resolved in clarification session)
   → Detected Project Type: Desktop application (Tauri: frontend + backend)
   → Set Structure Decision: Tauri架构 (src-tauri/ + src/)
3. Fill the Constitution Check section based on the constitution document
   → ✅ Constitution principles applied
4. Evaluate Constitution Check section below
   → ✅ No violations - design aligns with simplicity and necessity principles
   → Update Progress Tracking: Initial Constitution Check PASS
5. Execute Phase 0 → research.md
   → In progress
6. Execute Phase 1 → contracts, data-model.md, quickstart.md, CLAUDE.md
   → Pending
7. Re-evaluate Constitution Check section
   → Pending
8. Plan Phase 2 → Describe task generation approach (DO NOT create tasks.md)
   → Pending
9. STOP - Ready for /tasks command
```

**IMPORTANT**: The /plan command STOPS at step 8. Phases 2-4 are executed by other commands:
- Phase 2: /tasks command creates tasks.md
- Phase 3-4: Implementation execution (manual or via tools)

## Summary

本功能为Desktop应用添加启动时依赖检测与自动安装能力。系统在应用启动时自动检测所有必需和可选依赖项的完整性和版本兼容性,对可自动安装的依赖通过在线方式下载安装,对需手动安装的依赖提供详细的中文安装指引。检测和安装过程通过进度条向用户展示,无超时限制,支持必需依赖串行安装和可选依赖并行安装的混合策略。所有日志永久保留用于诊断。

**技术方法**:
- 利用现有Tauri架构,后端Rust实现依赖检测逻辑和安装任务管理
- 前端React组件展示进度条和安装引导界面
- 使用Tauri Command模式暴露依赖管理接口
- 依赖元数据和检测结果可选使用Redis存储(与现有存储策略保持一致)

**约束条件**:
- 与当前项目技术栈吻合,不新增或更换核心技术
- 复用Tauri/React/Redis/Playwright基础设施
- 遵循现有项目结构和命名规范

## Technical Context

**Language/Version**:
- 后端: Rust 1.75+
- 前端: TypeScript 5.0+, React 18+

**Primary Dependencies**:
- 后端: Tauri 1.x, Tokio (异步运行时), serde (序列化), tracing (日志)
- 前端: React 18+, TailwindCSS 3+ (UI组件)
- 可选: redis-rs (依赖检测结果缓存)

**Storage**:
- 依赖配置: 嵌入式配置文件或静态定义
- 检测日志: 文件系统持久化 (永久保留)
- 可选: Redis Hash存储检测结果缓存

**Testing**:
- 后端: `cargo test` (单元测试和集成测试)
- 契约测试: Rust contract tests in `src-tauri/tests/`
- 前端: React Testing Library

**Target Platform**:
- Windows/Linux/macOS Desktop (Tauri支持的平台)

**Project Type**: Desktop application (Tauri: frontend + backend)

**Performance Goals**:
- 检测启动延迟 < 2秒(理想状态,无超时限制)
- 进度条更新频率 >= 10 Hz (流畅用户体验)
- 并行安装可选依赖时内存开销 < 100MB

**Constraints**:
- 无超时限制,等待所有检测完成
- 仅支持在线安装,不考虑离线策略
- 必需依赖串行安装,可选依赖并行安装
- 日志永久保留,不自动清理
- 安装指引至少提供中文说明

**Scale/Scope**:
- 初始依赖项数量: 5-10个(估算)
- 支持扩展至20+依赖项
- 日志文件单次检测预计 < 10KB

**User Constraints**:
- 与当前项目技术栈吻合
- 如无必要,不新增或更换技术组件

## Constitution Check
*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### 一、存在即合理 (Existence Implies Necessity)
**状态**: ✅ PASS

- 每个依赖检测器、安装器、UI组件都服务于不可替代的功能需求
- 进度条组件必需(FR-004: 用户必须看到检测进度)
- 依赖分级(必需/可选)必需(FR-002: 系统必须区分依赖级别)
- 日志记录必需(FR-007: 永久保留诊断信息)
- 不引入冗余抽象层,直接在Tauri Command中实现核心逻辑

### 二、优雅即简约 (Elegance is Simplicity)
**状态**: ✅ PASS

- 命名遵循领域语言: `DependencyChecker`, `InstallationTask`, `ProgressIndicator`
- 状态机设计清晰: `Pending → Checking → Installing → Completed/Failed`
- 避免过度工程化: 不引入复杂的依赖注入框架或消息总线
- 代码自文档化: 结构体字段和方法名揭示意图

### 三、性能即艺术 (Performance is Art)
**状态**: ✅ PASS

- 并行策略优雅: 可选依赖使用Tokio并发,必需依赖串行保证顺序
- 避免阻塞主线程: 检测和安装在后台任务中执行
- 进度更新通过事件流式传递,避免轮询
- 日志异步写入,不阻塞检测流程

### 四、错误处理如为人处世的哲学 (Error Handling as Life Philosophy)
**状态**: ✅ PASS

- 网络错误: 记录详细日志,提示用户检查网络,提供重试选项
- 权限错误: 提示用户以管理员身份重启,提供重启引导
- 版本冲突: 记录冲突详情,提供手动解决指引
- 所有错误类型化(Result<T, DependencyError>),统一处理

### 五、日志是思想的表达 (Logs Express Thought)
**状态**: ✅ PASS

- 使用tracing框架结构化日志
- 日志级别明确: ERROR(权限/网络失败), WARN(可选依赖缺失), INFO(检测开始/完成), DEBUG(详细步骤)
- 每条日志包含上下文: 依赖名称、版本、时间戳、错误码
- 永久保留,支持离线诊断

**结论**: 设计符合所有章程原则,无需复杂性豁免。

## Project Structure

### Documentation (this feature)
```
specs/002-/
├── plan.md              # This file (/plan command output)
├── research.md          # Phase 0 output (/plan command)
├── data-model.md        # Phase 1 output (/plan command)
├── quickstart.md        # Phase 1 output (/plan command)
├── contracts/           # Phase 1 output (/plan command)
│   ├── check_dependencies.md
│   ├── install_dependency.md
│   └── query_dependency_status.md
└── tasks.md             # Phase 2 output (/tasks command - NOT created by /plan)
```

### Source Code (repository root)

基于Tauri Desktop架构 (frontend + backend):

```
src-tauri/
├── src/
│   ├── commands/
│   │   └── dependency_commands.rs    # check_dependencies, install_dependency等Tauri Commands
│   ├── models/
│   │   ├── dependency.rs              # Dependency, DependencyCheckResult, InstallationTask
│   │   └── errors.rs                  # DependencyError枚举
│   ├── services/
│   │   ├── dependency_checker.rs      # 依赖检测核心逻辑
│   │   ├── installer_service.rs       # 在线安装服务
│   │   └── logger_service.rs          # 日志持久化
│   └── utils/
│       └── version.rs                 # 版本比较工具
└── tests/
    ├── contract_check_dependencies.rs
    ├── contract_install_dependency.rs
    └── integration_test.rs

src/
├── components/
│   ├── DependencyProgress.tsx         # 进度条组件
│   ├── InstallationGuide.tsx          # 安装指引界面
│   └── DependencyStatus.tsx           # 依赖状态列表
├── pages/
│   └── StartupCheckPage.tsx           # 启动检测页面
└── hooks/
    └── useDependencyCheck.ts          # 依赖检测React Hook
```

**Structure Decision**:
选择Tauri Desktop架构(Option 2变体)。后端Rust负责依赖检测、安装逻辑和日志记录,前端React负责进度条展示和用户交互。通过Tauri Command桥接前后端,利用Tauri事件系统实时推送进度更新。结构与现有001-cookies功能保持一致,复用commands/models/services分层模式。

## Phase 0: Outline & Research

### 研究任务清单

由于Technical Context中无NEEDS CLARIFICATION标记(已在clarification session解决),Phase 0聚焦于技术选型验证和最佳实践研究:

1. **Tauri启动生命周期集成**
   - 研究任务: Tauri应用启动时如何插入依赖检测流程
   - 目标: 确定在`main.rs`或`setup hook`中触发检测的最佳时机
   - 输出: 启动流程集成方案

2. **Rust依赖检测模式**
   - 研究任务: Rust生态中检测外部依赖(如Node.js/Playwright/Redis)的常用方法
   - 调研库: `which` crate(检测可执行文件), `semver` crate(版本比较)
   - 输出: 依赖检测技术选型

3. **Tokio并发安装模式**
   - 研究任务: 使用Tokio实现必需依赖串行+可选依赖并行的混合策略
   - 关键点: `tokio::spawn`, `JoinSet`, 错误传播
   - 输出: 并发安装架构设计

4. **Tauri事件流进度更新**
   - 研究任务: 从Rust后台任务向React前端实时推送进度事件
   - 关键API: `app.emit_all()`, `tauri::Window::emit()`
   - 输出: 进度通信协议

5. **日志持久化策略**
   - 研究任务: tracing框架与文件日志Appender集成
   - 调研库: `tracing-appender`, `tracing-subscriber`
   - 输出: 日志存储方案(文件路径、格式、轮转策略)

6. **现有依赖项识别**
   - 研究任务: 分析项目当前依赖Playwright/Node.js/Redis的版本要求
   - 方法: 读取`package.json`, `Cargo.toml`, Docker配置
   - 输出: 初始依赖清单(5-10项)

### 研究输出文件

**Output**: `research.md` 包含以下章节:
- **启动集成方案**: Tauri setup hook调用时机
- **检测技术选型**: which + semver crates
- **并发模型**: Tokio JoinSet + sequential execution for critical deps
- **进度通信**: Tauri Event payload schema
- **日志方案**: tracing-appender文件输出,JSON格式
- **初始依赖清单**: Node.js 20+, pnpm, Playwright, Redis 7+, Rust 1.75+

## Phase 1: Design & Contracts
*Prerequisites: research.md complete*

### 1. 数据模型设计 (`data-model.md`)

基于规格说明中的Key Entities提取:

**核心实体**:

```rust
// Dependency: 依赖项定义
struct Dependency {
    name: String,
    version_requirement: String,  // semver格式
    description: String,
    level: DependencyLevel,       // Required | Optional
    auto_installable: bool,
    install_priority: u8,         // 1-10,数字越小优先级越高
    check_method: CheckMethod,    // Executable | Service | File
    install_guide: String,        // Markdown格式安装说明
}

// DependencyCheckResult: 检测结果
struct DependencyCheckResult {
    dependency_id: String,
    checked_at: DateTime<Utc>,
    status: CheckStatus,          // Satisfied | Missing | VersionMismatch | Corrupted
    detected_version: Option<String>,
    error_details: Option<String>,
}

// InstallationTask: 安装任务
struct InstallationTask {
    task_id: Uuid,
    dependency_id: String,
    started_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
    status: InstallStatus,        // Pending | InProgress | Success | Failed
    progress_percent: u8,
    error_message: Option<String>,
    install_log: Vec<String>,
}
```

**状态转换**:
```
Dependency Check: NotStarted → Checking → Completed(Satisfied/Missing/VersionMismatch)
Installation: Pending → InProgress → Success/Failed
```

### 2. API契约生成 (`contracts/`)

基于Functional Requirements设计Tauri Commands:

**Contract 1: check_dependencies**
- 输入: 无(检测所有依赖)
- 输出: `Vec<DependencyCheckResult>`
- 事件流: `dependency-check-progress` (每个依赖检测完成时emit)
- 错误: `DependencyError::CheckFailed`

**Contract 2: install_dependency**
- 输入: `dependency_id: String, force: bool`
- 输出: `InstallationTask`
- 事件流: `installation-progress` (进度百分比更新)
- 错误: `DependencyError::InstallFailed` (网络/权限/版本冲突)

**Contract 3: query_dependency_status**
- 输入: `dependency_id: Option<String>` (None表示查询全部)
- 输出: `Vec<DependencyCheckResult>`
- 错误: 无

**Contract 4: trigger_manual_check**
- 输入: 无
- 输出: `Vec<DependencyCheckResult>`
- 场景: 用户运行期间手动触发检测(FR-009)

### 3. 契约测试生成

位置: `src-tauri/tests/`

- `contract_check_dependencies.rs`: 测试返回正确的检测结果结构
- `contract_install_dependency.rs`: 测试安装任务创建和进度事件
- `contract_query_status.rs`: 测试状态查询接口

### 4. 集成测试场景 (`quickstart.md`)

基于Acceptance Scenarios:

**测试1**: 所有依赖满足场景
- 步骤: 模拟所有依赖已安装 → 启动应用 → 验证进度条快速完成 → 进入主界面
- 预期: 无安装动作,仅检测日志

**测试2**: 缺失可自动安装依赖
- 步骤: 移除一个可选依赖 → 启动 → 观察自动安装进度 → 验证安装成功
- 预期: 进度条显示下载和安装,最终状态为Satisfied

**测试3**: 权限不足场景
- 步骤: 模拟权限错误 → 启动安装 → 验证错误提示
- 预期: 显示"请以管理员身份打开应用"

### 5. 更新CLAUDE.md

通过脚本 `.specify/scripts/bash/update-agent-context.sh claude` 增量更新:
- 新增功能: 002-依赖检测与自动安装
- 新增Commands: check_dependencies, install_dependency, query_dependency_status
- 新增模型: Dependency, DependencyCheckResult, InstallationTask
- 保留001-cookies信息,标记最近变更

**Output**: data-model.md, contracts/*.md, contract tests, quickstart.md, 更新后的CLAUDE.md

## Phase 2: Task Planning Approach
*This section describes what the /tasks command will do - DO NOT execute during /plan*

**Task Generation Strategy**:
- 加载 `.specify/templates/tasks-template.md` 作为基础
- 从Phase 1产物生成任务:
  - 每个契约 → 1个契约测试任务 [P]
  - 每个实体 → 1个模型实现任务 [P]
  - 每个Tauri Command → 1个命令实现任务
  - 每个React组件 → 1个UI组件任务 [P]
  - 每个集成场景 → 1个集成测试任务

**Ordering Strategy**:
- TDD顺序: 测试先于实现
- 依赖顺序:
  1. 模型定义 (Dependency, CheckResult, Task)
  2. 契约测试 (失败状态)
  3. 服务层实现 (DependencyChecker, InstallerService)
  4. Tauri Commands实现
  5. 前端组件 (进度条, 安装指引)
  6. 集成测试
  7. Quickstart验证
- 并行标记 [P]:
  - 独立模型文件可并行
  - 独立契约测试可并行
  - 前端组件可并行

**Estimated Output**: 20-25个任务
- 模型任务: 3个 (Dependency, CheckResult, InstallationTask)
- 契约测试: 4个
- 服务实现: 3个 (Checker, Installer, Logger)
- Command实现: 4个
- UI组件: 3个 (Progress, Guide, Status)
- 集成测试: 3个
- 文档和配置: 2个

**IMPORTANT**: This phase is executed by the /tasks command, NOT by /plan

## Phase 3+: Future Implementation
*These phases are beyond the scope of the /plan command*

**Phase 3**: Task execution (/tasks command creates tasks.md)
**Phase 4**: Implementation (execute tasks.md following constitutional principles)
**Phase 5**: Validation (run tests, execute quickstart.md, performance validation)

## Complexity Tracking
*Fill ONLY if Constitution Check has violations that must be justified*

无违规项,表格留空。

## Progress Tracking
*This checklist is updated during execution flow*

**Phase Status**:
- [x] Phase 0: Research complete (/plan command) - ✅ research.md generated
- [x] Phase 1: Design complete (/plan command) - ✅ data-model.md, contracts/, quickstart.md, CLAUDE.md updated
- [x] Phase 2: Task planning approach described (/plan command) - ✅ Described in Phase 2 section
- [x] Phase 3: Tasks generated (/tasks command) - ✅ tasks.md with 34 ordered tasks
- [ ] Phase 4: Implementation complete - Execute tasks.md
- [ ] Phase 5: Validation passed - Run quickstart tests

**Gate Status**:
- [x] Initial Constitution Check: PASS
- [x] Post-Design Constitution Check: PASS - No violations detected in Phase 1 artifacts
- [x] All NEEDS CLARIFICATION resolved (completed in /clarify session)
- [x] Complexity deviations documented (无违规)

---
*Based on Constitution v1.0.0 - See `.specify/memory/constitution.md`*
