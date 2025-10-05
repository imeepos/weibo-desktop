# E2E 测试架构图

## 系统架构

```
┌─────────────────────────────────────────────────────────────┐
│                     Docker Ubuntu 22 环境                      │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  ┌──────────────┐         ┌──────────────┐                  │
│  │   Playwright │ ◄─────► │   Chromium   │                  │
│  │   Test Runner│         │   (Headless) │                  │
│  └──────┬───────┘         └──────────────┘                  │
│         │                                                     │
│         │ HTTP (localhost:1420)                              │
│         ▼                                                     │
│  ┌──────────────────────────────────────┐                   │
│  │         Tauri Application             │                   │
│  ├──────────────────────────────────────┤                   │
│  │  Frontend (React + Vite)             │                   │
│  │  - LoginPage                          │                   │
│  │  - QrcodeDisplay                      │                   │
│  │  - LoginStatus                        │                   │
│  ├──────────────────────────────────────┤                   │
│  │  Backend (Rust)                       │                   │
│  │  - generate_qrcode                    │                   │
│  │  - poll_login_status                  │                   │
│  │  - save_cookies                       │                   │
│  └──────────────┬───────────────────────┘                   │
│                 │                                             │
│                 │ TCP (redis:6379)                           │
│                 ▼                                             │
│  ┌──────────────────────────────────────┐                   │
│  │         Redis Server                  │                   │
│  │  - Cookies Storage                    │                   │
│  │  - TTL: 30 days                       │                   │
│  └──────────────────────────────────────┘                   │
│                                                               │
└─────────────────────────────────────────────────────────────┘
```

## 测试流程

```
┌─────────────┐
│ 开始测试     │
└──────┬──────┘
       │
       ▼
┌──────────────────────────────────┐
│ 1. 环境检查                       │
│   - Node.js, pnpm                │
│   - Redis 连接                   │
│   - Playwright 浏览器            │
└──────┬──────────────────────────┘
       │
       ▼
┌──────────────────────────────────┐
│ 2. 启动 Tauri 应用               │
│   - pnpm tauri dev               │
│   - 等待 http://localhost:1420   │
│   - 超时: 120 秒                 │
└──────┬──────────────────────────┘
       │
       ▼
┌──────────────────────────────────┐
│ 3. 运行测试用例                   │
│   ┌────────────────────────────┐ │
│   │ login.spec.ts              │ │
│   │  - 初始页面渲染            │ │
│   │  - 二维码生成              │ │
│   │  - 状态变化                │ │
│   │  - 视觉回归                │ │
│   └────────────────────────────┘ │
│   ┌────────────────────────────┐ │
│   │ accessibility.spec.ts      │ │
│   │  - WCAG 合规性             │ │
│   │  - 键盘导航                │ │
│   │  - 屏幕阅读器              │ │
│   └────────────────────────────┘ │
└──────┬──────────────────────────┘
       │
       ▼
┌──────────────────────────────────┐
│ 4. 收集结果                       │
│   - 截图 (失败时)                │
│   - 视频 (失败时)                │
│   - 追踪文件                     │
│   - HTML 报告                    │
└──────┬──────────────────────────┘
       │
       ▼
┌──────────────────────────────────┐
│ 5. 生成报告                       │
│   - playwright-report/index.html │
│   - test-results/                │
└──────┬──────────────────────────┘
       │
       ▼
┌─────────────┐
│ 测试完成     │
└─────────────┘
```

## 文件依赖关系

```
playwright.config.ts
    │
    ├── webServer ───────► pnpm tauri dev
    │                          │
    │                          ├── src-tauri/ (Rust Backend)
    │                          └── src/ (React Frontend)
    │
    ├── projects ────────► Chromium
    │                          │
    │                          └── launchOptions
    │                                 │
    │                                 └── --no-sandbox
    │                                     --disable-setuid-sandbox
    │                                     --disable-dev-shm-usage
    │                                     --disable-gpu
    │
    └── testDir ─────────► e2e/
                               │
                               ├── login.spec.ts
                               │      │
                               │      ├── test('初始页面元素')
                               │      ├── test('二维码生成')
                               │      └── test('视觉回归')
                               │
                               └── accessibility.spec.ts
                                      │
                                      ├── test('WCAG 合规')
                                      └── test('键盘导航')
```

## 数据流

```
[用户操作] ──► [Playwright API]
                    │
                    ▼
            [Chromium Browser]
                    │
                    │ HTTP Request
                    ▼
            [Tauri Frontend]
                    │
                    │ invoke()
                    ▼
            [Tauri Backend Commands]
                    │
                    │ Redis Protocol
                    ▼
            [Redis Database]
```

## 测试用例分类

```
E2E Tests (14 个)
│
├── 功能测试 (10 个) - login.spec.ts
│   ├── UI 渲染
│   │   ├── 初始页面元素
│   │   ├── 二维码图片显示
│   │   └── 响应式布局
│   │
│   ├── 用户交互
│   │   ├── 按钮点击
│   │   ├── 加载状态
│   │   └── 状态切换
│   │
│   ├── 数据显示
│   │   ├── 会话 ID
│   │   ├── 倒计时
│   │   └── 事件消息
│   │
│   └── 回归测试
│       └── 视觉截图对比
│
└── 可访问性测试 (4 个) - accessibility.spec.ts
    ├── 标准合规
    │   ├── 初始页面 WCAG
    │   └── 二维码页面 WCAG
    │
    └── 用户体验
        ├── 键盘导航
        └── Alt 文本
```

## 配置层级

```
项目根目录
│
├── 全局配置
│   ├── playwright.config.ts ─► 主配置
│   ├── .env.test ──────────► 环境变量
│   └── package.json ────────► 测试脚本
│
├── 测试配置
│   └── e2e/
│       ├── tsconfig.json ──► TypeScript 配置
│       └── *.spec.ts ────────► 测试用例
│
└── 运行时配置
    ├── scripts/
    │   ├── run-e2e-tests.sh ─► 运行脚本
    │   └── check-test-env.sh ► 环境检查
    │
    └── Docker
        ├── Dockerfile.playwright ─► 镜像定义
        └── .playwright-docker.yml ► Compose 配置
```

## 错误处理流程

```
测试执行
    │
    ├── 成功 ─────────────► 生成报告
    │                         └── ✓ 所有测试通过
    │
    └── 失败
        │
        ├── 截图 ──────────► test-results/**/*.png
        │
        ├── 视频 ──────────► test-results/**/*.webm
        │
        ├── 追踪 ──────────► test-results/**/*.zip
        │                      │
        │                      └── playwright show-trace
        │
        └── 报告 ──────────► playwright-report/
                               │
                               └── pnpm test:report
```

## CI/CD 集成点

```
GitHub Actions
    │
    ├── Checkout Code
    │
    ├── Setup Environment
    │   ├── Node.js 20
    │   ├── pnpm
    │   └── Redis Service
    │
    ├── Install Dependencies
    │   ├── pnpm install
    │   └── playwright install chromium
    │
    ├── Run Tests ────────► pnpm test:e2e
    │                          │
    │                          └── CI=true (自动重试 2 次)
    │
    └── Upload Artifacts
        ├── playwright-report/
        └── test-results/
```

---

**架构原则**:

1. **分层清晰**: 配置 → 运行 → 测试 → 报告
2. **职责单一**: 每个组件专注一件事
3. **可扩展性**: 易于添加新测试
4. **可调试性**: 丰富的失败信息
5. **自动化**: 最少人工干预

