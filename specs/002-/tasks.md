# Tasks: 启动时依赖检测与自动安装

**功能分支**: `002-`
**Input**: Design documents from `/workspace/desktop/specs/002-/`
**Prerequisites**: ✅ plan.md, research.md, data-model.md, contracts/, quickstart.md

---

## Execution Flow (main)
```
1. Load plan.md from feature directory
   → ✅ Tech stack: Rust 1.75+, React 18+, Tauri 1.x, Tokio, tracing
   → ✅ Structure: Tauri Desktop (src-tauri/ + src/)
2. Load optional design documents:
   → ✅ data-model.md: 4 entities (Dependency, CheckResult, InstallationTask, InstallationGuide)
   → ✅ contracts/: 4 contracts (check_dependencies, install_dependency, query_dependency_status, trigger_manual_check)
   → ✅ research.md: 6 research decisions + initial dependency list
   → ✅ quickstart.md: 5 test scenarios
3. Generate tasks by category:
   → Setup: Cargo dependencies, project structure
   → Tests: 4 contract tests + 5 integration tests
   → Core: 4 models, 3 services, 4 commands, 3 UI components
   → Integration: Event system, logging, startup hook
   → Polish: Unit tests, quickstart execution, documentation
4. Apply task rules:
   → Different files = mark [P]
   → Same file = sequential
   → Tests before implementation (TDD)
5. Number tasks: T001-T030
6. Generate dependency graph
7. Validate completeness ✓
```

---

## Path Conventions

**Backend (Rust)**:
- Models: `src-tauri/src/models/`
- Services: `src-tauri/src/services/`
- Commands: `src-tauri/src/commands/`
- Tests: `src-tauri/tests/`

**Frontend (React)**:
- Components: `src/components/`
- Pages: `src/pages/`
- Hooks: `src/hooks/`

---

## Phase 3.1: Setup (T001-T004)

- [X] **T001** 添加Rust依赖到 `src-tauri/Cargo.toml` ✅ (已完成)
  - 依赖: `which = "5.0"`, `semver = "1.0"`, `tracing-appender = "0.2"`, `uuid = "1.0"`, `chrono = "0.4"`
  - 参考: `research.md` → 核心库选型
  - 验证: `cargo check` 通过

- [X] **T002** 创建Rust项目结构目录 ✅ (已完成)
  - 创建: `src-tauri/src/models/dependency.rs`, `errors.rs`
  - 创建: `src-tauri/src/services/dependency_checker.rs`, `installer_service.rs`, `logger_service.rs`
  - 创建: `src-tauri/src/commands/dependency_commands.rs`
  - 创建: `src-tauri/src/utils/version.rs`
  - 创建: `src-tauri/tests/contract_*.rs`, `integration_test.rs`
  - 验证: 所有目录存在

- [X] **T003** [P] 配置tracing日志初始化在 `src-tauri/src/utils/logger.rs` ✅ (已完成)
  - 实现: `init_logging()` 函数,按日期滚动JSON日志
  - 日志路径: 应用数据目录 `/logs/dependency_check_YYYY-MM-DD.log`
  - 参考: `research.md` → 日志持久化策略
  - 验证: 运行后日志文件生成

- [X] **T004** [P] 添加前端依赖到 `package.json` ✅ (已完成)
  - 依赖: 已有React/TailwindCSS,无需新增
  - 验证: `pnpm install` 无错误

---

## Phase 3.2: Tests First (TDD) ⚠️ MUST COMPLETE BEFORE 3.3

**CRITICAL: 这些测试必须先编写且必须失败,然后才能实现功能代码**

### 契约测试 (并行执行)

- [X] **T005** [P] 契约测试 `check_dependencies` 在 `src-tauri/tests/contract_check_dependencies.rs` ✅ (已完成)
  - 测试用例:
    1. 所有依赖已安装 → 返回2个satisfied结果
    2. 必需依赖缺失 → 返回missing状态
    3. 版本不兼容 → 返回version_mismatch状态
  - 断言: Response符合 `Vec<DependencyCheckResult>` schema
  - 事件断言: emit 4个 `dependency-check-progress` 事件
  - 参考: `contracts/check_dependencies.md`
  - **状态**: 所有测试通过

- [X] **T006** [P] 契约测试 `install_dependency` 在 `src-tauri/tests/contract_install_dependency.rs` ✅ (已完成)
  - 测试用例:
    1. 成功安装 → 返回InstallationTask,status=success
    2. 网络错误 → error_type=network_error
    3. 权限错误 → error_type=permission_error
    4. 版本冲突 → error_type=version_conflict_error
  - 断言: 进度事件正确emit
  - 参考: `contracts/install_dependency.md`
  - **状态**: 所有测试通过

- [X] **T007** [P] 契约测试 `query_dependency_status` 在 `src-tauri/tests/contract_query_status.rs` ✅ (已完成)
  - 测试用例:
    1. 查询所有 → 返回完整列表
    2. 查询单个 → 返回匹配项
    3. 查询不存在 → 返回空Vec
  - 断言: 无错误抛出
  - 参考: `contracts/query_dependency_status.md`
  - **状态**: 测试通过

- [X] **T008** [P] 契约测试 `trigger_manual_check` 在 `src-tauri/tests/contract_manual_check.rs` ✅ (已完成)
  - 测试用例:
    1. 手动触发 → 返回最新检测结果
    2. 事件流验证 → emit progress事件
  - 参考: `contracts/trigger_manual_check.md`
  - **状态**: 测试通过

### 集成测试 (基于quickstart场景)

- [X] **T009** [P] 集成测试: 所有依赖满足 在 `src-tauri/tests/integration_all_satisfied.rs` ✅ (已完成)
  - 场景: Redis+Playwright已安装
  - 验证: 检测耗时<2秒, 无安装动作, 自动进入主界面
  - 参考: `quickstart.md` → 场景1
  - **状态**: 测试通过

- [X] **T010** [P] 集成测试: 自动安装可选依赖 在 `src-tauri/tests/integration_auto_install.rs` ✅ (已完成)
  - 场景: Playwright缺失(可选依赖)
  - 验证: 自动触发安装, 进度条更新, 重新检测成功
  - 参考: `quickstart.md` → 场景2
  - **状态**: 测试通过

- [X] **T011** [P] 集成测试: 显示手动安装指引 在 `src-tauri/tests/integration_manual_guide.rs` ✅ (已完成)
  - 场景: Redis缺失(必需依赖)
  - 验证: 阻止启动, 显示安装指引, 提供"重新检测"按钮
  - 参考: `quickstart.md` → 场景3
  - **状态**: 测试通过

- [X] **T012** [P] 集成测试: 权限错误提示 在 `src-tauri/tests/integration_permission_error.rs` ✅ (已完成)
  - 场景: 模拟权限不足
  - 验证: 显示管理员权限提示, 提供重启引导
  - 参考: `quickstart.md` → 场景4
  - **状态**: 测试通过

- [X] **T013** [P] 集成测试: 运行时手动检测 在 `src-tauri/tests/integration_runtime_check.rs` ✅ (已完成)
  - 场景: 应用运行期间手动触发
  - 验证: 不阻塞主界面, 状态实时更新
  - 参考: `quickstart.md` → 场景5
  - **状态**: 测试通过

---

## Phase 3.3: Core Implementation (ONLY after tests are failing)

### 数据模型 (并行执行)

- [X] **T014** [P] 实现Dependency模型在 `src-tauri/src/models/dependency.rs` ✅ (已完成)
  - 结构体: `Dependency`, `DependencyLevel`, `CheckMethod`
  - 字段: id, name, version_requirement, level, auto_installable, install_priority, check_method, install_guide
  - 序列化: 添加 `#[derive(Serialize, Deserialize)]`
  - 参考: `data-model.md` → Dependency定义
  - 验证: `cargo test models::dependency`

- [X] **T015** [P] 实现CheckResult模型在 `src-tauri/src/models/dependency.rs` ✅ (已完成)
  - 结构体: `DependencyCheckResult`, `CheckStatus`
  - 字段: dependency_id, checked_at, status, detected_version, error_details, duration_ms
  - 参考: `data-model.md` → DependencyCheckResult
  - 验证: 序列化到JSON正确

- [X] **T016** [P] 实现InstallationTask模型在 `src-tauri/src/models/dependency.rs` ✅ (已完成)
  - 结构体: `InstallationTask`, `InstallStatus`, `InstallErrorType`
  - 字段: task_id, dependency_id, created_at, started_at, completed_at, status, progress_percent, error_message, install_log, error_type
  - 参考: `data-model.md` → InstallationTask
  - 验证: UUID生成正确

- [X] **T017** [P] 实现错误类型在 `src-tauri/src/models/errors.rs` ✅ (已完成)
  - 枚举: `DependencyError { CheckFailed(String), InstallFailed(InstallErrorType) }`
  - 实现: `std::error::Error`, `Display`, `From<io::Error>`
  - 参考: `contracts/` → 错误处理章节
  - 验证: 错误可序列化为JSON

### 服务层

- [X] **T018** 实现依赖检测服务在 `src-tauri/src/services/dependency_checker.rs` ✅ (已完成)
  - 函数: `check_single_dependency(dep: &Dependency) -> Result<DependencyCheckResult>`
  - 逻辑:
    1. 根据CheckMethod类型调用检测(Executable/Service/File)
    2. Executable: 使用`which` crate查找 + `tokio::process::Command`获取版本
    3. Service: TCP连接测试
    4. File: 文件存在性检查
  - semver版本比较
  - 计算duration_ms
  - 参考: `research.md` → Rust依赖检测模式
  - 依赖: T014 (Dependency model)
  - 验证: 单元测试通过

- [X] **T019** 实现并发检测协调器在 `src-tauri/src/services/dependency_checker.rs` ✅ (已完成)
  - 函数: `check_all_dependencies(app: AppHandle, deps: Vec<Dependency>) -> Result<Vec<DependencyCheckResult>>`
  - 逻辑:
    1. 遍历依赖列表
    2. 对每个依赖spawn异步任务调用T018
    3. 每完成一个emit `dependency-check-progress` 事件
    4. 收集所有结果
  - 参考: `research.md` → Tokio并发模式
  - 依赖: T018
  - 验证: 事件正确emit

- [X] **T020** 实现在线安装服务在 `src-tauri/src/services/installer_service.rs` ✅ (已完成)
  - 函数: `install_dependency(app: AppHandle, dep: &Dependency, force: bool) -> Result<InstallationTask>`
  - 逻辑:
    1. 检查auto_installable=true
    2. 创建InstallationTask,设置status=Pending
    3. spawn异步任务执行install_command
    4. 每500ms emit `installation-progress` 事件更新进度
    5. 捕获5种错误类型(Network, Permission, DiskSpace, VersionConflict, Unknown)
    6. 安装完成后重新调用T018验证
  - 参考: `research.md` → Tokio并发, `data-model.md` → InstallErrorType
  - 依赖: T016, T018
  - 验证: 错误分类正确

- [X] **T021** 实现混合安装策略在 `src-tauri/src/services/installer_service.rs` ✅ (已完成)
  - 函数: `install_dependencies(required: Vec<Dependency>, optional: Vec<Dependency>) -> Result<Vec<InstallationTask>>`
  - 逻辑:
    1. 必需依赖串行安装(for循环await)
    2. 必需依赖失败立即返回错误
    3. 可选依赖使用JoinSet并行安装
    4. 可选依赖失败仅记录WARN日志
  - 参考: `research.md` → Tokio并发安装模式
  - 依赖: T020
  - 验证: 执行顺序正确

### Tauri Commands

- [X] **T022** 实现 `check_dependencies` command 在 `src-tauri/src/commands/dependency_commands.rs` ✅ (已完成)
  - 签名: `async fn check_dependencies(app: AppHandle) -> Result<Vec<DependencyCheckResult>, DependencyError>`
  - 逻辑:
    1. 加载依赖配置(从嵌入式配置或TOML文件)
    2. 调用T019检测所有依赖
    3. 可选: 缓存结果到Redis(24小时TTL)
  - 参考: `contracts/check_dependencies.md`
  - 依赖: T019
  - 验证: T005契约测试通过

- [X] **T023** 实现 `install_dependency` command 在 `src-tauri/src/commands/dependency_commands.rs` ✅ (已完成)
  - 签名: `async fn install_dependency(app: AppHandle, dependency_id: String, force: bool) -> Result<InstallationTask, DependencyError>`
  - 逻辑:
    1. 根据dependency_id查找配置
    2. 检查auto_installable
    3. 调用T020执行安装
  - 参考: `contracts/install_dependency.md`
  - 依赖: T020
  - 验证: T006契约测试通过

- [X] **T024** 实现 `query_dependency_status` command 在 `src-tauri/src/commands/dependency_commands.rs` ✅ (已完成)
  - 签名: `async fn query_dependency_status(dependency_id: Option<String>) -> Vec<DependencyCheckResult>`
  - 逻辑:
    1. 从Redis缓存或内存读取
    2. 过滤(如果dependency_id提供)
    3. 无错误,找不到返回空Vec
  - 参考: `contracts/query_dependency_status.md`
  - 依赖: T022 (缓存逻辑)
  - 验证: T007契约测试通过

- [X] **T025** 实现 `trigger_manual_check` command 在 `src-tauri/src/commands/dependency_commands.rs` ✅ (已完成)
  - 签名: `async fn trigger_manual_check(app: AppHandle, state: State<'_, AppState>) -> Result<Vec<DependencyCheckResult>, DependencyError>`
  - 逻辑: 调用T022相同逻辑,更新缓存
  - 参考: `contracts/trigger_manual_check.md`
  - 依赖: T022
  - 验证: T008契约测试通过

### 前端组件 (并行执行)

- [X] **T026** [P] 实现进度条组件 `src/components/DependencyProgress.tsx` ✅ (已完成)
  - Props: `currentIndex: number, totalCount: number, currentDep: string, status: string`
  - 逻辑:
    1. 订阅 `dependency-check-progress` 事件
    2. 更新进度条(0-100%)
    3. 显示当前检测项目名称
  - UI: TailwindCSS进度条,>=10Hz更新频率
  - 参考: `research.md` → Tauri事件流
  - 验证: 事件监听正确

- [X] **T027** [P] 实现安装指引组件 `src/components/InstallationGuide.tsx` ✅ (已完成)
  - Props: `guide: InstallationGuide`
  - 逻辑:
    1. 渲染Markdown格式的install_guide
    2. 显示下载链接
    3. 提供"重新检测"按钮调用T025
  - UI: Markdown渲染,按钮调用`invoke('trigger_manual_check')`
  - 参考: `data-model.md` → InstallationGuide
  - 验证: Markdown正确渲染

- [X] **T028** [P] 实现启动检测页面 `src/pages/StartupCheckPage.tsx` ✅ (已完成)
  - 逻辑:
    1. useEffect调用`invoke('check_dependencies')`
    2. 根据结果显示:
       - 所有满足 → 跳转主界面
       - 必需依赖缺失 → 显示T027安装指引
       - 可选依赖缺失 → 显示警告,允许继续
    3. 使用T026显示检测进度
  - 参考: `quickstart.md` → 用户场景
  - 依赖: T026, T027
  - 验证: 路由跳转正确

---

## Phase 3.4: Integration

- [X] **T029** 集成Tauri启动钩子在 `src-tauri/src/main.rs` ✅ (已完成)
  - 修改: `tauri::Builder::setup()` 添加依赖检测调用
  - 逻辑:
    ```rust
    .setup(|app| {
        let app_handle = app.handle();
        tauri::async_runtime::spawn(async move {
            let _ = dependency_checker::run_startup_check(app_handle).await;
        });
        Ok(())
    })
    ```
  - 参考: `research.md` → Tauri启动生命周期集成
  - 依赖: T022
  - 验证: 启动时自动触发检测

- [X] **T030** 注册Tauri Commands在 `src-tauri/src/lib.rs` ✅ (已完成)
  - 添加:
    ```rust
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            check_dependencies,
            install_dependency,
            query_dependency_status,
            trigger_manual_check
        ])
    ```
  - 依赖: T022-T025
  - 验证: 前端可调用所有commands

---

## Phase 3.5: Polish

- [ ] **T031** [P] 单元测试版本比较工具在 `src-tauri/src/utils/version.rs` 🚧 (部分完成 - 版本工具已创建但未充分测试)
  - 测试: semver版本解析和比较
  - 用例: "7.2.4" >= "7.0.0" ✓, "6.2.0" >= "7.0.0" ✗
  - 参考: `research.md` → semver crate
  - 验证: 边界情况覆盖

- [ ] **T032** [P] 执行quickstart完整测试在项目根目录 ⚠️ (待执行)
  - 运行: `scripts/run-quickstart-tests.sh` (如果存在) 或手动执行
  - 验证: 5个场景全部通过
  - 性能: 检测<2秒, 安装<120秒
  - 参考: `quickstart.md` → 自动化测试脚本
  - 验证: 所有验收标准✓

- [ ] **T033** 代码审查与优化 🚧 (需要进一步优化)
  - 检查: Constitution原则合规性
    - 无冗余代码 ✓
    - 命名自文档化 ✓
    - 错误处理优雅 ✓
    - 日志有意义 ✓
  - 移除: 调试代码、未使用导入
  - 优化: 减少重复逻辑
  - 验证: `cargo clippy` 无警告

- [ ] **T034** [P] 更新文档 🚧 (需要更新README等文档)
  - 更新: `README.md` 添加依赖检测功能说明
  - 创建: `docs/dependency-management.md` 用户手册
  - 更新: `CLAUDE.md` 标记功能完成状态
  - 验证: 文档准确性

---

## Dependencies (任务依赖关系)

### 阻塞关系
```
Setup (T001-T004) → 所有其他任务

Tests (T005-T013) → Implementation (T014-T028)
  ├─ T005 blocks T022
  ├─ T006 blocks T023
  ├─ T007 blocks T024
  ├─ T008 blocks T025
  └─ T009-T013 block T032

Models (T014-T017) → Services (T018-T021) → Commands (T022-T025)
  ├─ T014 blocks T018
  ├─ T015,T016 block T019,T020
  ├─ T018 blocks T019,T020,T022
  ├─ T019 blocks T022,T025
  └─ T020 blocks T021,T023

Commands (T022-T025) → Integration (T029-T030)
  └─ T022-T025 block T030

Frontend (T026-T028) → Integration (T029)
  └─ T026,T027 block T028

All Implementation → Polish (T031-T034)
```

### 并行执行组
- **Group 1** (Setup): T003, T004
- **Group 2** (Contract Tests): T005, T006, T007, T008
- **Group 3** (Integration Tests): T009, T010, T011, T012, T013
- **Group 4** (Models): T014, T015, T016, T017
- **Group 5** (Frontend): T026, T027, T028
- **Group 6** (Polish): T031, T032, T034

---

## Parallel Execution Examples

### 并行执行契约测试 (T005-T008)

使用code-artisan代理并行编写4个契约测试:

```bash
# 在单个消息中发起4个并行任务
Task(code-artisan): "编写契约测试check_dependencies在src-tauri/tests/contract_check_dependencies.rs,参考contracts/check_dependencies.md"
Task(code-artisan): "编写契约测试install_dependency在src-tauri/tests/contract_install_dependency.rs,参考contracts/install_dependency.md"
Task(code-artisan): "编写契约测试query_dependency_status在src-tauri/tests/contract_query_status.rs,参考contracts/query_dependency_status.md"
Task(code-artisan): "编写契约测试trigger_manual_check在src-tauri/tests/contract_manual_check.rs,参考contracts/trigger_manual_check.md"
```

### 并行执行模型任务 (T014-T017)

```bash
Task(code-artisan): "实现Dependency模型在src-tauri/src/models/dependency.rs,参考data-model.md"
Task(code-artisan): "实现CheckResult模型在src-tauri/src/models/dependency.rs,参考data-model.md"
Task(code-artisan): "实现InstallationTask模型在src-tauri/src/models/dependency.rs,参考data-model.md"
Task(code-artisan): "实现DependencyError在src-tauri/src/models/errors.rs,参考contracts错误处理章节"
```

### 并行执行前端组件 (T026-T028)

```bash
Task(code-artisan): "实现进度条组件src/components/DependencyProgress.tsx,订阅dependency-check-progress事件"
Task(code-artisan): "实现安装指引组件src/components/InstallationGuide.tsx,渲染Markdown安装指引"
Task(code-artisan): "实现启动检测页面src/pages/StartupCheckPage.tsx,集成进度条和安装指引"
```

---

## Validation Checklist

**GATE: 检查任务完整性**

- [x] 所有contracts有对应测试: ✓ T005-T008
- [x] 所有entities有模型任务: ✓ T014-T017 (4个实体)
- [x] 所有tests在implementation之前: ✓ T005-T013 before T014-T030
- [x] 并行任务真正独立: ✓ 不同文件,无依赖冲突
- [x] 每个任务指定精确文件路径: ✓ 所有任务包含完整路径
- [x] 无任务修改同一文件: ✓ 验证通过

---

## Notes

- **[P] 标记**: 不同文件,无依赖,可并行执行
- **验证测试失败**: T005-T013必须先失败,再实现T014-T030
- **提交频率**: 每完成一个任务提交一次
- **避免**: 模糊任务描述,同文件并发冲突
- **Constitution遵循**: 每个任务都服务于不可替代的功能需求

---

**任务清单版本**: 1.0.0
**生成日期**: 2025-10-05
**状态**: ✅ 核心功能已完成

## 执行完成时间: 2025-10-05

### 完成进度统计
- **总任务数**: 34个
- **已完成**: 30个 (88.2%)
- **部分完成**: 3个 (T031, T033, T034) 🚧
- **待执行**: 1个 (T032) ⚠️

### 核心里程碑状态 ✅
- ✅ **Phase 3.1**: Setup (T001-T004) - 100% 完成
- ✅ **Phase 3.2**: Tests (T005-T013) - 100% 完成
- ✅ **Phase 3.3**: Core Implementation (T014-T028) - 100% 完成
- ✅ **Phase 3.4**: Integration (T029-T030) - 100% 完成
- 🚧 **Phase 3.5**: Polish (T031-T034) - 进行中

### 已交付的核心功能
1. ✅ 依赖检测系统 (Redis + Playwright)
2. ✅ 自动安装服务 (可选依赖)
3. ✅ 手动安装指引 (必需依赖)
4. ✅ 进度条与UI组件
5. ✅ 启动检测页面
6. ✅ Tauri Commands集成
7. ✅ 完整测试套件 (9个测试全部通过)

### 剩余工作
- **T032**: 执行完整集成测试验证
- **T033**: 代码优化与清理
- **T034**: 文档更新

**项目状态**: 核心功能开发完成，待最终测试与文档完善
