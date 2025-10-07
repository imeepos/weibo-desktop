#!/bin/bash
# 简单的Redis连接测试脚本

set -e

echo "=========================================="
echo "Redis 远程连接测试"
echo "=========================================="

# 从.env文件读取配置
source .env

echo "配置信息:"
echo "  REDIS_HOST: $REDIS_HOST"
echo "  REDIS_PORT: $REDIS_PORT"
echo "  REDIS_PASSWORD: ${REDIS_PASSWORD:0:4}****"
echo ""

# 测试1: 使用docker容器测试连接
echo "[测试1] 通过Docker容器测试Redis连接..."
if docker exec desktop-redis redis-cli -h "$REDIS_HOST" -p "$REDIS_PORT" -a "$REDIS_PASSWORD" PING > /dev/null 2>&1; then
    echo "✓ Redis连接成功 (PONG)"
else
    echo "✗ Redis连接失败"
    exit 1
fi

# 测试2: 测试SET/GET操作
echo ""
echo "[测试2] 测试 SET/GET 操作..."
TEST_KEY="test:config:$(date +%s)"
TEST_VALUE="远程Redis配置测试成功"

docker exec desktop-redis redis-cli -h "$REDIS_HOST" -p "$REDIS_PORT" -a "$REDIS_PASSWORD" SET "$TEST_KEY" "$TEST_VALUE" > /dev/null 2>&1
RESULT=$(docker exec desktop-redis redis-cli -h "$REDIS_HOST" -p "$REDIS_PORT" -a "$REDIS_PASSWORD" GET "$TEST_KEY" 2>/dev/null)
docker exec desktop-redis redis-cli -h "$REDIS_HOST" -p "$REDIS_PORT" -a "$REDIS_PASSWORD" DEL "$TEST_KEY" > /dev/null 2>&1

if [ "$RESULT" = "$TEST_VALUE" ]; then
    echo "✓ SET/GET 操作正常"
else
    echo "✗ SET/GET 操作失败"
    exit 1
fi

# 测试3: 测试INFO命令
echo ""
echo "[测试3] 获取Redis服务器信息..."
docker exec desktop-redis redis-cli -h "$REDIS_HOST" -p "$REDIS_PORT" -a "$REDIS_PASSWORD" INFO server 2>&1 | grep "redis_version" | head -1

echo ""
echo "=========================================="
echo "✓ 所有测试通过！"
echo "Redis远程服务器 ($REDIS_HOST:$REDIS_PORT) 配置正确"
echo "=========================================="
