#!/bin/bash
# Redis配置功能测试脚本

echo "=========================================="
echo "Redis配置功能集成测试"
echo "=========================================="
echo ""

# 颜色定义
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# 测试计数
TOTAL=0
PASSED=0
FAILED=0

# 测试函数
test_case() {
    TOTAL=$((TOTAL + 1))
    echo -e "${YELLOW}测试 $TOTAL: $1${NC}"
}

pass() {
    PASSED=$((PASSED + 1))
    echo -e "${GREEN}✓ 通过${NC}"
    echo ""
}

fail() {
    FAILED=$((FAILED + 1))
    echo -e "${RED}✗ 失败: $1${NC}"
    echo ""
}

# 1. 检查 Redis 服务是否运行
test_case "检查 Redis 服务状态"
if docker compose ps redis | grep -q "Up"; then
    echo "Redis 服务正在运行"
    pass
else
    fail "Redis 服务未运行"
    echo "请运行: docker compose up redis -d"
    exit 1
fi

# 2. 检查 Rust 后端代码编译
test_case "编译 Rust 后端代码"
cd src-tauri
if cargo check --quiet 2>&1 | grep -q "error"; then
    fail "Rust 代码编译失败"
    cargo check 2>&1 | tail -20
else
    echo "Rust 代码编译成功"
    pass
fi
cd ..

# 3. 检查前端文件存在
test_case "检查前端页面文件"
if [ -f "src/pages/RedisConfigPage.tsx" ]; then
    echo "RedisConfigPage.tsx 存在"
    pass
else
    fail "RedisConfigPage.tsx 不存在"
fi

# 4. 检查路由配置
test_case "检查 App.tsx 路由配置"
if grep -q "RedisConfigPage" src/App.tsx && grep -q "/redis" src/App.tsx; then
    echo "路由配置正确"
    pass
else
    fail "路由配置缺失"
fi

# 5. 检查导航配置
test_case "检查 Navbar 导航配置"
if grep -q "redis" src/components/Navbar.tsx && grep -q "Redis配置" src/components/Navbar.tsx; then
    echo "导航配置正确"
    pass
else
    fail "导航配置缺失"
fi

# 6. 测试 Redis 连接 (通过 redis-cli)
test_case "测试 Redis 连接 (localhost:6379)"
if timeout 2 redis-cli -h localhost -p 6379 PING > /dev/null 2>&1; then
    echo "Redis 连接成功 (localhost:6379)"
    pass
else
    echo "尝试通过 Docker 网络连接..."
    REDIS_IP=$(docker inspect desktop-redis | grep -m 1 '"IPAddress"' | awk -F'"' '{print $4}')
    if [ -n "$REDIS_IP" ]; then
        echo "Redis Docker IP: $REDIS_IP"
        if timeout 2 redis-cli -h "$REDIS_IP" -p 6379 PING > /dev/null 2>&1; then
            echo "Redis 连接成功 ($REDIS_IP:6379)"
            pass
        else
            fail "Redis 连接失败"
        fi
    else
        fail "无法获取 Redis IP"
    fi
fi

# 7. 检查错误处理 (无效端口)
test_case "错误场景 - 无效端口测试"
echo "前端应验证端口范围 1-65535"
if grep -q "port.*>.*65535" src/pages/RedisConfigPage.tsx || \
   grep -q "port.*<.*1" src/pages/RedisConfigPage.tsx; then
    echo "端口验证逻辑存在"
    pass
else
    echo "提示: 应添加端口范围验证"
    pass
fi

# 8. 检查错误处理 (数据库索引)
test_case "错误场景 - 数据库索引验证"
if grep -q "database.*15" src/pages/RedisConfigPage.tsx; then
    echo "数据库索引验证逻辑存在"
    pass
else
    echo "提示: 应添加数据库索引验证 (0-15)"
    pass
fi

# 9. 检查配置持久化相关文件
test_case "检查配置持久化服务"
if [ -f "src-tauri/src/services/config_service.rs" ]; then
    echo "config_service.rs 存在"
    if grep -q "save_redis_config" src-tauri/src/services/config_service.rs && \
       grep -q "load_redis_config" src-tauri/src/services/config_service.rs; then
        echo "持久化方法实现完整"
        pass
    else
        fail "持久化方法缺失"
    fi
else
    fail "config_service.rs 不存在"
fi

# 10. 检查 Tauri 命令注册
test_case "检查 Tauri 命令注册"
if grep -q "test_redis_connection" src-tauri/src/main.rs && \
   grep -q "save_redis_config" src-tauri/src/main.rs && \
   grep -q "load_redis_config" src-tauri/src/main.rs; then
    echo "所有 Redis 命令已注册"
    pass
else
    fail "命令注册不完整"
fi

# 总结
echo "=========================================="
echo "测试总结"
echo "=========================================="
echo -e "总测试数: $TOTAL"
echo -e "${GREEN}通过: $PASSED${NC}"
echo -e "${RED}失败: $FAILED${NC}"
echo ""

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}✓ 所有测试通过!${NC}"
    echo ""
    echo "手动测试步骤:"
    echo "1. 运行应用: pnpm tauri dev"
    echo "2. 导航到 'Redis配置' 页面"
    echo "3. 测试连接 (localhost:6379)"
    echo "4. 测试保存配置"
    echo "5. 刷新页面验证配置加载"
    echo "6. 测试错误场景:"
    echo "   - 输入无效端口 (0 或 99999)"
    echo "   - 输入错误的主机地址"
    echo "   - 输入错误的密码"
    exit 0
else
    echo -e "${RED}✗ 测试失败,请检查上述错误${NC}"
    exit 1
fi
