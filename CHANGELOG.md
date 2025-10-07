# 更新日志

本项目的所有重要变更都将记录在此文件中。

格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.0.0/),
版本号遵循 [语义化版本](https://semver.org/lang/zh-CN/)。

## [Unreleased]

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
