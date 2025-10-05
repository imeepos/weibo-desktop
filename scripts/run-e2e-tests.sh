#!/bin/bash
set -e

###############################################################################
# Tauri + Playwright E2E 测试运行脚本
#
# 职责:
# 1. 确保 Redis 服务可用
# 2. 安装 Playwright 浏览器 (如需要)
# 3. 启动 Tauri 应用
# 4. 运行 E2E 测试
# 5. 清理资源
#
# 哲学: 自动化应该简化流程,而非增加复杂度
###############################################################################

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

echo "🚀 Tauri E2E 测试启动器"
echo "========================"

# 切换到项目根目录
cd "$PROJECT_ROOT"

# 加载测试环境变量
if [ -f .env.test ]; then
  echo "📋 加载测试环境变量..."
  set -a
  source .env.test
  set +a
fi

# 检查 Redis 连接
echo "🔍 检查 Redis 连接..."
if ! timeout 5 bash -c "</dev/tcp/localhost/6379" 2>/dev/null; then
  echo "⚠️  警告: 本地 Redis (localhost:6379) 不可用"
  echo "   尝试使用 Docker 网络内的 Redis..."

  # 尝试 Docker 内部 redis 主机名
  if ! timeout 5 bash -c "</dev/tcp/redis/6379" 2>/dev/null; then
    echo "❌ 错误: Redis 服务不可用"
    echo "   请启动 Redis 服务: docker compose up redis -d"
    exit 1
  else
    echo "✅ 连接到 Docker 网络 Redis (redis:6379)"
    export REDIS_URL="redis://redis:6379"
  fi
else
  echo "✅ 连接到本地 Redis (localhost:6379)"
  export REDIS_URL="redis://localhost:6379"
fi

# 检查 Playwright 浏览器
echo "🌐 检查 Playwright 浏览器..."
if ! pnpm playwright --version >/dev/null 2>&1; then
  echo "📦 安装 Playwright..."
  pnpm install --frozen-lockfile
fi

# 检查 Chromium 是否已安装
if [ ! -d "$HOME/.cache/ms-playwright/chromium-"* ]; then
  echo "📥 安装 Chromium 浏览器..."
  pnpm exec playwright install chromium --with-deps
else
  echo "✅ Chromium 已安装"
fi

# 运行测试
echo ""
echo "🧪 开始运行 E2E 测试..."
echo "========================"

# 解析命令行参数
TEST_MODE="${1:-test}"

case "$TEST_MODE" in
  test)
    echo "模式: 标准测试 (无头模式)"
    pnpm test:e2e
    ;;

  headed)
    echo "模式: 可视化测试 (有头模式)"
    pnpm test:e2e:headed
    ;;

  debug)
    echo "模式: 调试模式"
    pnpm test:e2e:debug
    ;;

  ui)
    echo "模式: UI 模式 (交互式)"
    pnpm test:e2e:ui
    ;;

  *)
    echo "❌ 未知模式: $TEST_MODE"
    echo "可用模式: test, headed, debug, ui"
    exit 1
    ;;
esac

TEST_EXIT_CODE=$?

# 测试结束
echo ""
echo "========================"
if [ $TEST_EXIT_CODE -eq 0 ]; then
  echo "✅ 测试通过!"
  echo "📊 查看报告: pnpm test:report"
else
  echo "❌ 测试失败 (退出码: $TEST_EXIT_CODE)"
  echo "📊 查看报告: pnpm test:report"
  echo "🔍 调试建议:"
  echo "   - 查看截图: test-results/"
  echo "   - 查看追踪: playwright-report/"
  echo "   - 调试模式: ./scripts/run-e2e-tests.sh debug"
fi

exit $TEST_EXIT_CODE
