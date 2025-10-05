# E2E 测试快速开始指南

为 Tauri 应用配置 Playwright E2E 测试,适用于 Docker Ubuntu 22 无头环境。

## 🚀 快速开始 (3 步)

### 1. 安装依赖

```bash
# 安装项目依赖 (包括 Playwright)
pnpm install

# 安装 Playwright Chromium 浏览器
pnpm exec playwright install chromium --with-deps
```

### 2. 启动 Redis

```bash
# 使用 Docker Compose
docker compose up redis -d

# 验证连接
redis-cli ping  # 应返回 PONG
```

### 3. 运行测试

```bash
# 方式一: 使用测试脚本 (推荐)
./scripts/run-e2e-tests.sh test

# 方式二: 直接运行
pnpm test:e2e
```

## 📋 检查环境

运行环境检查脚本,确保所有依赖就绪:

```bash
./scripts/check-test-env.sh
```

输出示例:

```
🔍 E2E 测试环境检查
====================
✓ Node.js 已安装: v20.19.5
✓ pnpm 已安装: 10.18.0
✓ Redis 可访问 (localhost:6379)
✓ Playwright 已安装
✓ Chromium 浏览器已安装
✓ Rust 已安装
✓ Tauri CLI 已安装

📊 检查总结
✓ 环境完美! 可以运行测试
```

## 📁 文件结构

```
desktop/
├── e2e/                          # 测试文件目录
│   ├── login.spec.ts            # 登录界面测试
│   ├── accessibility.spec.ts    # 可访问性测试
│   ├── tsconfig.json            # TypeScript 配置
│   └── README.md                # 测试说明
│
├── scripts/
│   ├── run-e2e-tests.sh         # 测试运行脚本
│   └── check-test-env.sh        # 环境检查脚本
│
├── playwright.config.ts         # Playwright 配置
├── .env.test                    # 测试环境变量
├── E2E_TESTING_GUIDE.md         # 详细测试指南
├── QUICKSTART_E2E_TESTING.md    # 本文件
│
├── Dockerfile.playwright        # Docker 测试镜像
└── .playwright-docker.yml       # Docker Compose 配置
```

## 🧪 测试命令

```bash
# 标准无头测试
pnpm test:e2e

# UI 交互式模式 (本地开发)
pnpm test:e2e:ui

# 有头模式 (需要显示器)
pnpm test:e2e:headed

# 调试模式 (逐步执行)
pnpm test:e2e:debug

# 查看测试报告
pnpm test:report

# 更新截图基准
pnpm exec playwright test --update-snapshots
```

## 🎯 测试内容

### 登录界面测试 (e2e/login.spec.ts)

- ✅ 初始页面元素渲染
- ✅ 二维码生成流程
- ✅ 状态变化和倒计时
- ✅ 会话 ID 显示
- ✅ 错误处理
- ✅ 视觉回归测试

### 可访问性测试 (e2e/accessibility.spec.ts)

- ✅ WCAG 2.1 合规性
- ✅ 键盘导航
- ✅ 屏幕阅读器兼容
- ✅ Alt 文本验证

## 🐳 Docker 环境运行

### 方式一: 使用 Docker Compose

```bash
# 运行测试
docker compose -f .playwright-docker.yml up playwright-tests

# 清理
docker compose -f .playwright-docker.yml down
```

### 方式二: 使用 Dockerfile

```bash
# 构建镜像
docker build -f Dockerfile.playwright -t tauri-e2e-tests .

# 运行测试
docker run --rm \
  --network host \
  -e REDIS_URL=redis://localhost:6379 \
  tauri-e2e-tests
```

## 🔧 配置说明

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

  // 等待 Tauri 应用启动 (Rust 编译可能较慢)
  webServer: {
    command: 'pnpm tauri dev',
    url: 'http://localhost:1420',
    timeout: 120000, // 2 分钟
  },
}
```

### 环境变量 (.env.test)

```bash
# Redis 配置
REDIS_URL=redis://redis:6379  # Docker 网络内部
# 或
REDIS_URL=redis://localhost:6379  # 本地开发

# Tauri 端口
VITE_PORT=1420

# Playwright 配置
PLAYWRIGHT_HEADLESS=true
```

## 🐛 常见问题

### Q: "Running as root without --no-sandbox"

已在配置中添加 `--no-sandbox` 参数,无需额外操作。

### Q: 测试超时 "Timed out waiting for http://localhost:1420"

Rust 编译时间较长,解决方案:

```bash
# 方案一: 手动启动 Tauri (推荐)
pnpm tauri dev

# 在另一个终端运行测试
pnpm test:e2e

# 方案二: 增加超时时间 (已设为 120 秒)
```

### Q: Redis 连接失败

```bash
# 启动 Redis
docker compose up redis -d

# 验证连接
redis-cli ping

# 检查端口
netstat -tlnp | grep 6379
```

### Q: 可访问性测试失败 "Cannot find module @axe-core/playwright"

```bash
# 重新安装依赖
pnpm install --frozen-lockfile
```

### Q: Chromium 启动失败 "Missing dependencies"

```bash
# 安装系统依赖
pnpm exec playwright install-deps chromium
```

## 📊 测试输出

### 成功示例

```
🧪 开始运行 E2E 测试...
========================

Running 10 tests using 1 worker

  ✓ 微博扫码登录界面 › 应该正确显示初始页面元素 (1.2s)
  ✓ 微博扫码登录界面 › 点击生成二维码按钮应该触发加载状态 (0.8s)
  ✓ 微博扫码登录界面 › 成功生成二维码后应该显示二维码图片 (2.5s)
  ✓ 微博扫码登录界面 › 应该显示会话ID信息 (1.1s)
  ✓ 微博扫码登录界面 › 应该显示倒计时 (1.3s)
  ...

  10 passed (15.3s)

========================
✅ 测试通过!
📊 查看报告: pnpm test:report
```

### 失败处理

失败时自动生成调试信息:

```
test-results/
├── login-spec-ts-应该正确显示初始页面元素/
│   ├── test-failed-1.png      # 失败截图
│   └── trace.zip              # 追踪文件
```

查看详情:

```bash
# HTML 报告
pnpm test:report

# 追踪回放
pnpm exec playwright show-trace test-results/.../trace.zip
```

## 🎓 下一步

1. **阅读详细指南**: [E2E_TESTING_GUIDE.md](./E2E_TESTING_GUIDE.md)
2. **查看测试示例**: [e2e/README.md](./e2e/README.md)
3. **编写新测试**: 参考 `e2e/login.spec.ts`
4. **CI/CD 集成**: 参考 E2E_TESTING_GUIDE.md 中的 GitHub Actions 配置

## 📚 参考资源

- [Playwright 官方文档](https://playwright.dev)
- [Tauri 测试指南](https://tauri.app/v1/guides/testing)
- [Axe 可访问性](https://github.com/dequelabs/axe-core)

---

**设计哲学**:
- **存在即合理**: 每个测试验证不可或缺的功能
- **优雅即简约**: 测试代码清晰表达意图
- **错误是哲学**: 失败时提供足够信息定位问题

Happy Testing! 🎭
