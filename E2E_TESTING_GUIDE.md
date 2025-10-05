# Tauri + Playwright E2E 测试指南

本指南介绍如何在 Docker Ubuntu 22 无头环境下运行 Tauri 应用的 E2E 测试。

## 目录结构

```
desktop/
├── e2e/                      # E2E 测试目录
│   ├── login.spec.ts        # 登录界面测试
│   ├── accessibility.spec.ts # 可访问性测试
│   └── tsconfig.json        # TypeScript 配置
├── scripts/
│   └── run-e2e-tests.sh     # 测试运行脚本
├── playwright.config.ts     # Playwright 配置
├── .env.test                # 测试环境变量
└── package.json             # 测试脚本定义
```

## 前置条件

### 1. 系统依赖

在 Docker Ubuntu 22 环境中,确保已安装:

```bash
# 系统库 (Playwright Chromium 需要)
apt-get update && apt-get install -y \
  libnss3 \
  libnspr4 \
  libatk1.0-0 \
  libatk-bridge2.0-0 \
  libcups2 \
  libdrm2 \
  libdbus-1-3 \
  libxkbcommon0 \
  libxcomposite1 \
  libxdamage1 \
  libxfixes3 \
  libxrandr2 \
  libgbm1 \
  libpango-1.0-0 \
  libcairo2 \
  libasound2
```

### 2. Redis 服务

测试需要 Redis 服务运行:

```bash
# 使用 Docker Compose 启动
docker compose up redis -d

# 验证连接
redis-cli ping  # 应返回 PONG
```

### 3. Node.js 依赖

```bash
# 安装项目依赖
pnpm install

# 安装 Playwright 浏览器
pnpm exec playwright install chromium --with-deps
```

## 快速开始

### 方式一: 使用测试脚本 (推荐)

```bash
# 运行所有测试 (无头模式)
./scripts/run-e2e-tests.sh test

# 可视化模式 (需要图形界面)
./scripts/run-e2e-tests.sh headed

# UI 交互式模式
./scripts/run-e2e-tests.sh ui

# 调试模式
./scripts/run-e2e-tests.sh debug
```

### 方式二: 直接使用 pnpm

```bash
# 标准无头测试
pnpm test:e2e

# 交互式 UI 模式
pnpm test:e2e:ui

# 有头模式 (需要显示器)
pnpm test:e2e:headed

# 调试模式 (逐步执行)
pnpm test:e2e:debug

# 查看测试报告
pnpm test:report
```

## 测试配置

### Playwright 配置 (playwright.config.ts)

关键配置项:

```typescript
{
  // Docker 无头环境必需
  headless: true,

  // Chromium 启动参数
  launchOptions: {
    args: [
      '--no-sandbox',              // Docker 必需
      '--disable-setuid-sandbox',  // Docker 必需
      '--disable-dev-shm-usage',   // 避免共享内存问题
      '--disable-gpu',             // 无头环境禁用 GPU
    ],
  },

  // 等待 Tauri 应用启动
  webServer: {
    command: 'pnpm tauri dev',
    url: 'http://localhost:1420',
    timeout: 120000, // 2 分钟 (Rust 编译较慢)
  },
}
```

### 环境变量 (.env.test)

```bash
# Redis 配置
REDIS_URL=redis://redis:6379  # Docker 网络内部

# Tauri 端口
VITE_PORT=1420

# Playwright 配置
PLAYWRIGHT_HEADLESS=true
```

## 测试用例

### 登录界面测试 (e2e/login.spec.ts)

验证内容:
- ✅ 初始页面元素渲染
- ✅ 二维码生成流程
- ✅ 状态变化反馈
- ✅ 倒计时显示
- ✅ 错误处理
- ✅ 视觉回归

### 可访问性测试 (e2e/accessibility.spec.ts)

验证内容:
- ✅ WCAG 2.1 标准合规性
- ✅ 键盘导航支持
- ✅ 屏幕阅读器兼容
- ✅ Alt 文本正确性

## Docker 环境特殊配置

### 1. 无头模式强制启用

Docker 容器内没有图形界面,必须使用 headless 模式:

```typescript
// playwright.config.ts
use: {
  headless: true, // 强制无头
}
```

### 2. 浏览器沙箱禁用

Docker 容器需要禁用沙箱:

```typescript
launchOptions: {
  args: ['--no-sandbox', '--disable-setuid-sandbox'],
}
```

### 3. 共享内存限制

避免 `/dev/shm` 空间不足:

```typescript
launchOptions: {
  args: ['--disable-dev-shm-usage'],
}
```

### 4. Redis 连接处理

优先使用 Docker 内部网络:

```bash
# 容器内访问
REDIS_URL=redis://redis:6379

# 容器外访问
REDIS_URL=redis://localhost:6379
```

## 测试输出

### 成功运行示例

```
🚀 Tauri E2E 测试启动器
========================
📋 加载测试环境变量...
🔍 检查 Redis 连接...
✅ 连接到 Docker 网络 Redis (redis:6379)
🌐 检查 Playwright 浏览器...
✅ Chromium 已安装

🧪 开始运行 E2E 测试...
========================

Running 10 tests using 1 worker

  ✓ 微博扫码登录界面 › 应该正确显示初始页面元素 (1.2s)
  ✓ 微博扫码登录界面 › 点击生成二维码按钮应该触发加载状态 (0.8s)
  ✓ 微博扫码登录界面 › 成功生成二维码后应该显示二维码图片 (2.5s)
  ...

  10 passed (15.3s)

========================
✅ 测试通过!
📊 查看报告: pnpm test:report
```

### 失败处理

测试失败时自动生成:

```
test-results/
├── login-spec-ts-应该正确显示初始页面元素/
│   ├── test-failed-1.png      # 失败截图
│   └── trace.zip              # 追踪文件
└── ...
```

查看失败详情:

```bash
# 查看 HTML 报告
pnpm test:report

# 查看追踪文件
pnpm exec playwright show-trace test-results/.../trace.zip
```

## 调试技巧

### 1. 逐步调试

```bash
pnpm test:e2e:debug
```

会启动 Playwright Inspector,允许:
- 逐行执行测试
- 查看元素选择器
- 实时修改测试代码

### 2. 截图对比

失败时自动截图,对比基准:

```bash
# 更新基准截图
pnpm exec playwright test --update-snapshots
```

### 3. 追踪回放

查看测试执行过程:

```bash
pnpm exec playwright show-trace test-results/.../trace.zip
```

### 4. 本地可视化

如果本地有显示器:

```bash
# 有头模式运行
pnpm test:e2e:headed

# UI 交互式模式
pnpm test:e2e:ui
```

## 常见问题

### Q1: Chromium 启动失败 "Running as root without --no-sandbox"

**解决方案**: 已在配置中添加 `--no-sandbox` 参数。

### Q2: 测试超时 "Timed out waiting for http://localhost:1420"

**原因**: Rust 编译时间较长

**解决方案**:
- 增加 `webServer.timeout` (已设为 120 秒)
- 手动启动 Tauri: `pnpm tauri dev`,然后运行测试

### Q3: Redis 连接失败

**解决方案**:

```bash
# 启动 Redis
docker compose up redis -d

# 验证连接
redis-cli ping

# 检查端口
netstat -tlnp | grep 6379
```

### Q4: "/dev/shm 空间不足" 错误

**解决方案**: 已在配置中添加 `--disable-dev-shm-usage`。

如果仍然失败,扩大 Docker 共享内存:

```yaml
# docker-compose.yml
services:
  app:
    shm_size: '2gb'
```

### Q5: 可访问性测试失败 "Cannot find module @axe-core/playwright"

**解决方案**:

```bash
pnpm install --frozen-lockfile
```

## CI/CD 集成

### GitHub Actions 示例

```yaml
name: E2E Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-22.04

    services:
      redis:
        image: redis:7-alpine
        ports:
          - 6379:6379

    steps:
      - uses: actions/checkout@v3

      - uses: pnpm/action-setup@v2
        with:
          version: 8

      - uses: actions/setup-node@v3
        with:
          node-version: 20
          cache: 'pnpm'

      - name: Install dependencies
        run: pnpm install --frozen-lockfile

      - name: Install Playwright browsers
        run: pnpm exec playwright install chromium --with-deps

      - name: Run E2E tests
        run: pnpm test:e2e
        env:
          CI: true
          REDIS_URL: redis://localhost:6379

      - name: Upload test results
        if: always()
        uses: actions/upload-artifact@v3
        with:
          name: playwright-report
          path: playwright-report/
```

## 性能优化

### 1. 并行执行

单个 Tauri 实例不支持并行,但可以按文件串行:

```typescript
// playwright.config.ts
workers: 1, // 单 worker
fullyParallel: false, // 禁用并行
```

### 2. 复用服务器

本地开发时复用已启动的 Tauri:

```typescript
webServer: {
  reuseExistingServer: !process.env.CI,
}
```

### 3. 选择性测试

运行特定测试:

```bash
# 单个文件
pnpm exec playwright test login.spec.ts

# 单个测试
pnpm exec playwright test -g "应该正确显示初始页面元素"

# 跳过慢测试
pnpm exec playwright test --grep-invert @slow
```

## 最佳实践

1. **测试独立性**: 每个测试独立运行,不依赖其他测试状态
2. **明确等待**: 使用 `waitFor` 而非固定 `sleep`
3. **语义化选择器**: 优先使用文本、角色,而非 CSS 选择器
4. **失败重试**: CI 环境自动重试 2 次
5. **视觉回归**: 关键界面使用截图对比
6. **可访问性**: 每个页面验证 WCAG 合规性

## 参考资源

- [Playwright 官方文档](https://playwright.dev)
- [Tauri 测试指南](https://tauri.app/v1/guides/testing)
- [Axe 可访问性](https://github.com/dequelabs/axe-core)
- [WCAG 2.1 标准](https://www.w3.org/WAI/WCAG21/quickref/)

---

**存在即合理**: 每个测试用例都验证不可或缺的功能
**优雅即简约**: 测试代码清晰表达意图,无冗余断言
**错误是哲学**: 失败时提供足够信息定位问题

Happy Testing! 🎭
