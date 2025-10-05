# E2E 测试命令速查表

快速参考所有 Playwright E2E 测试相关命令。

## 🚀 快速开始

```bash
# 一键检查环境
./scripts/check-test-env.sh

# 运行所有测试
pnpm test:e2e

# 查看测试报告
pnpm test:report
```

## 📦 安装与设置

### 首次安装

```bash
# 安装项目依赖
pnpm install

# 安装 Playwright 浏览器
pnpm exec playwright install chromium

# 安装系统依赖 (Ubuntu/Debian)
pnpm exec playwright install-deps chromium
```

### Docker 环境

```bash
# 使用官方镜像
docker pull mcr.microsoft.com/playwright:v1.40.0-jammy

# 使用项目 Dockerfile
docker build -f Dockerfile.playwright -t tauri-e2e-tests .
```

## 🧪 运行测试

### 基础命令

```bash
# 标准无头测试
pnpm test:e2e

# 等价命令
pnpm exec playwright test
./scripts/run-e2e-tests.sh test
```

### 交互模式

```bash
# UI 模式 (推荐)
pnpm test:e2e:ui

# 有头模式 (显示浏览器)
pnpm test:e2e:headed

# 调试模式 (逐步执行)
pnpm test:e2e:debug
```

### 选择性运行

```bash
# 运行单个文件
pnpm exec playwright test login.spec.ts

# 运行单个测试
pnpm exec playwright test -g "应该正确显示初始页面元素"

# 运行特定 describe 块
pnpm exec playwright test -g "微博扫码登录界面"

# 跳过某些测试
pnpm exec playwright test --grep-invert @slow
```

### 并行与重试

```bash
# 单线程运行 (默认)
pnpm exec playwright test --workers=1

# 重试失败测试
pnpm exec playwright test --retries=2

# 仅运行失败的测试
pnpm exec playwright test --last-failed
```

## 📊 报告与调试

### 查看报告

```bash
# HTML 报告 (推荐)
pnpm test:report

# 等价命令
pnpm exec playwright show-report

# JSON 报告
cat playwright-report/results.json | jq
```

### 查看追踪

```bash
# 查看失败测试的追踪
pnpm exec playwright show-trace test-results/[测试名称]/trace.zip

# 示例
pnpm exec playwright show-trace test-results/login-spec-ts-初始页面元素/trace.zip
```

### 截图管理

```bash
# 更新基准截图
pnpm exec playwright test --update-snapshots

# 仅更新失败的截图
pnpm exec playwright test --update-snapshots --grep "初始页面截图对比"

# 查看截图差异
ls test-results/**/*-diff.png
```

## 🛠️ 环境管理

### Redis 服务

```bash
# 启动 Redis
docker compose up redis -d

# 停止 Redis
docker compose down redis

# 检查 Redis 连接
redis-cli ping

# 查看 Redis 数据
redis-cli
> KEYS weibo:cookies:*
> HGETALL weibo:cookies:123456789
```

### Tauri 应用

```bash
# 手动启动 Tauri (推荐)
pnpm tauri dev

# 后台启动
nohup pnpm tauri dev > tauri.log 2>&1 &

# 检查端口
curl http://localhost:1420

# 查看日志
tail -f tauri.log
```

### 清理环境

```bash
# 删除测试产物
rm -rf test-results/ playwright-report/

# 删除 node_modules 重新安装
rm -rf node_modules && pnpm install

# 清理 Playwright 缓存
rm -rf ~/.cache/ms-playwright
pnpm exec playwright install chromium
```

## 🐳 Docker 命令

### Docker Compose

```bash
# 运行测试容器
docker compose -f .playwright-docker.yml up playwright-tests

# 后台运行
docker compose -f .playwright-docker.yml up -d playwright-tests

# 查看日志
docker compose -f .playwright-docker.yml logs -f playwright-tests

# 清理
docker compose -f .playwright-docker.yml down
```

### 独立容器

```bash
# 构建镜像
docker build -f Dockerfile.playwright -t tauri-e2e-tests .

# 运行测试
docker run --rm \
  --network host \
  -e REDIS_URL=redis://localhost:6379 \
  tauri-e2e-tests

# 交互式运行
docker run --rm -it \
  --network host \
  -v $(pwd):/workspace/desktop \
  tauri-e2e-tests \
  bash
```

## 🔍 诊断命令

### 环境检查

```bash
# 完整环境检查
./scripts/check-test-env.sh

# 检查 Node.js
node --version  # 应该 >= 20

# 检查 pnpm
pnpm --version

# 检查 Playwright
pnpm exec playwright --version

# 检查 Rust
rustc --version

# 检查 Tauri
pnpm exec tauri --version
```

### 网络诊断

```bash
# 检查 Tauri 端口
netstat -tlnp | grep 1420
lsof -i :1420

# 检查 Redis 端口
netstat -tlnp | grep 6379
lsof -i :6379

# 测试 HTTP 连接
curl -v http://localhost:1420

# 测试 Redis 连接
timeout 2 bash -c "</dev/tcp/localhost/6379" && echo "Redis OK"
```

### 日志查看

```bash
# Playwright 调试日志
DEBUG=pw:api pnpm test:e2e

# 详细日志
DEBUG=* pnpm test:e2e

# 浏览器控制台日志
pnpm exec playwright test --browser-console-logs
```

## 📝 配置文件编辑

```bash
# 编辑 Playwright 配置
vim playwright.config.ts

# 编辑环境变量
vim .env.test

# 编辑测试用例
vim e2e/login.spec.ts

# 编辑 package.json 脚本
vim package.json
```

## 🎯 常用组合

### 开发流程

```bash
# 1. 检查环境
./scripts/check-test-env.sh

# 2. 启动服务
docker compose up redis -d
pnpm tauri dev  # 在另一个终端

# 3. 运行测试 (UI 模式)
pnpm test:e2e:ui

# 4. 查看报告
pnpm test:report
```

### 调试失败

```bash
# 1. 重新运行失败的测试
pnpm exec playwright test --last-failed

# 2. 调试模式运行
pnpm test:e2e:debug

# 3. 查看追踪
pnpm exec playwright show-trace test-results/.../trace.zip

# 4. 查看截图
open test-results/**/*-diff.png
```

### CI/CD 流程

```bash
# 1. 安装依赖
pnpm install --frozen-lockfile
pnpm exec playwright install chromium --with-deps

# 2. 启动服务
docker compose up redis -d

# 3. 运行测试
CI=true pnpm test:e2e

# 4. 上传报告
tar -czf playwright-report.tar.gz playwright-report/
```

## 🔧 高级用法

### 自定义配置

```bash
# 使用自定义配置文件
pnpm exec playwright test --config=playwright.custom.config.ts

# 覆盖配置选项
pnpm exec playwright test --timeout=60000 --retries=3

# 设置环境变量
REDIS_URL=redis://custom-host:6379 pnpm test:e2e
```

### 代码生成

```bash
# 生成测试代码 (录制)
pnpm exec playwright codegen http://localhost:1420

# 生成选择器
pnpm exec playwright inspector
```

### 性能分析

```bash
# 追踪性能
pnpm exec playwright test --trace=on

# 查看性能追踪
pnpm exec playwright show-trace trace.zip
```

## 📚 快速参考

### pnpm 脚本

| 命令 | 说明 |
|------|------|
| `pnpm test:e2e` | 运行所有测试 |
| `pnpm test:e2e:ui` | UI 交互模式 |
| `pnpm test:e2e:headed` | 有头模式 |
| `pnpm test:e2e:debug` | 调试模式 |
| `pnpm test:report` | 查看报告 |
| `pnpm test:install` | 安装浏览器 |

### 环境变量

| 变量 | 说明 | 示例 |
|------|------|------|
| `REDIS_URL` | Redis 连接 | `redis://localhost:6379` |
| `VITE_PORT` | Tauri 端口 | `1420` |
| `PLAYWRIGHT_HEADLESS` | 无头模式 | `true` |
| `CI` | CI 环境标记 | `true` |
| `DEBUG` | 调试日志 | `pw:api` |

### 文件路径

| 路径 | 说明 |
|------|------|
| `playwright.config.ts` | 主配置 |
| `e2e/*.spec.ts` | 测试文件 |
| `.env.test` | 环境变量 |
| `test-results/` | 测试结果 |
| `playwright-report/` | HTML 报告 |

---

**提示**: 使用 `pnpm exec playwright --help` 查看完整命令列表
