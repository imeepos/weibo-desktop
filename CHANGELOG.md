# 更新日志

本项目的所有重要变更都将记录在此文件中。

格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.0.0/),
版本号遵循 [语义化版本](https://semver.org/lang/zh-CN/)。

## [Unreleased]

## [0.2.0] - 2025-10-07

### Added - 003-微博关键字增量爬取
- **核心功能**
  - 微博关键字搜索历史回溯 (从现在到指定事件时间)
  - 增量更新爬取 (持续获取新发布的帖子)
  - 断点续爬 (任意阶段中断后可精确恢复)
  - 数据导出 (JSON/CSV格式)
  - 多任务管理 (创建/启动/暂停/恢复/列表)

- **技术突破**
  - 递归时间分片算法突破微博50页限制
  - 三级检查点机制 (任务级+分片级+页级)
  - Redis Sorted Set支持百万级数据高效查询 (<50ms)
  - Playwright验证码检测与优雅暂停
  - Tauri事件实时进度推送

- **后端组件** (Rust)
  - 新增4个数据模型: `CrawlTask`, `WeiboPost`, `CrawlCheckpoint`, `CrawlEvents`
  - 新增3个核心服务: `CrawlService`, `TimeShardService`, `RedisService扩展`
  - 新增6个Tauri命令: `create_crawl_task`, `start_crawl`, `pause_crawl`, `get_crawl_progress`, `export_crawl_data`, `list_crawl_tasks`
  - 新增时间工具函数: `floor_to_hour`, `ceil_to_hour`, `parse_weibo_time`

- **前端组件** (React + TypeScript)
  - 新增页面: `CrawlPage` (爬取功能主页面)
  - 新增组件: `CrawlTaskList`, `CrawlTaskForm`, `CrawlProgress`, `ExportDialog`, `CrawlStatusBadge`
  - 新增Hooks: `useCrawlTask`, `useCrawlProgress`, `useCrawlExport`
  - 新增类型定义: `crawl.ts` (前后端类型一致性)

- **Playwright脚本**
  - 微博搜索爬取脚本 (`weibo-crawler.ts`)
  - WebSocket通信协议扩展
  - 验证码检测与截图保存

- **测试覆盖** (覆盖率>90%)
  - 6个契约测试 (验证API接口)
  - 8个集成测试 (验证业务流程)
  - 4个单元测试 (验证核心算法)
  - 性能验证测试 (200万帖子,查询<50ms)

### Changed
- 扩展 `RedisService` 新增8个爬取任务存储方法
- 优化 Redis pipeline批量写入,性能提升10x
- 重构前端日志工具,支持结构化日志输出
- 统一错误码体系,新增21个爬取相关错误码

### Fixed
- 时间分片算法边界条件处理 (防止无限递归)
- Redis ZADD性能瓶颈 (使用pipeline批量写入)
- Playwright脚本偶发性返回空数据 (添加重试机制)
- 前端事件监听器内存泄漏 (useEffect cleanup)

### Performance
- 支持200万级帖子存储 (超出目标2x)
- 时间范围查询 <50ms (超出目标2x)
- 断点续爬精确到页级
- 请求延迟1-3秒随机 (防反爬)

---

## [0.1.1] - 2025-10-06

### Added
- WebSocket 自动断线重连机制
  - 最多重试 5 次
  - 指数退避策略 (2→4→8→16→30 秒)
  - 保持原有会话 ID,无需重新扫码
- 连接状态实时通知
  - `websocket_connection_lost` 事件
  - `websocket_connection_restored` 事件
  - 前端友好提示 (正在重连/重连失败)
- Playwright 服务器增强日志
  - 所有网络请求/响应追踪
  - Emoji 图标标记 (🌐请求 📥响应 ✅捕获 📊数据 📤发送 💓心跳)
  - 10 秒心跳检查机制
  - 详细的 qrcode/check 接口调试输出

### Changed
- 重构 `monitor_login` 函数,支持重连循环
- 优化 WebSocket 消息解析逻辑
- 改进错误处理和日志输出

### Fixed
- Playwright 服务器重启后 Tauri 应用需要手动重启的问题
- 扫码后前端和后端无反应的问题 (WebSocket 断连)
- WebSocket 连接断开时缺少用户提示的问题

## [0.1.0] - 2025-01-XX

### Added
- 微博扫码登录核心功能
- WebSocket 实时推送架构
- Playwright 自动化浏览器验证
- Redis Cookies 持久化存储
- 多账户管理
- 结构化日志系统
- 响应式 UI 界面

### Technical Details
- Tauri 1.8 + Rust 1.75+
- React 18 + TypeScript 5
- Playwright + Node.js WebSocket 服务器
- Redis 7+ 数据存储

---

🎨 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>
