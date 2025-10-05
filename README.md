# 微博扫码登录 Cookies 获取工具

一个基于 Tauri 的桌面应用,用于通过扫码登录获取微博 Cookies 并保存到 Redis。

## 功能特性

- ✅ 微博扫码登录 (二维码生成 + 轮询)
- ✅ Cookies 自动验证 (通过 Playwright 调用微博 API)
- ✅ Redis 持久化存储 (30天过期)
- ✅ 多账户管理 (支持保存多个微博账户)
- ✅ 结构化日志 (JSON 格式,按天轮转)
- ✅ 响应式 UI (支持桌面和移动端)

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
cd src-tauri && cargo build
cd ../playwright && pnpm install && pnpm run build
```

### 2. 配置环境变量

创建 `.env` 文件:
```env
REDIS_URL=redis://localhost:6379
WEIBO_APP_KEY=your_app_key
PLAYWRIGHT_SCRIPT_PATH=./playwright/dist/validate-cookies.js
```

### 3. 启动 Redis

```bash
docker run -d -p 6379:6379 --name weibo-redis redis:7-alpine
```

### 4. 运行应用

```bash
pnpm tauri dev
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

本项目遵循 **代码艺术家宪章** (`.specify/memory/constitution.md`),体现五大核心原则:

1. **存在即合理** - 每个组件都有不可替代的存在理由
2. **优雅即简约** - 代码自我阐述,无冗余注释
3. **性能即艺术** - 异步操作,连接池,优化构建
4. **错误处理哲学** - 结构化错误,用户友好提示
5. **日志表达思想** - 结构化日志,讲述系统故事

## 性能指标

- 二维码生成: < 500ms
- 轮询延迟: < 1s
- Cookies验证: < 2s
- Redis操作: < 100ms

## 许可证

MIT

## 贡献

欢迎提交 Issue 和 Pull Request!

🎨 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>
