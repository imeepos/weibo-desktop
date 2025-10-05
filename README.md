# 微博扫码登录 - Desktop Application

通过微博官方扫码登录API获取网站cookies并存储到Redis的Tauri桌面应用。

## 技术栈

- **前端**: React 18 + TypeScript + TailwindCSS
- **后端**: Rust (Tauri 1.5)
- **自动化**: Playwright (Node.js)
- **存储**: Redis
- **日志**: tracing + tracing-subscriber (JSON格式)

## 项目结构

```
/workspace/desktop/
├── src/                    # React前端源码
│   ├── main.tsx           # 前端入口
│   ├── App.tsx            # 主应用组件
│   └── index.css          # TailwindCSS样式
├── src-tauri/             # Rust后端源码
│   ├── src/
│   │   ├── main.rs        # Tauri入口
│   │   ├── lib.rs         # 模块声明
│   │   ├── commands/      # Tauri Commands (前后端桥梁)
│   │   ├── models/        # 数据模型
│   │   │   └── errors.rs  # 错误类型定义
│   │   ├── services/      # 业务逻辑服务
│   │   └── utils/         # 工具函数
│   │       └── logger.rs  # 结构化日志系统
│   ├── Cargo.toml         # Rust依赖配置
│   └── tauri.conf.json    # Tauri配置
├── playwright/            # Playwright自动化脚本
│   ├── src/
│   │   └── validate-cookies.ts  # Cookies验证脚本
│   └── package.json       # Playwright依赖
├── logs/                  # 日志输出目录
└── package.json           # 前端依赖配置
```

## 开发环境要求

- Rust 1.75+
- Node.js 20+
- pnpm (推荐使用pnpm workspace)
- Redis Server (本地或远程)

## 快速开始

### 1. 安装依赖

```bash
# 安装前端依赖
pnpm install

# 安装Playwright依赖
cd playwright
pnpm install
cd ..

# Rust依赖会在首次构建时自动下载
```

### 2. 配置环境变量

```bash
# 设置日志级别 (可选)
export RUST_LOG=info  # 可选值: trace, debug, info, warn, error

# Redis连接配置 (后续Phase会使用)
export REDIS_URL=redis://localhost:6379
```

### 3. 开发模式

```bash
# 启动Tauri开发服务器
pnpm tauri dev
```

### 4. 构建生产版本

```bash
# 构建前端和Tauri应用
pnpm tauri build
```

## Phase 1 完成状态 ✅

- ✅ Tauri项目目录结构
- ✅ Rust依赖配置 (Cargo.toml)
- ✅ 前端依赖配置 (React + TypeScript + TailwindCSS)
- ✅ 错误类型定义 (errors.rs)
- ✅ 结构化日志系统 (logger.rs)

## 下一步 (Phase 2)

参考 `specs/001-cookies/tasks.md`:

- T006: 实现数据模型 (QrCode, Cookies, ValidationResult)
- T007: 实现Redis服务层
- T008: 实现微博API客户端
- T009: 实现Tauri Commands
- T010: 实现前端UI组件

## 宪章原则

本项目遵循 `.specify/memory/constitution.md` 中定义的五大原则:

1. **存在即合理**: 每个文件、每行代码都有明确目的
2. **优雅即简约**: 代码自我阐述,命名清晰
3. **性能即艺术**: 异步设计,高效的数据结构
4. **错误处理如为人处世的哲学**: 结构化错误,清晰的错误消息
5. **日志是思想的表达**: 结构化JSON日志,讲述系统故事

## 许可证

待定
