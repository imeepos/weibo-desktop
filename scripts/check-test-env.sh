#!/bin/bash

###############################################################################
# E2E 测试环境检查脚本
#
# 验证运行 Playwright 测试所需的所有依赖
###############################################################################

set +e  # 允许命令失败,继续检查

echo "🔍 E2E 测试环境检查"
echo "===================="
echo ""

ERRORS=0
WARNINGS=0

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

check_pass() {
  echo -e "${GREEN}✓${NC} $1"
}

check_fail() {
  echo -e "${RED}✗${NC} $1"
  ERRORS=$((ERRORS + 1))
}

check_warn() {
  echo -e "${YELLOW}⚠${NC} $1"
  WARNINGS=$((WARNINGS + 1))
}

# 1. Node.js 检查
echo "📦 检查 Node.js..."
if command -v node &>/dev/null; then
  NODE_VERSION=$(node --version)
  check_pass "Node.js 已安装: $NODE_VERSION"

  # 检查版本是否 >= 20
  NODE_MAJOR=$(echo "$NODE_VERSION" | cut -d'.' -f1 | sed 's/v//')
  if [ "$NODE_MAJOR" -ge 20 ]; then
    check_pass "Node.js 版本满足要求 (>= 20)"
  else
    check_warn "Node.js 版本偏低 (推荐 >= 20)"
  fi
else
  check_fail "Node.js 未安装"
fi
echo ""

# 2. pnpm 检查
echo "📦 检查 pnpm..."
if command -v pnpm &>/dev/null; then
  PNPM_VERSION=$(pnpm --version)
  check_pass "pnpm 已安装: $PNPM_VERSION"
else
  check_fail "pnpm 未安装 (运行: npm install -g pnpm)"
fi
echo ""

# 3. Redis 检查
echo "🗄️  检查 Redis..."

# 尝试 localhost
if timeout 2 bash -c "</dev/tcp/localhost/6379" 2>/dev/null; then
  check_pass "Redis 可访问 (localhost:6379)"
elif timeout 2 bash -c "</dev/tcp/redis/6379" 2>/dev/null; then
  check_pass "Redis 可访问 (redis:6379 Docker 网络)"
else
  check_fail "Redis 不可访问"
  echo "   提示: docker compose up redis -d"
fi
echo ""

# 4. Playwright 检查
echo "🎭 检查 Playwright..."
if [ -f "node_modules/.bin/playwright" ]; then
  check_pass "Playwright 已安装"

  # 检查 Chromium
  if [ -d "$HOME/.cache/ms-playwright/chromium-"* ] 2>/dev/null; then
    check_pass "Chromium 浏览器已安装"
  else
    check_warn "Chromium 浏览器未安装"
    echo "   提示: pnpm exec playwright install chromium --with-deps"
  fi
else
  check_fail "Playwright 未安装"
  echo "   提示: pnpm install"
fi
echo ""

# 5. Rust 检查 (Tauri 需要)
echo "🦀 检查 Rust..."
if command -v rustc &>/dev/null; then
  RUST_VERSION=$(rustc --version)
  check_pass "Rust 已安装: $RUST_VERSION"
else
  check_warn "Rust 未安装 (Tauri 需要)"
  echo "   提示: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
fi
echo ""

# 6. Tauri CLI 检查
echo "🚀 检查 Tauri..."
if [ -f "node_modules/.bin/tauri" ]; then
  check_pass "Tauri CLI 已安装"
else
  check_fail "Tauri CLI 未安装"
  echo "   提示: pnpm install"
fi
echo ""

# 7. 系统库检查 (Playwright Chromium 需要)
echo "🔧 检查系统库..."
MISSING_LIBS=()

check_lib() {
  if ! ldconfig -p | grep -q "$1"; then
    MISSING_LIBS+=("$1")
  fi
}

check_lib "libnss3"
check_lib "libatk-1.0"
check_lib "libcups"
check_lib "libdrm"
check_lib "libgbm"

if [ ${#MISSING_LIBS[@]} -eq 0 ]; then
  check_pass "所有必需的系统库已安装"
else
  check_warn "缺少部分系统库: ${MISSING_LIBS[*]}"
  echo "   提示: pnpm exec playwright install-deps chromium"
fi
echo ""

# 8. 显示环境检查
echo "🖥️  检查显示环境..."
if [ -n "$DISPLAY" ]; then
  check_pass "DISPLAY 已设置: $DISPLAY"
else
  check_pass "无头环境 (适合 Docker)"
fi
echo ""

# 9. 共享内存检查
echo "💾 检查共享内存..."
if [ -d "/dev/shm" ]; then
  SHM_SIZE=$(df -h /dev/shm | tail -1 | awk '{print $2}')
  check_pass "/dev/shm 可用: $SHM_SIZE"

  # 检查是否足够大
  SHM_SIZE_MB=$(df -m /dev/shm | tail -1 | awk '{print $2}')
  if [ "$SHM_SIZE_MB" -lt 64 ]; then
    check_warn "/dev/shm 可能过小 (建议 >= 64MB)"
    echo "   Docker: 在 compose 中设置 shm_size: '2gb'"
  fi
else
  check_warn "/dev/shm 不可用"
fi
echo ""

# 10. 测试文件检查
echo "📄 检查测试文件..."
if [ -f "playwright.config.ts" ]; then
  check_pass "playwright.config.ts 存在"
else
  check_fail "playwright.config.ts 缺失"
fi

if [ -d "e2e" ] && [ "$(ls -A e2e/*.spec.ts 2>/dev/null)" ]; then
  TEST_COUNT=$(ls e2e/*.spec.ts 2>/dev/null | wc -l)
  check_pass "找到 $TEST_COUNT 个测试文件"
else
  check_fail "e2e/ 目录为空或不存在"
fi
echo ""

# 总结
echo "===================="
echo "📊 检查总结"
echo "===================="

if [ $ERRORS -eq 0 ] && [ $WARNINGS -eq 0 ]; then
  echo -e "${GREEN}✓ 环境完美! 可以运行测试${NC}"
  echo ""
  echo "运行测试:"
  echo "  pnpm test:e2e"
  exit 0
elif [ $ERRORS -eq 0 ]; then
  echo -e "${YELLOW}⚠ 发现 $WARNINGS 个警告,但可以运行测试${NC}"
  echo ""
  echo "运行测试:"
  echo "  pnpm test:e2e"
  exit 0
else
  echo -e "${RED}✗ 发现 $ERRORS 个错误, $WARNINGS 个警告${NC}"
  echo ""
  echo "修复建议:"
  echo "  1. pnpm install"
  echo "  2. pnpm exec playwright install chromium --with-deps"
  echo "  3. docker compose up redis -d"
  exit 1
fi
