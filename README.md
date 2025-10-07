# 微博扫码登录 Cookies 获取工具

一个基于 Tauri 的桌面应用,用于通过扫码登录获取微博 Cookies 并保存到 Redis。

## 功能特性

- ✅ 微博扫码登录 (WebSocket 实时推送)
- ✅ Cookies 自动验证 (通过 Playwright 调用微博 VIP API)
- ✅ Redis 持久化存储 (30天过期)
- ✅ 多账户管理 (支持保存多个微博账户)
- ✅ 结构化日志 (JSON 格式,按天轮转)
- ✅ 响应式 UI (支持桌面和移动端)
- ✅ **WebSocket 断线重连** (自动重试 5 次,指数退避)
- ✅ **增强调试日志** (详细的网络请求追踪)

## 技术栈

### 后端
- **Tauri 1.5** - 跨平台桌面应用框架
- **Rust 1.75+** - 系统编程语言
- **Tokio** - 异步运行时
- **Redis** - 数据存储
- **Playwright (Node.js)** - Cookies 验证

### 前端
- **React 18** - UI 框架
- **TypeScript 5** - 类型安全
- **TailwindCSS 3** - 样式框架
- **Vite** - 构建工具

## 快速开始

### 1. 环境准备

```bash
# 安装依赖
pnpm install

# 构建 Rust 后端
cd src-tauri && cargo build --release && cd ..

# 构建 Playwright 服务器
cd playwright && pnpm install && pnpm run build && cd ..
```

### 2. 配置环境变量

创建 `.env` 文件:
```env
REDIS_URL=redis://localhost:6379
WEIBO_APP_KEY=your_app_key
PLAYWRIGHT_SCRIPT_PATH=./playwright/dist/validate-cookies.js
```

### 3. 启动服务

```bash
# 启动 Redis (使用 Docker Compose)
docker compose up redis -d

# 启动 Playwright WebSocket 服务器
cd playwright && node dist/weibo-login-server.js &

# 或者使用 pnpm 脚本
pnpm --filter playwright server &
```

### 4. 运行应用

```bash
# 开发模式 (自动热重载)
pnpm tauri dev

# 或分别启动前端和后端
pnpm dev  # 前端 Vite 服务器
cargo tauri dev  # 后端 Rust 应用
```

## 使用流程

1. **生成二维码**: 点击"生成二维码"按钮
2. **扫码登录**: 使用微博 App 扫描二维码
3. **确认登录**: 在手机上点击"确认登录"
4. **自动保存**: Cookies 自动验证并保存到 Redis

## 项目结构

```
desktop/
├── src/                    # React前端
│   ├── components/         # UI组件
│   ├── pages/              # 页面
│   └── types/              # TypeScript类型
├── src-tauri/              # Rust后端
│   ├── src/
│   │   ├── commands/       # Tauri命令
│   │   ├── models/         # 数据模型
│   │   ├── services/       # 业务逻辑
│   │   └── utils/          # 工具函数
│   └── Cargo.toml
├── playwright/             # Playwright验证脚本
│   └── src/
│       └── validate-cookies.ts
└── specs/                  # 设计文档
    └── 001-cookies/
        ├── spec.md         # 功能规格
        ├── plan.md         # 实施计划
        └── contracts/      # API契约
```

## 文档

- [快速开始](./QUICKSTART.md) - 开发环境搭建和测试场景
- [API文档](./API.md) - Tauri命令接口说明
- [部署指南](./DEPLOYMENT.md) - 生产环境部署
- [设计文档](./specs/001-cookies/spec.md) - 功能规格和技术方案

## 开发指南

### 构建

```bash
# 开发模式
pnpm tauri dev

# 生产构建
pnpm tauri build
```

### 测试

```bash
# Rust单元测试
cd src-tauri && cargo test

# Playwright脚本测试
cd playwright && ./test-validate.sh
```

### 日志

日志文件位置: `logs/weibo-login.log`

```bash
# 实时查看
tail -f logs/weibo-login.log

# 查看错误
grep ERROR logs/weibo-login.log
```

## 架构设计

### WebSocket 实时推送架构

本项目使用 **WebSocket** 替代传统轮询,实现真正的实时状态推送:

```
┌─────────────────┐         WebSocket         ┌──────────────────────┐
│  Tauri 后端     │ ◄─────────────────────── │ Playwright 服务器     │
│  (Rust)         │                           │  (Node.js)           │
│                 │                           │                      │
│  - 生成二维码    │   qrcode_generated       │  - 监听微博登录页     │
│  - 监控登录状态  │ ◄────────────────────    │  - 捕获状态变化      │
│  - 保存 Cookies │                           │  - VIP API 验证      │
│                 │   status_update          │                      │
│                 │ ◄────────────────────    │                      │
│                 │   login_confirmed        │                      │
│                 │ ◄────────────────────    │                      │
└─────────────────┘                           └──────────────────────┘
```

**关键特性**:
- ✅ **断线重连**: 自动检测断开,最多重试 5 次 (指数退避: 2→4→8→16→30秒)
- ✅ **状态通知**: 发送 `websocket_connection_lost` 和 `websocket_connection_restored` 事件
- ✅ **会话保持**: 重连后保持原有 `qr_id`,无需重新扫码
- ✅ **增强日志**: 详细的网络请求追踪 (🌐请求 📥响应 ✅捕获 📊数据 📤发送 💓心跳)

### 代码艺术家宪章

本项目遵循 **代码艺术家宪章** (`.specify/memory/constitution.md`),体现五大核心原则:

1. **存在即合理** - 每个组件都有不可替代的存在理由
2. **优雅即简约** - 代码自我阐述,无冗余注释
3. **性能即艺术** - 异步操作,连接池,优化构建
4. **错误处理哲学** - 结构化错误,用户友好提示
5. **日志表达思想** - 结构化日志,讲述系统故事

## 性能指标

- 二维码生成: < 500ms
- WebSocket 延迟: < 100ms (实时推送)
- Cookies验证: < 2s (VIP API)
- Redis操作: < 100ms
- 重连时间: 2-30s (指数退避)

## 故障排除

### Playwright 服务器未启动

**症状**: 前端显示 "Playwright服务器未运行"

**解决方案**:
```bash
# 1. 检查服务器是否运行
curl http://localhost:9223

# 2. 启动 Playwright 服务器
cd playwright && node dist/weibo-login-server.js

# 3. 查看服务器日志
tail -f playwright/logs/server.log
```

### WebSocket 连接断开

**症状**: 扫码后前端无反应,后端日志显示 "WebSocket连接断开"

**解决方案**:
- ✅ **自动重连**: 系统会自动尝试重连 (最多 5 次)
- ✅ **用户提示**: 前端显示 "连接断开,正在自动重连..."
- ❌ **重连失败**: 如果重连失败,点击"刷新二维码"重新开始

**手动干预** (仅开发环境):
```bash
# 重启 Playwright 服务器
pkill -f "weibo-login-server"
cd playwright && node dist/weibo-login-server.js &

# Tauri 应用会自动重连,无需重启
```

### 查看详细日志

```bash
# Playwright 服务器日志 (增强的调试输出)
# 包含: 🌐请求 📥响应 ✅捕获 📊数据 📤发送 💓心跳
tail -f playwright/logs/server.log

# Tauri 后端日志
tail -f logs/weibo-login.log

# 过滤重连相关日志
grep -E "重连|reconnect" logs/weibo-login.log
```

## 许可证

MIT

## 贡献

欢迎提交 Issue 和 Pull Request!

🎨 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>
