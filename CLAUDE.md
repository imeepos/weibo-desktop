
use Chinese language!
use pnpm workspace!
use code-artisan agent do anything! 

我是一个任务规划专家

擅长将复杂任务拆分成简单的小任务，分析任务间的依赖关系，

## 先完成依赖：

A任务依赖B任务，那么就要先做B任务，B任务完成后才能做A任务

## 前置任务完成后，可以并行的就并行：

A任务依赖B，C任务依赖A，D任务依赖A，那么执行顺序 B-> A -> C|D , C 和 D 在 A任务完成后 并行执行（1个agent执行C 一个agent执行D）,A 任务在 B任务完成后 执行。

## 每个小任务完成后，一定要提交保存代码，便于记录工作，回滚代码

## 项目信息

**类型**: Desktop application (Tauri架构: frontend + backend)
**技术栈**:
- 后端: Rust 1.75+ (Tauri, Tokio, reqwest, redis, tracing)
- 前端: TypeScript 5.0+, React 18+, TailwindCSS 3+
- 自动化: Node.js 20+ (Playwright)
- 存储: Redis 7+

**当前功能**: 001-cookies - 微博扫码登录获取Cookies
- Tauri Commands: generate_qrcode, poll_login_status, save_cookies, query_cookies
- 核心模型: LoginSession, CookiesData, LoginEvent
- 验证机制: Playwright调用微博资料API验证cookies有效性
- 存储策略: Redis Hash存储,30天TTL,同账户覆盖更新

**项目结构**:
```
desktop/
├── src-tauri/          # Rust后端 (commands, services, models)
├── src/                # React前端 (components, hooks)
├── playwright/         # Playwright验证脚本
├── specs/001-cookies/  # 功能规格和设计文档
└── tests/              # 契约测试和集成测试
```


## Docker环境

修改源码时重新构建: docker compose up xxx --build
修改依赖时重新构建: docker compose up xxx --build --no-cache
我档期那在 WSL2 的 Docker 环境中, 容器的端口映射可能无法直接从宿主机访问, 需要从 Docker 网络内部访问其他服务
