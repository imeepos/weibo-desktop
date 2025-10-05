
use Chinese!
use code-artisan agent do anything!
use pnpm workspace!

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

**最近变更**:
- 2025-10-05: 完成微博扫码登录功能规划和设计文档
- Phase 0: 技术研究 (Tauri+Playwright集成, 微博API, Redis连接池)
- Phase 1: 数据模型、契约定义、测试场景设计

---

## Docker环境

修改源码时重新构建: docker compose up xxx --build
修改依赖时重新构建: docker compose up xxx --build --no-cache
我档期那在 WSL2 的 Docker 环境中, 容器的端口映射可能无法直接从宿主机访问, 需要从 Docker 网络内部访问其他服务
