
# Implementation Plan: 微博关键字增量爬取

**Branch**: `003-` | **Date**: 2025-10-07 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/home/ubuntu/worktrees/desktop/specs/003-/spec.md`

## Execution Flow (/plan command scope)
```
1. Load feature spec from Input path
   → If not found: ERROR "No feature spec at {path}"
2. Fill Technical Context (scan for NEEDS CLARIFICATION)
   → Detect Project Type from file system structure or context (web=frontend+backend, mobile=app+api)
   → Set Structure Decision based on project type
3. Fill the Constitution Check section based on the content of the constitution document.
4. Evaluate Constitution Check section below
   → If violations exist: Document in Complexity Tracking
   → If no justification possible: ERROR "Simplify approach first"
   → Update Progress Tracking: Initial Constitution Check
5. Execute Phase 0 → research.md
   → If NEEDS CLARIFICATION remain: ERROR "Resolve unknowns"
6. Execute Phase 1 → contracts, data-model.md, quickstart.md, agent-specific template file (e.g., `CLAUDE.md` for Claude Code, `.github/copilot-instructions.md` for GitHub Copilot, `GEMINI.md` for Gemini CLI, `QWEN.md` for Qwen Code, or `AGENTS.md` for all other agents).
7. Re-evaluate Constitution Check section
   → If new violations: Refactor design, return to Phase 1
   → Update Progress Tracking: Post-Design Constitution Check
8. Plan Phase 2 → Describe task generation approach (DO NOT create tasks.md)
9. STOP - Ready for /tasks command
```

**IMPORTANT**: The /plan command STOPS at step 7. Phases 2-4 are executed by other commands:
- Phase 2: /tasks command creates tasks.md
- Phase 3-4: Implementation execution (manual or via tools)

## Summary

本功能实现微博关键字的历史回溯和增量更新爬取。用户指定关键字和事件开始时间后,系统从当前时间向后回溯收集所有历史帖子,并支持持续获取新发布的内容。核心技术挑战包括:

1. **突破50页限制**: 微博搜索API最多返回50页结果,通过时间分片策略将大时间范围拆分成更小的子范围,确保每个子范围内帖子数<50页
2. **断点续爬**: 在每页爬取后保存检查点(时间范围+页码+方向),支持任意阶段中断后恢复
3. **时间精度处理**: 微博帖子时间仅精确到小时,需通过帖子ID去重和时间边界重叠处理避免重复
4. **复用登录态**: 集成001-cookies功能,通过`query_cookies`从Redis获取已验证的登录凭证
5. **反爬虫应对**: 随机延迟+验证码检测+优雅暂停机制

技术方案: Rust后端(Tauri commands + 爬取服务) + Redis存储(Sorted Set按时间排序帖子) + Playwright爬取脚本 + React前端展示进度。

## Technical Context

**Language/Version**: Rust 1.75+ (后端), TypeScript 5.0+ (前端), Node.js 20+ (Playwright)
**Primary Dependencies**:
- 后端: Tauri 2.x, tokio (异步运行时), reqwest (HTTP客户端), redis/deadpool-redis (存储), chrono (时间处理), serde (序列化), tracing (日志)
- 前端: React 18+, Vite, TailwindCSS 3+, @tauri-apps/api
- 自动化: Playwright (已有独立server,复用001-cookies架构)

**Storage**: Redis 7+ (复用RedisService连接池)
- 任务信息: Hash (`crawl:task:{task_id}`)
- 帖子存储: Sorted Set (`crawl:posts:{task_id}`, score=发布时间戳) 支持时间范围查询
- 检查点: Hash (`crawl:checkpoint:{task_id}`)
- 去重索引: Set (`crawl:post_ids:{task_id}`)

**Testing**:
- Rust: cargo test (单元测试 + 契约测试)
- E2E: Playwright测试脚本
- 契约测试: 基于contracts/生成的测试用例

**Target Platform**: Linux/Windows/macOS Desktop (Tauri跨平台)

**Project Type**: web (Tauri = frontend + backend)

**Performance Goals**:
- 支持百万级帖子存储 (Redis Sorted Set高效范围查询)
- 每页爬取延迟1-3秒 (防反爬)
- 断点恢复时间 <1秒 (直接从Redis读取检查点)
- 帖子查询按时间范围 <100ms

**Constraints**:
- 微博搜索最多50页分页限制 (通过时间分片突破)
- 帖子时间精度仅到小时 (需通过ID去重)
- 防反爬机制: 需随机延迟、User-Agent轮换、验证码检测
- 单任务顺序执行 (避免并发爬取触发限流)

**Scale/Scope**:
- 单用户多任务管理 (任务间独立存储)
- 热门关键字可能数百万帖子 (需高效存储和分页查询)
- 支持长时间运行 (历史回溯可能持续数小时到数天)
- 前端实时进度展示 (通过Tauri事件推送)

**Cookies来源**: 复用001-cookies功能,通过`query_cookies` Tauri command从Redis获取已验证的登录态,避免重复扫码登录

## Constitution Check
*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

基于宪章v1.0.0的5个核心原则评估此功能:

### 一、存在即合理 (Existence Implies Necessity)

**评估结果**: ✅ PASS

**新增抽象及其不可替代性**:
1. **CrawlTask模型**: 表示爬取任务的生命周期状态,无法用LoginSession替代(不同的业务实体)
2. **WeiboPost模型**: 存储帖子内容和元数据,是核心数据实体,不可或缺
3. **CrawlCheckpoint模型**: 支持断点续爬的检查点,存储时间范围和页码,无法与任务信息合并(需独立更新频率)
4. **TimeShardService**: 封装时间分片算法,处理50页限制,算法复杂度高(递归拆分),必须独立抽象
5. **CrawlService**: 协调爬取逻辑、状态管理、进度推送,是不可替代的业务编排层

**复用现有抽象**:
- 复用`RedisService`: 不重复实现连接池管理
- 复用`CookiesData`: 通过`query_cookies`获取登录态,避免重复扫码
- 复用Playwright server架构: 不重新设计爬虫基础设施

**避免冗余**:
- 不创建单独的"爬取配置"模型(延迟/重试参数直接内置在CrawlService中)
- 不创建"导出服务"的抽象层(导出逻辑简单,直接在command中实现)

### 二、优雅即简约 (Elegance is Simplicity)

**评估结果**: ✅ PASS

**代码自文档化策略**:
1. **命名讲述故事**:
   - `floor_to_hour()` / `ceil_to_hour()`: 函数名直接揭示时间处理意图
   - `split_time_range_if_needed()`: 名称描述了时间分片的条件逻辑
   - `CrawlDirection::Backward` / `Forward`: 枚举值清晰表达爬取方向

2. **最小注释原则**:
   - 复杂算法(时间分片递归)需注释说明边界条件
   - 微博API的URL结构需注释说明参数含义
   - 其他代码通过类型和命名自解释

3. **类型系统表达约束**:
   - 使用`DateTime<Utc>`而非时间戳字符串,类型即文档
   - 使用枚举`CrawlStatus`而非字符串,状态机清晰可见

### 三、性能即艺术 (Performance is Art)

**评估结果**: ✅ PASS

**算法优雅性**:
1. **Redis Sorted Set存储**: 自然支持按时间范围查询(O(log N + M)),无需索引设计
2. **时间分片算法**: 递归二分时间范围,最坏情况O(log T),优雅且高效
3. **增量爬取**: 只查询最大时间后的帖子,避免全量扫描

**性能与可读性平衡**:
- 使用`ZADD`批量写入帖子(而非逐条插入),性能提升10x,代码仍清晰
- 检查点保存使用`HSET`,简单直接,无过早优化
- 去重使用`SADD`(O(1)),而非先`SISMEMBER`再`SADD`(减少一次往返)

**避免过早优化**:
- 暂不实现帖子分页查询(等实际需求明确后再优化)
- 暂不实现多任务并行(先保证单任务正确性)

### 四、错误处理如为人处世的哲学 (Error Handling as Life Philosophy)

**评估结果**: ✅ PASS

**优雅错误处理设计**:
1. **反爬检测**: 检测到验证码时优雅暂停任务,通过事件通知用户"需要人工处理",而非持续重试加剧风险
2. **网络失败**: 重试3次后进入`Failed`状态,记录失败原因,用户可手动恢复(resume命令)
3. **时间分片失败**: 当时间范围无法再拆分(如1小时内仍>50页)时,记录警告日志并跳过该时间段,保证任务继续
4. **Redis连接失败**: 复用`StorageError`,错误消息包含上下文(task_id, operation),便于诊断

**错误即架构反思**:
- 如果频繁遇到"50页限制",说明时间分片粒度不够细,触发算法调整
- 如果大量任务进入`Failed`状态,说明反爬策略需优化(延迟时长/User-Agent池)

### 五、日志是思想的表达 (Logs Express Thought)

**评估结果**: ✅ PASS

**结构化日志设计**:
1. **进度追踪**: `任务ID={}, 当前时间范围={}-{}, 当前页={}, 已爬取={}`
2. **状态转换**: `任务 {} 从 {} 转换到 {}, 触发原因: {}`
3. **异常情况**: `任务 {} 检测到验证码, URL={}, 截图已保存: {}`
4. **性能指标**: `时间分片完成, 原始范围={}-{}, 拆分为{}个子范围, 耗时{}ms`

**避免噪音日志**:
- 不记录每条帖子的详细内容(仅记录帖子ID和数量统计)
- 不在循环内打印DEBUG日志(仅在每页完成时记录INFO)
- 敏感数据(cookies)仅记录键名,不记录值(复用`sample_for_logging()`模式)

**结论**: 所有宪章原则均满足,设计哲学与项目章程对齐。

---

## Constitution Check (Post-Design Re-evaluation)

**重新评估时间**: Phase 1完成后

### 设计验证结果

经过完整的数据模型设计、契约定义和项目结构规划后,重新评估5个核心原则:

#### 一、存在即合理 (Existence Implies Necessity) - ✅ PASS

**验证**:
- 3个核心模型 (CrawlTask, WeiboPost, CrawlCheckpoint) 职责清晰,无重叠
- 2个核心服务 (CrawlService, TimeShardService) 算法复杂度高,必须独立
- ExportService被移除 (导出逻辑简单,直接在command中实现) ✅ 证明设计自省有效
- 6个Tauri commands对应6个用户操作,无冗余

**新发现的潜在冗余** (已消除):
- 初始设计中的"爬取配置模型"被移除 → 参数直接内置在CrawlService
- "导出服务"抽象被移除 → 逻辑简单,无需抽象层

#### 二、优雅即简约 (Elegance is Simplicity) - ✅ PASS

**验证**:
- 契约文档完整定义输入输出,代码实现时无需额外注释
- 状态机转换函数`can_transition_to()`自解释,名称即文档
- Redis存储键命名模式统一: `crawl:{entity}:{id}`

**命名审查**:
- `split_time_range_if_needed()` ✅ 名称揭示条件逻辑
- `advance_page()` vs `next_page()` → 选择`advance_page`更具动作性
- `complete_current_shard()` ✅ 清晰表达"完成当前分片并进入下一个"

#### 三、性能即艺术 (Performance is Art) - ✅ PASS

**验证**:
- Redis数据结构选择经过算法复杂度分析 (见research.md)
- 时间分片算法O(log T)优雅且高效
- 批量写入 (`ZADD`多个member) 减少网络往返

**无过早优化证据**:
- 未实现帖子分页查询 (先实现基础功能)
- 未实现多任务并行 (先保证单任务正确性)
- 未实现User-Agent池 (固定随机延迟已足够)

#### 四、错误处理如为人处世的哲学 (Error Handling as Life Philosophy) - ✅ PASS

**验证**:
- 6个契约定义了21个错误码,覆盖所有失败场景
- 验证码检测 → 优雅暂停 (而非持续重试) ✅ 体现"保护用户"哲学
- 网络错误 → 重试3次后进入Failed状态 → 用户可手动恢复 ✅ 赋予用户控制权
- 每个错误消息包含上下文 (task_id, operation, details) ✅ 便于诊断

**错误状态转换审查**:
- Failed状态允许转换回HistoryCrawling ✅ 支持手动重试
- Paused状态可恢复到上次活跃状态 ✅ 灵活恢复

#### 五、日志是思想的表达 (Logs Express Thought) - ✅ PASS

**验证**:
- quickstart.md中定义了4类结构化日志:
  1. 进度追踪: `任务ID={}, 当前时间范围={}-{}, 已爬取={}`
  2. 状态转换: `任务 {} 从 {} 转换到 {}`
  3. 异常情况: `任务 {} 检测到验证码, URL={}`
  4. 性能指标: `时间分片完成, 拆分为{}个子范围, 耗时{}ms`
- 所有日志使用tracing crate的结构化日志,支持过滤和聚合
- 敏感数据不记录 (cookies仅记录键名,复用`sample_for_logging()`模式)

**日志级别分配**:
- ERROR: 导致任务失败的错误 (网络超时、Redis连接失败)
- WARN: 需要注意但不致命的情况 (50页限制、cookies即将过期)
- INFO: 正常进度和状态变化 (任务启动、每页完成)
- DEBUG: 详细调试信息 (时间分片计算、检查点保存)

---

### 设计完整性检查

1. **数据模型完整性**: ✅
   - 所有spec.md中的Key Entities都有对应的Rust结构
   - 所有字段都有验证规则
   - 状态机转换完整覆盖

2. **契约完整性**: ✅
   - spec.md中的所有Functional Requirements都有对应的契约
   - 每个契约包含: 请求/响应schema、错误码、测试要点
   - 6个契约覆盖CRUD + 控制 (start/pause/export)

3. **测试覆盖完整性**: ✅
   - quickstart.md定义8个端到端场景
   - 覆盖正常流程 + 边界情况 + 异常处理
   - 包含性能验证标准

4. **与001-cookies集成明确性**: ✅
   - 集成点唯一: `query_cookies` command
   - 依赖关系清晰: 003仅在创建任务时依赖001
   - 隔离策略明确: 运行时完全独立

---

### 最终评估

**结论**: ✅ **PASS** - 所有宪章原则在设计阶段仍然满足,无新增违规项

**复杂度证明**:
- TimeShardService的复杂度是本质性的 (递归二分算法),无法简化
- CrawlService的职责是不可替代的 (编排多个服务,管理异步任务)

**设计改进** (相比初始评估):
- 移除了2个不必要的抽象 (爬取配置模型、导出服务)
- 证明了设计过程中的持续自省和简化

## Project Structure

### Documentation (this feature)
```
specs/003-/
├── plan.md              # This file (/plan command output)
├── spec.md              # Feature specification
├── research.md          # Phase 0 output - 技术方案研究
├── data-model.md        # Phase 1 output - 数据模型设计
├── quickstart.md        # Phase 1 output - 端到端测试场景
├── contracts/           # Phase 1 output - Tauri命令契约
│   ├── create_crawl_task.md
│   ├── start_crawl.md
│   ├── pause_crawl.md
│   ├── get_crawl_progress.md
│   ├── export_crawl_data.md
│   └── list_crawl_tasks.md
└── tasks.md             # Phase 2 output (/tasks command - NOT created by /plan)
```

### Source Code (repository root)

**Tauri架构** (Project Type=web: Rust后端 + React前端):

```
src-tauri/src/
├── models/
│   ├── mod.rs
│   ├── crawl_task.rs          # CrawlTask, CrawlStatus
│   ├── weibo_post.rs          # WeiboPost
│   ├── crawl_checkpoint.rs    # CrawlCheckpoint, CrawlDirection
│   ├── crawl_errors.rs        # CrawlError枚举 (复用StorageError, ApiError)
│   └── crawl_events.rs        # CrawlProgressEvent, CrawlCompletedEvent, CrawlErrorEvent
│
├── services/
│   ├── mod.rs
│   ├── redis_service.rs       # [已存在] 复用连接池,扩展爬取任务存储方法
│   ├── crawl_service.rs       # CrawlService: 核心爬取编排逻辑
│   ├── time_shard_service.rs  # TimeShardService: 时间分片算法
│   └── export_service.rs      # ExportService: JSON/CSV导出 (或直接在command中实现)
│
├── commands/
│   ├── mod.rs
│   ├── cookies_commands.rs    # [已存在] query_cookies, list_all_uids等
│   └── crawl_commands.rs      # create_crawl_task, start_crawl, pause_crawl等
│
├── utils/
│   ├── mod.rs
│   └── time_utils.rs          # floor_to_hour, ceil_to_hour, parse_weibo_time
│
├── main.rs                    # Tauri应用入口,注册commands
├── lib.rs
└── Cargo.toml

src/
├── components/
│   ├── CrawlTaskList.tsx      # 任务列表组件
│   ├── CrawlTaskForm.tsx      # 新建任务表单
│   ├── CrawlProgress.tsx      # 实时进度展示 (进度条、统计)
│   ├── CrawlStatusBadge.tsx   # 状态徽章 (Created/Running/Paused/Failed)
│   └── ExportDialog.tsx       # 导出对话框 (选择格式/时间范围)
│
├── pages/
│   └── CrawlPage.tsx          # 爬取功能主页面
│
├── hooks/
│   ├── useCrawlTask.ts        # 任务CRUD操作
│   ├── useCrawlProgress.ts    # 监听进度事件
│   └── useCrawlExport.ts      # 导出操作
│
└── types/
    └── crawl.ts               # 前端类型定义 (对应后端模型)

playwright/src/
├── weibo-crawler.ts           # 微博爬取脚本 (复用已有server架构)
└── types/                     # WebSocket通信协议类型定义

tests/
├── contract/
│   ├── test_create_crawl_task.rs    # 契约测试: create_crawl_task
│   ├── test_start_crawl.rs
│   ├── test_pause_crawl.rs
│   ├── test_get_crawl_progress.rs
│   ├── test_export_crawl_data.rs
│   └── test_list_crawl_tasks.rs
│
├── integration/
│   ├── test_time_shard.rs           # 集成测试: 时间分片算法
│   ├── test_checkpoint_resume.rs    # 集成测试: 断点续爬
│   └── test_captcha_handling.rs     # 集成测试: 验证码检测
│
└── unit/
    ├── test_crawl_task_model.rs     # 单元测试: CrawlTask状态机
    ├── test_time_utils.rs           # 单元测试: 时间处理函数
    └── test_weibo_post_validation.rs # 单元测试: WeiboPost验证
```

**Structure Decision**:
- 选择Tauri架构 (web类型: frontend + backend)
- Rust后端职责: 业务逻辑、状态管理、Redis存储、Playwright通信
- React前端职责: UI展示、事件监听、用户交互
- Playwright职责: 浏览器自动化、微博API调用
- 测试分三层: 契约测试 (验证API接口) → 集成测试 (验证业务流程) → 单元测试 (验证函数逻辑)

**复用策略**:
- `RedisService`: 扩展方法,不修改已有接口
- `cookies_commands.rs`: 复用`query_cookies`获取登录态
- Playwright server架构: 复用WebSocket通信机制,新增爬取脚本

**隔离策略**:
- 爬取功能的模型、服务、命令独立于001-cookies
- 仅在创建任务时依赖`query_cookies`,运行时完全独立
- 前端新增独立页面,不修改已有页面

## Phase 0: Outline & Research

**Status**: ✅ 已完成

**研究成果** (`research.md`):

1. **微博搜索URL结构和限制**
   - 决策: 使用移动端API `https://m.weibo.cn/api/container/getIndex`
   - 50页限制确认,通过时间分片突破

2. **Redis存储策略**
   - 决策: Sorted Set (帖子) + Hash (任务/检查点) + Set (去重)
   - 理由: 自然支持时间范围查询,O(1)去重检查

3. **时间分片算法**
   - 决策: 递归二分时间范围 + 自适应分片
   - 最小粒度: 1小时

4. **断点续爬检查点设计**
   - 决策: 三级检查点 (任务级 + 分片级 + 页级)
   - 恢复精度: 从下一页继续

5. **Playwright爬取脚本架构**
   - 决策: 复用001-cookies的server架构
   - WebSocket通信协议定义

6. **与001-cookies集成方式**
   - 决策: 通过`query_cookies`获取登录态
   - Cookies有效期检查: 7天

7. **导出格式**
   - 决策: JSON + CSV双格式支持

8. **NEEDS CLARIFICATION解决**:
   - FR-018: 复用001-cookies
   - FR-025: 指数退避重试 (1s/2s/4s)
   - FR-027: 固定随机延迟1-3秒
   - 并发策略: 单任务顺序执行

**输出**: `/home/ubuntu/worktrees/desktop/specs/003-/research.md`

## Phase 1: Design & Contracts

**Status**: ✅ 已完成

**设计产出**:

1. **数据模型** (`data-model.md`):
   - `CrawlTask`: 任务模型 + 状态机 (6种状态)
   - `WeiboPost`: 帖子模型 + Redis存储结构
   - `CrawlCheckpoint`: 检查点模型 + 恢复逻辑
   - 状态转换图和验证规则

2. **API契约** (`contracts/`):
   - `create_crawl_task.md`: 创建任务契约
   - `start_crawl.md`: 启动爬取契约 (含事件推送)
   - `pause_crawl.md`: 暂停任务契约
   - `get_crawl_progress.md`: 查询进度契约
   - `export_crawl_data.md`: 导出数据契约
   - `list_crawl_tasks.md`: 列出任务契约

3. **测试场景** (`quickstart.md`):
   - 8个端到端场景 (创建→启动→暂停→恢复→完成→增量→导出→异常)
   - 性能验证 (百万级数据)
   - 边界测试清单

4. **契约测试骨架** (Phase 2生成):
   - 每个契约对应一个测试文件
   - 测试用例从契约的"测试要点"生成

**输出文件**:
- `/home/ubuntu/worktrees/desktop/specs/003-/data-model.md`
- `/home/ubuntu/worktrees/desktop/specs/003-/quickstart.md`
- `/home/ubuntu/worktrees/desktop/specs/003-/contracts/*.md` (6个契约)

## Phase 2: Task Planning Approach
*This section describes what the /tasks command will do - DO NOT execute during /plan*

### Task Generation Strategy

/tasks命令将基于Phase 1的设计文档生成完整的任务列表:

#### 1. 从contracts生成契约测试任务 (可并行)

为每个契约生成一个测试任务:
- `tests/contract/test_create_crawl_task.rs` [P]
- `tests/contract/test_start_crawl.rs` [P]
- `tests/contract/test_pause_crawl.rs` [P]
- `tests/contract/test_get_crawl_progress.rs` [P]
- `tests/contract/test_export_crawl_data.rs` [P]
- `tests/contract/test_list_crawl_tasks.rs` [P]

测试内容从contracts/*.md的"测试要点"章节提取。

#### 2. 从data-model生成模型创建任务 (可并行)

为每个模型生成一个文件创建任务:
- `src-tauri/src/models/crawl_task.rs` [P]
  - 包含: CrawlTask结构、CrawlStatus枚举、状态机转换逻辑、验证方法
- `src-tauri/src/models/weibo_post.rs` [P]
  - 包含: WeiboPost结构、序列化/反序列化、验证方法
- `src-tauri/src/models/crawl_checkpoint.rs` [P]
  - 包含: CrawlCheckpoint结构、CrawlDirection枚举、恢复逻辑
- `src-tauri/src/models/crawl_events.rs` [P]
  - 包含: CrawlProgressEvent、CrawlCompletedEvent、CrawlErrorEvent

#### 3. 从quickstart生成集成测试任务 (可并行)

为quickstart中的8个场景生成集成测试:
- `tests/integration/test_scenario_1_create_task.rs` [P]
- `tests/integration/test_scenario_2_start_crawl.rs` [P]
- ... (共8个场景)

#### 4. 按依赖顺序生成实现任务

**第一层: 工具函数** (无依赖)
- Task: 实现 `src-tauri/src/utils/time_utils.rs` [P]
  - `floor_to_hour()`, `ceil_to_hour()`, `parse_weibo_time()`

**第二层: 服务扩展** (依赖Redis连接池)
- Task: 扩展 `src-tauri/src/services/redis_service.rs`
  - 新增方法: `save_crawl_task()`, `load_task()`, `save_checkpoint()`, `load_checkpoint()`, `save_posts()`, `get_posts_by_time_range()`

**第三层: 核心服务** (依赖models + redis_service)
- Task: 实现 `src-tauri/src/services/time_shard_service.rs`
  - `split_time_range_if_needed()` (递归二分算法)
- Task: 实现 `src-tauri/src/services/crawl_service.rs`
  - `start_history_crawl()`, `start_incremental_crawl()`, `cancel_crawl()`, 事件推送

**第四层: Commands** (依赖services)
- Task: 实现 `src-tauri/src/commands/crawl_commands.rs`
  - 6个Tauri commands (create/start/pause/get_progress/export/list)

**第五层: Playwright脚本** (依赖WebSocket协议)
- Task: 实现 `playwright/src/weibo-crawler.ts`
  - WebSocket通信、爬取逻辑、验证码检测

**第六层: 前端组件** (可并行)
- Task: 实现 `src/components/CrawlTaskList.tsx` [P]
- Task: 实现 `src/components/CrawlTaskForm.tsx` [P]
- Task: 实现 `src/components/CrawlProgress.tsx` [P]
- Task: 实现 `src/components/ExportDialog.tsx` [P]

**第七层: 前端页面** (依赖components)
- Task: 实现 `src/pages/CrawlPage.tsx`

#### 5. TDD执行顺序

每个功能模块遵循TDD顺序:
1. 编写契约测试 (红色阶段: 测试失败)
2. 实现最小代码使测试通过 (绿色阶段)
3. 重构代码 (保持测试通过)

### Ordering Strategy

**依赖层级**:
```
L0: [P] time_utils, models (可并行)
L1: redis_service扩展 (依赖models)
L2: [P] time_shard_service, crawl_service (可并行,依赖L1)
L3: crawl_commands (依赖L2)
L4: playwright脚本 (依赖L3的WebSocket协议)
L5: [P] 前端组件 (可并行,依赖L3的API)
L6: 前端页面 (依赖L5)
```

**并行执行标记**:
- [P]: 同一层级内可并行执行的任务
- 示例: L0的4个model文件可由4个agent并行创建

### Estimated Output

**总任务数**: 约35-40个任务

**任务分类**:
- 契约测试: 6个
- 模型创建: 4个
- 集成测试: 8个
- 工具函数: 1个
- 服务扩展/实现: 3个
- Commands: 1个
- Playwright脚本: 1个
- 前端组件: 4个
- 前端页面: 1个
- 单元测试: 5-10个 (根据复杂度生成)

**预估开发时间** (单agent顺序执行):
- Phase 2 (任务生成): 10分钟
- Phase 3 (实现): 20-30小时
- Phase 4 (验证): 2-4小时

**并行执行优化**:
- 如果有4个agent并行,L0+L2+L5可并行,总时间可缩短至10-15小时

### 任务模板示例

```markdown
## Task 001: 创建CrawlTask模型

**Priority**: High
**Parallel**: Yes [P]
**Dependencies**: None

**Description**:
创建 `src-tauri/src/models/crawl_task.rs`,实现CrawlTask结构和状态机。

**Acceptance Criteria**:
- [ ] 定义CrawlTask结构 (包含所有data-model.md中的字段)
- [ ] 定义CrawlStatus枚举 (6种状态)
- [ ] 实现can_transition_to()方法 (状态转换验证)
- [ ] 实现validate()方法 (数据验证)
- [ ] 通过单元测试 (tests/unit/test_crawl_task_model.rs)

**Reference**:
- Data Model: specs/003-/data-model.md (CrawlTask章节)
```

### IMPORTANT

此阶段由 `/tasks` 命令执行,NOT by `/plan`。本plan文档在此停止,准备移交给/tasks命令。

## Phase 3+: Future Implementation
*These phases are beyond the scope of the /plan command*

**Phase 3**: Task execution (/tasks command creates tasks.md)  
**Phase 4**: Implementation (execute tasks.md following constitutional principles)  
**Phase 5**: Validation (run tests, execute quickstart.md, performance validation)

## Complexity Tracking
*Fill ONLY if Constitution Check has violations that must be justified*

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| [e.g., 4th project] | [current need] | [why 3 projects insufficient] |
| [e.g., Repository pattern] | [specific problem] | [why direct DB access insufficient] |


## Progress Tracking
*This checklist is updated during execution flow*

**Phase Status**:
- [x] Phase 0: Research complete (/plan command) ✅
- [x] Phase 1: Design complete (/plan command) ✅
- [x] Phase 2: Task planning complete (/plan command - describe approach only) ✅
- [ ] Phase 3: Tasks generated (/tasks command) - 待执行
- [ ] Phase 4: Implementation complete - 待执行
- [ ] Phase 5: Validation passed - 待执行

**Gate Status**:
- [x] Initial Constitution Check: PASS (所有5个原则均满足,无违规项)
- [x] Post-Design Constitution Check: PASS (设计后重新评估,仍然满足所有原则)
- [x] All NEEDS CLARIFICATION resolved (research.md中全部解决)
- [x] Complexity deviations documented (无违规项,无需文档化)

**产出清单**:
- ✅ research.md: 技术方案研究,解决所有NEEDS CLARIFICATION
- ✅ data-model.md: 3个核心模型设计 + 状态机 + Redis存储结构
- ✅ contracts/: 6个Tauri命令契约,覆盖所有功能需求
- ✅ quickstart.md: 8个端到端验收测试场景
- ✅ plan.md (本文件): 完整实施计划

---
*Based on Constitution v2.1.1 - See `/memory/constitution.md`*
