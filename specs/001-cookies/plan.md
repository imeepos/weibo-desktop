# Implementation Plan: 微博扫码登录获取Cookies

**Branch**: `001-cookies` | **Date**: 2025-10-05 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/001-cookies/spec.md`

## Execution Flow (/plan command scope)
```
1. Load feature spec from Input path
   → ✓ Feature spec loaded successfully
2. Fill Technical Context (scan for NEEDS CLARIFICATION)
   → ✓ All fields filled based on Tauri + Playwright stack
   → Detect Project Type: Desktop application (Tauri)
   → Set Structure Decision: Tauri frontend-backend architecture
3. Fill the Constitution Check section based on constitution document
   → ✓ Five core principles mapped to gates
4. Evaluate Constitution Check section
   → ✓ No violations detected
   → ✓ Progress Tracking: Initial Constitution Check PASS
5. Execute Phase 0 → research.md
   → ✓ Research document created with technical decisions
6. Execute Phase 1 → contracts, data-model.md, quickstart.md, CLAUDE.md
   → ✓ Design documents created
7. Re-evaluate Constitution Check section
   → ✓ Post-Design Constitution Check PASS
8. Plan Phase 2 → Task generation approach described
9. ✓ STOP - Ready for /tasks command
```

## Summary

用户需要通过微博官方扫码登录API获取网站cookies并存储到Redis。系统使用Tauri桌面框架构建,前端展示二维码和状态,后端Rust处理API调用、轮询、验证和存储。采用Playwright进行浏览器自动化,通过结构化日志记录所有关键事件,实现优雅的错误处理和高性能的异步操作。

**技术路径**: Tauri Commands 作为前后端桥梁,Rust async/await 处理微博API交互和Redis存储,React前端展示二维码和实时状态更新,tracing-subscriber 实现结构化日志,Playwright Node.js 处理浏览器自动化验证。

## Technical Context

**Language/Version**:
- 后端: Rust 1.75+ (stable)
- 前端: TypeScript 5.0+, React 18+
- 自动化: Node.js 20+ (Playwright runtime)

**Primary Dependencies**:
- Tauri 1.5+ (桌面框架)
- tokio 1.35+ (async runtime)
- reqwest 0.11+ (HTTP client)
- redis 0.24+ (Redis client)
- serde 1.0+ (序列化)
- tracing + tracing-subscriber 0.3+ (结构化日志)
- playwright (Node.js, 浏览器自动化)
- React 18+ + TailwindCSS 3+ (UI)
- @tauri-apps/api (前端桥接)

**Storage**:
- Redis 7+ (远程,cookies存储)
- 文件系统 (日志永久存储,使用 tracing-appender)

**Testing**:
- Rust: cargo test (单元测试)
- Integration: contract tests (契约测试)
- E2E: manual quickstart scenarios

**Target Platform**:
- Windows 10+, macOS 11+, Linux (X11/Wayland)
- Tauri 跨平台构建

**Project Type**: Desktop application (Tauri架构: frontend + backend)

**Performance Goals**:
- 二维码生成响应: <500ms
- 状态轮询间隔: 2-3秒
- Cookies验证: <2秒
- Redis操作: <100ms
- UI响应性: <16ms (60fps)

**Constraints**:
- 依赖微博官方API可用性和稳定性
- 单进程异步架构 (Tokio runtime)
- 日志文件大小管理 (rotation策略)
- Redis连接池限制 (最多10个连接)
- 内存占用: <200MB

**Scale/Scope**:
- 支持多账户 (最多50个活跃账户)
- 并发登录会话: 最多5个
- 日志保留: 按文件大小rotation (100MB/文件,保留最近10个)
- Redis keys: 按账户UID组织

## Constitution Check
*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

基于 `/workspace/desktop/.specify/memory/constitution.md` v1.0.0:

### 一、存在即合理 (Existence Implies Necessity)
- [x] **Gate 1.1**: 每个Tauri command都有明确的单一职责
  - `generate_qrcode`: 仅生成二维码
  - `poll_login_status`: 仅轮询状态
  - `save_cookies`: 仅存储cookies
  - `query_cookies`: 仅查询cookies
  - 无冗余或重复功能
- [x] **Gate 1.2**: 数据模型中的每个字段都不可替代
  - LoginSession: qr_id, status, timestamps → 状态跟踪必需
  - CookiesData: cookies, uid, timestamps → 存储和验证必需
  - LoginEvent: event_type, timestamp, session_id → 日志追溯必需
- [x] **Gate 1.3**: 无不必要的抽象层
  - 直接使用 reqwest 调用API,无Repository pattern
  - 直接使用 redis crate,无ORM
  - Tauri commands 直接调用业务逻辑,无中间层

### 二、优雅即简约 (Elegance is Simplicity)
- [x] **Gate 2.1**: 代码自文档化,无冗余注释
  - 函数名揭示意图: `generate_qrcode`, `validate_cookies_via_profile_api`
  - 类型名讲述故事: `LoginSession`, `QrCodeStatus::Scanned`
  - 错误类型有意义: `ApiError::QrCodeExpired`, `StorageError::RedisConnectionFailed`
- [x] **Gate 2.2**: 命名遵循领域语言
  - 使用微博术语: `uid`, `cookies`, `qrcode`
  - 状态转换清晰: `Pending → Scanned → Confirmed → Validated`
  - 事件类型明确: `Generated`, `Scanned`, `ConfirmedSuccess`, `ValidationFailed`
- [x] **Gate 2.3**: 代码流畅如散文
  - Rust类型系统表达状态: `Result<CookiesData, ValidationError>`
  - 异步流程清晰: `async fn poll_until_confirmed() -> Result<Cookies>`
  - 错误处理链式优雅: `.map_err(|e| ApiError::RequestFailed(e))?`

### 三、性能即艺术 (Performance is Art)
- [x] **Gate 3.1**: 算法选择优雅高效
  - 轮询使用 exponential backoff: 2s → 3s → 5s (避免API压力)
  - Redis连接池复用 (避免频繁创建连接)
  - 异步并发处理多账户 (tokio::spawn)
- [x] **Gate 3.2**: 性能目标可测量
  - 所有Performance Goals都有明确阈值
  - 使用 `tracing::instrument` 测量函数耗时
  - Redis操作超时设置: 5秒
- [x] **Gate 3.3**: 优化不损害可读性
  - 连接池配置明确: `max_size: 10, min_idle: 2`
  - 避免过早优化: 先实现正确性,再profile瓶颈

### 四、错误处理如为人处世的哲学 (Error Handling as Life Philosophy)
- [x] **Gate 4.1**: 错误类型有教育意义
  - `ApiError::QrCodeExpired { generated_at, expired_at }` → 提供时间上下文
  - `ValidationError::ProfileApiFailed { cookies_sample, api_response }` → 帮助诊断
  - `StorageError::RedisConnectionFailed { endpoint, retry_count }` → 指导重试
- [x] **Gate 4.2**: 失败路径优雅处理
  - 网络错误自动重试 (最多3次,exponential backoff)
  - Redis失败降级: 返回错误而非崩溃
  - API限流: 检测429状态码并延迟重试
- [x] **Gate 4.3**: 错误触发架构反思
  - 频繁的二维码过期 → 考虑调整轮询间隔
  - 验证失败率高 → 记录API响应用于分析
  - Redis连接失败 → 日志记录用于运维告警

### 五、日志是思想的表达 (Logs Express Thought)
- [x] **Gate 5.1**: 日志有意义且可操作
  - 使用结构化字段: `tracing::info!(uid = %uid, qr_id = %qr_id, "QR code generated")`
  - 关键路径必有日志: 生成、扫描、确认、验证、存储
  - 错误日志包含上下文: `tracing::error!(error = ?e, cookies = %cookies_sample, "Validation failed")`
- [x] **Gate 5.2**: 日志级别恰当
  - ERROR: API失败、验证失败、存储失败
  - WARN: 二维码过期、轮询超时、重试
  - INFO: 成功路径关键事件
  - DEBUG: 详细的API请求/响应 (仅开发环境)
- [x] **Gate 5.3**: 避免噪音日志
  - 轮询循环不记录每次请求,仅记录状态变化
  - 成功的Redis操作不记录,仅记录失败
  - 使用 `tracing::instrument(skip_all)` 避免敏感数据泄漏

**Initial Check Status**: ✅ PASS - 所有gates通过,无违规项

## Project Structure

### Documentation (this feature)
```
specs/001-cookies/
├── plan.md              # 本文件 (实施计划)
├── research.md          # Phase 0 技术研究
├── data-model.md        # Phase 1 数据模型设计
├── quickstart.md        # Phase 1 手动测试场景
└── contracts/           # Phase 1 Tauri命令契约
    ├── generate_qrcode.md
    ├── poll_login_status.md
    ├── save_cookies.md
    └── query_cookies.md
```

### Source Code (repository root)
```
desktop/                          # Tauri项目根目录
├── src-tauri/                    # Rust后端
│   ├── src/
│   │   ├── main.rs               # Tauri应用入口
│   │   ├── commands/             # Tauri命令处理器
│   │   │   ├── mod.rs
│   │   │   ├── qrcode.rs         # generate_qrcode
│   │   │   ├── login_poll.rs     # poll_login_status
│   │   │   ├── cookies_save.rs   # save_cookies
│   │   │   └── cookies_query.rs  # query_cookies
│   │   ├── services/             # 业务逻辑
│   │   │   ├── mod.rs
│   │   │   ├── weibo_api.rs      # 微博API调用
│   │   │   ├── cookies_validator.rs  # Cookies验证
│   │   │   └── redis_storage.rs  # Redis存储
│   │   ├── models/               # 数据模型
│   │   │   ├── mod.rs
│   │   │   ├── login_session.rs  # LoginSession
│   │   │   ├── cookies_data.rs   # CookiesData
│   │   │   └── login_event.rs    # LoginEvent
│   │   ├── errors.rs             # 错误类型定义
│   │   └── logging.rs            # 日志配置
│   ├── Cargo.toml
│   └── tauri.conf.json
│
├── src/                          # React前端
│   ├── components/
│   │   ├── QrCodeDisplay.tsx     # 二维码展示组件
│   │   ├── LoginStatus.tsx       # 登录状态组件
│   │   └── CookiesViewer.tsx     # Cookies查看组件
│   ├── hooks/
│   │   ├── useQrCodeLogin.ts     # 登录逻辑hook
│   │   └── useCookiesQuery.ts    # 查询逻辑hook
│   ├── services/
│   │   └── tauri-commands.ts     # Tauri命令封装
│   ├── App.tsx
│   └── main.tsx
│
├── playwright/                   # Playwright自动化脚本
│   ├── package.json
│   ├── validate-cookies.js       # Cookies验证脚本
│   └── weibo-profile-api.js      # 微博资料API调用
│
├── tests/                        # 集成测试
│   ├── contract/                 # 契约测试
│   │   ├── test_generate_qrcode.rs
│   │   ├── test_poll_login_status.rs
│   │   ├── test_save_cookies.rs
│   │   └── test_query_cookies.rs
│   └── integration/              # 集成测试
│       └── test_login_flow.rs
│
├── logs/                         # 日志文件目录 (运行时生成)
│   └── weibo-login.log
│
├── package.json                  # 前端依赖
├── pnpm-workspace.yaml           # pnpm workspace配置
└── README.md
```

**Structure Decision**:
选择 Tauri 桌面应用架构,包含:
1. **src-tauri/**: Rust后端,处理所有业务逻辑、API调用、存储和日志
2. **src/**: React前端,负责UI展示和用户交互
3. **playwright/**: 独立的Node.js项目,用于浏览器自动化验证cookies
4. **tests/**: Rust测试,包含契约测试和集成测试

此结构符合Tauri最佳实践,前后端分离,职责清晰,便于并行开发和测试。

## Phase 0: Outline & Research

### 提取的未知项和研究任务

基于 Technical Context,需要研究的关键技术决策:

1. **Tauri与Playwright集成**: Tauri后端如何调用Node.js Playwright脚本进行cookies验证
2. **微博扫码登录API**: 官方API的调用模式、参数、响应格式、错误码
3. **Rust异步模式**: Tokio + Tauri commands的最佳实践,避免阻塞主线程
4. **Redis连接池管理**: 最佳配置、连接复用、错误处理
5. **结构化日志方案**: tracing-subscriber的配置、文件rotation、性能影响

### 研究方法

为每个未知项生成研究任务:
- 微博API: 查阅官方文档、分析网络请求、确定endpoint和参数
- Tauri+Playwright: 探索进程间通信方案 (IPC, child_process, sidecar)
- Rust async: 研究 `#[tauri::command]` 的async支持和最佳实践
- Redis: 评估 `redis` vs `deadpool-redis` vs `bb8-redis` 连接池
- Logging: 配置 `tracing-appender` 的文件rotation和性能优化

### 研究成果整合

所有研究结果记录在 `research.md`,格式为:
```markdown
## [主题]
**Decision**: [选择的方案]
**Rationale**: [选择理由]
**Alternatives**: [考虑过的其他方案及其缺点]
**Implementation Notes**: [实施要点]
```

**Output**: research.md (见同目录文件)

## Phase 1: Design & Contracts

*Prerequisites: research.md complete*

### 1. 数据模型设计

从feature spec的Key Entities提取,输出到 `data-model.md`:

**LoginSession** (登录会话):
- 字段: qr_id (String), status (Enum), created_at, scanned_at?, confirmed_at?
- 状态转换: Pending → Scanned → Confirmed → Expired
- 验证规则: qr_id非空, 时间戳递增

**CookiesData** (Cookies数据):
- 字段: uid (String), cookies (HashMap<String, String>), fetched_at, validated_at, redis_key
- 验证规则: uid非空, cookies非空, 通过profile API验证
- 存储格式: Redis Hash, key = `weibo:cookies:{uid}`

**LoginEvent** (登录事件):
- 字段: event_type (Enum), timestamp, session_id, details (JSON)
- 事件类型: Generated, Scanned, Confirmed, ValidationSuccess, ValidationFailed, Error
- 日志格式: 结构化JSON,输出到文件

### 2. API契约生成

从functional requirements提取用户操作,生成Tauri命令契约:

**Contracts**:
1. `generate_qrcode` → `/contracts/generate_qrcode.md`
   - Request: 无参数
   - Response: `{ qr_id: string, qr_image: base64, expires_at: timestamp }`
   - Errors: `ApiError::NetworkFailed`, `ApiError::InvalidResponse`

2. `poll_login_status` → `/contracts/poll_login_status.md`
   - Request: `{ qr_id: string }`
   - Response: `{ status: "pending" | "scanned" | "confirmed" | "expired", cookies?: CookiesData }`
   - Errors: `ApiError::QrCodeNotFound`, `ApiError::QrCodeExpired`

3. `save_cookies` → `/contracts/save_cookies.md`
   - Request: `{ uid: string, cookies: Map<string, string> }`
   - Response: `{ success: boolean, redis_key: string }`
   - Errors: `ValidationError::ProfileApiFailed`, `StorageError::RedisConnectionFailed`

4. `query_cookies` → `/contracts/query_cookies.md`
   - Request: `{ uid: string }`
   - Response: `{ cookies: Map<string, string>, fetched_at: timestamp }`
   - Errors: `StorageError::NotFound`, `StorageError::RedisConnectionFailed`

### 3. 契约测试生成

为每个契约生成测试文件 (Phase 2实施阶段创建):
- `tests/contract/test_generate_qrcode.rs`: 测试生成二维码响应schema
- `tests/contract/test_poll_login_status.rs`: 测试状态轮询响应
- `tests/contract/test_save_cookies.rs`: 测试存储响应
- `tests/contract/test_query_cookies.rs`: 测试查询响应

### 4. 测试场景提取

从User Scenarios提取,输出到 `quickstart.md`:
- 场景1: 生成二维码 → 手动验证二维码显示
- 场景2: 扫描二维码 → 验证状态更新为"已扫描"
- 场景3: 确认登录 → 验证cookies获取和存储
- 场景4: 查询cookies → 验证从Redis读取
- 场景5: 重复登录 → 验证覆盖逻辑

### 5. 更新Agent上下文

执行脚本:
```bash
.specify/scripts/bash/update-agent-context.sh claude
```

增量更新 `/workspace/desktop/CLAUDE.md`:
- 添加新技术栈: Tauri 1.5+, Tokio, Redis, Playwright
- 添加项目结构: src-tauri/, src/, playwright/
- 更新最近变更: 微博扫码登录功能设计完成
- 保持文件在150行以内

**Output**:
- data-model.md
- contracts/generate_qrcode.md
- contracts/poll_login_status.md
- contracts/save_cookies.md
- contracts/query_cookies.md
- quickstart.md
- CLAUDE.md (更新)

## Phase 2: Task Planning Approach

*This section describes what the /tasks command will do - DO NOT execute during /plan*

### 任务生成策略

基于Phase 1的设计文档生成任务:

**从契约生成测试任务**:
- 每个contract → 1个契约测试任务 [P] (可并行)
- 测试验证Request/Response schema
- 使用mock数据,测试应失败 (无实现)

**从数据模型生成任务**:
- 每个entity → 1个模型创建任务 [P]
- 包含字段定义、验证逻辑、状态转换
- 单元测试覆盖

**从用户故事生成任务**:
- 每个acceptance scenario → 1个集成测试任务
- 覆盖完整的用户流程
- 依赖模型和服务实现

**实施任务**:
- 实现Tauri commands (依赖模型和服务)
- 实现微博API服务 (依赖HTTP client)
- 实现Redis存储服务 (依赖连接池)
- 实现Cookies验证服务 (依赖Playwright脚本)
- 实现日志配置 (独立任务)
- 实现前端组件 (依赖Tauri commands)

### 任务排序策略

**TDD顺序**: 测试先于实现
1. 契约测试 (定义接口)
2. 单元测试 (定义行为)
3. 实现代码 (让测试通过)
4. 集成测试 (验证流程)

**依赖顺序**:
1. 基础设施: 错误类型、日志配置 [P]
2. 数据模型: LoginSession, CookiesData, LoginEvent [P]
3. 服务层: WeiboApiService, RedisStorage, CookiesValidator [P]
4. 命令层: Tauri commands (依赖服务层)
5. 前端层: React组件 (依赖命令层)
6. 集成测试: 端到端流程 (依赖所有层)

**并行标记**:
- 独立文件的任务标记 [P]
- 相同依赖的任务可并行
- 跨层依赖必须串行

### 预估输出

**任务数量**: 约28-32个任务

**任务分类**:
- 基础设施: 3个任务 (错误、日志、配置)
- 数据模型: 3个任务 (3个实体)
- 契约测试: 4个任务 (4个命令)
- 服务实现: 5个任务 (API、存储、验证、Playwright、连接池)
- 命令实现: 4个任务 (4个Tauri commands)
- 前端实现: 5个任务 (3个组件 + 2个hooks)
- 集成测试: 5个任务 (覆盖5个核心场景)
- 文档和配置: 3个任务 (README, Cargo.toml, package.json)

**IMPORTANT**: This phase is executed by the /tasks command, NOT by /plan

## Phase 3+: Future Implementation

*These phases are beyond the scope of the /plan command*

**Phase 3**: Task execution (/tasks command creates tasks.md)
**Phase 4**: Implementation (execute tasks.md following constitutional principles)
**Phase 5**: Validation (run tests, execute quickstart.md, performance validation)

### 验证标准

Phase 5将验证:
- 所有契约测试通过
- 所有单元测试通过
- 所有集成测试通过
- quickstart.md场景手动验证通过
- 性能目标达成 (使用 `cargo bench` 或 `criterion`)
- 日志输出符合规范 (结构化、有意义、无噪音)
- Constitution Check所有gates仍然PASS

## Complexity Tracking

*填写仅当Constitution Check有必须合理化的违规项时*

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| N/A | 无违规项 | N/A |

## Progress Tracking

*This checklist is updated during execution flow*

**Phase Status**:
- [x] Phase 0: Research complete (/plan command)
  - ✓ research.md created with all technical decisions
  - ✓ Tauri+Playwright integration approach defined
  - ✓ Weibo API calling patterns documented
  - ✓ Redis connection pool strategy finalized
  - ✓ Structured logging configuration planned
- [x] Phase 1: Design complete (/plan command)
  - ✓ data-model.md created with 3 core entities
  - ✓ 4 Tauri command contracts defined
  - ✓ quickstart.md with 10 test scenarios
  - ✓ CLAUDE.md updated with project context
- [x] Phase 2: Task planning complete (/plan command - describe approach only)
  - ✓ Task generation strategy documented
  - ✓ TDD and dependency ordering defined
  - ✓ Estimated 28-32 tasks
- [ ] Phase 3: Tasks generated (/tasks command)
- [ ] Phase 4: Implementation complete
- [ ] Phase 5: Validation passed

**Gate Status**:
- [x] Initial Constitution Check: PASS
- [x] Post-Design Constitution Check: PASS
  - ✓ Re-evaluated all 5 principles against final design
  - ✓ No new violations introduced
  - ✓ All design decisions align with constitution
- [x] All NEEDS CLARIFICATION resolved
  - ✓ No NEEDS CLARIFICATION markers in Technical Context
  - ✓ All research questions answered in research.md
- [x] Complexity deviations documented
  - ✓ No violations detected
  - ✓ Complexity Tracking table: N/A

---
*Based on Constitution v1.0.0 - See `.specify/memory/constitution.md`*
