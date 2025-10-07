# Feature Specification: 微博关键字增量爬取

**Feature Branch**: `003-`
**Created**: 2025-10-07
**Status**: Draft
**Input**: User description: "微博关键字增量爬取方案设计"

## Execution Flow (main)
```
1. Parse user description from Input
   → Feature: 微博搜索关键字的历史回溯 + 增量更新爬取
2. Extract key concepts from description
   → Actors: 用户、系统爬虫
   → Actions: 创建任务、回溯历史数据、增量更新、暂停/恢复、导出数据
   → Data: 爬取任务、微博帖子、检查点
   → Constraints: 50页分页限制、时间精度仅到小时、防反爬机制
3. For each unclear aspect:
   → [已标注 NEEDS CLARIFICATION]
4. Fill User Scenarios & Testing section
   → 完成主要场景和边界测试
5. Generate Functional Requirements
   → 所有需求可测试
6. Identify Key Entities
   → 爬取任务、微博帖子、检查点
7. Run Review Checklist
   → 存在部分 NEEDS CLARIFICATION 项
8. Return: SUCCESS (spec ready for planning)
```

---

## ⚡ Quick Guidelines
- ✅ Focus on WHAT users need and WHY
- ❌ Avoid HOW to implement (no tech stack, APIs, code structure)
- 👥 Written for business stakeholders, not developers

---

## User Scenarios & Testing

### Primary User Story
作为数据分析人员,我需要收集微博上特定关键字的历史数据和最新数据,以便进行舆情分析和趋势研究。我希望系统能自动从事件发生时间开始回溯所有相关帖子,并能持续获取新发布的帖子,即使中途中断也能从断点继续。

### Acceptance Scenarios
1. **Given** 用户指定关键字"国庆"和事件开始时间"2025-10-01 00:00", **When** 用户创建爬取任务并启动, **Then** 系统从当前时间开始向后回溯到2025-10-01,收集所有包含该关键字的帖子

2. **Given** 历史回溯已完成的任务, **When** 用户启动增量更新, **Then** 系统仅爬取自上次最大时间后新发布的帖子

3. **Given** 任务正在爬取第30页时用户关闭应用, **When** 用户重新打开应用并恢复任务, **Then** 系统从第31页继续爬取,不会重复已爬取数据

4. **Given** 某时间范围内帖子数量超过50页限制, **When** 系统检测到分页限制, **Then** 系统自动将时间范围拆分成更小的子范围继续爬取

5. **Given** 用户已完成数据爬取, **When** 用户选择导出数据为JSON或CSV格式, **Then** 系统生成包含所有帖子详情的导出文件

### Edge Cases
- **中断恢复**: 任何阶段中断(网络断开、程序崩溃、用户主动暂停)后都能准确恢复
- **空结果处理**: 当搜索时间范围内无结果时,系统应正确识别并完成任务
- **反爬虫检测**: 遇到验证码或被封禁时,系统应暂停任务并通知用户,不应持续重试导致账号风险
- **时间边界重叠**: 由于时间只能精确到小时,需处理时间边界数据的去重
- **超大数据量**: 热门关键字可能产生数百万条数据,系统需保证存储和查询性能
- **任务状态冲突**: 多个客户端同时操作同一任务时的状态同步

## Requirements

### Functional Requirements

#### 任务管理
- **FR-001**: 系统必须允许用户创建爬取任务,指定关键字和事件开始时间
- **FR-002**: 系统必须为每个任务生成唯一标识符
- **FR-003**: 系统必须记录任务的状态(已创建、历史回溯中、历史完成、增量更新中、已暂停、失败)
- **FR-004**: 用户必须能够启动、暂停和恢复任务
- **FR-005**: 系统必须显示所有任务的列表及其状态

#### 历史回溯
- **FR-006**: 系统必须从当前时间开始向后回溯到用户指定的事件开始时间
- **FR-007**: 系统必须记录已爬取的最小帖子时间(向下取整到小时)
- **FR-008**: 当搜索结果为空或达到事件开始时间时,系统必须标记历史回溯完成
- **FR-009**: 系统必须自动处理微博搜索的50页分页限制,通过时间分片策略突破限制

#### 增量更新
- **FR-010**: 历史回溯完成后,系统必须支持增量爬取新发布的帖子
- **FR-011**: 系统必须记录已爬取的最大帖子时间
- **FR-012**: 增量爬取时,系统必须仅获取最大时间后的新帖子

#### 断点续爬
- **FR-013**: 系统必须在每页爬取后保存检查点(当前时间范围、当前页码、爬取方向)
- **FR-014**: 任务恢复时,系统必须从最后一个检查点继续,不重复已爬取数据
- **FR-015**: 系统必须通过帖子ID去重,确保同一帖子不会重复保存

#### 数据采集
- **FR-016**: 系统必须提取每条微博的以下信息:帖子ID、内容、发布时间、作者UID、作者昵称、转发数、评论数、点赞数
- **FR-017**: 系统必须记录每条帖子的爬取时间
- **FR-018**: 系统必须使用真实登录的cookies进行爬取 [NEEDS CLARIFICATION: cookies从何处获取?是否复用001-cookies功能?]

#### 进度追踪
- **FR-019**: 系统必须实时显示任务进度:已爬取帖子数、当前时间范围、当前页码、任务状态
- **FR-020**: 系统必须记录任务的创建时间和最后更新时间

#### 数据导出
- **FR-021**: 用户必须能够将爬取数据导出为JSON或CSV格式
- **FR-022**: 导出文件必须包含所有帖子的完整信息
- **FR-023**: 系统必须返回导出文件的保存路径

#### 错误处理
- **FR-024**: 系统必须检测反爬虫机制(验证码),并暂停任务通知用户
- **FR-025**: 网络请求失败时,系统必须重试最多3次 [NEEDS CLARIFICATION: 重试间隔是多少?是否可配置?]
- **FR-026**: 任务失败时,系统必须记录失败原因

#### 性能和限制
- **FR-027**: 每次请求间隔必须设置延迟以避免反爬检测 [NEEDS CLARIFICATION: 延迟时长是固定1-3秒还是可配置?]
- **FR-028**: 系统必须支持存储至少百万级别的帖子数据
- **FR-029**: 帖子查询必须支持按时间范围快速筛选

### Key Entities

- **爬取任务 (CrawlTask)**: 代表一次关键字爬取任务,包含任务ID、搜索关键字、事件开始时间、任务状态、已爬取时间范围(最小/最大帖子时间)、已爬取帖子总数、创建时间、更新时间

- **微博帖子 (WeiboPost)**: 代表一条微博帖子,包含帖子ID、所属任务ID、帖子内容、发布时间、作者信息(UID/昵称)、互动数据(转发数/评论数/点赞数)、爬取时间

- **检查点 (Checkpoint)**: 代表任务的断点续爬信息,包含任务ID、当前爬取时间范围(起始/结束时间)、当前页码、爬取方向(向后回溯/向前更新)、检查点保存时间

---

## Review & Acceptance Checklist

### Content Quality
- [x] No implementation details (languages, frameworks, APIs)
- [x] Focused on user value and business needs
- [x] Written for non-technical stakeholders
- [x] All mandatory sections completed

### Requirement Completeness
- [ ] No [NEEDS CLARIFICATION] markers remain (存在3处需澄清)
- [x] Requirements are testable and unambiguous
- [x] Success criteria are measurable
- [x] Scope is clearly bounded
- [ ] Dependencies and assumptions identified (需确认与001-cookies的依赖关系)

**待澄清问题**:
1. FR-018: 爬取cookies从何处获取?是否复用001-cookies功能的已保存cookies?
2. FR-025: 网络请求重试间隔是多少?是否需要可配置?
3. FR-027: 请求延迟时长是固定1-3秒还是需要可配置?支持随机延迟吗?
4. 依赖关系: 此功能是否依赖001-cookies功能提供的登录态?
5. 并发策略: 是否支持多任务并行爬取?还是单任务顺序执行? [NEEDS CLARIFICATION]

---

## Execution Status

- [x] User description parsed
- [x] Key concepts extracted
- [x] Ambiguities marked
- [x] User scenarios defined
- [x] Requirements generated
- [x] Entities identified
- [ ] Review checklist passed (部分检查项未通过,需澄清)

---
