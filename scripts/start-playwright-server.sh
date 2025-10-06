#!/bin/bash
# 启动Playwright Server

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLAYWRIGHT_DIR="$SCRIPT_DIR/../playwright"
PID_FILE="/tmp/playwright-server.pid"

cd "$PLAYWRIGHT_DIR"

# 检查是否已经在运行
if [ -f "$PID_FILE" ]; then
  PID=$(cat "$PID_FILE")
  if ps -p "$PID" > /dev/null 2>&1; then
    echo "Playwright server已在运行 (PID: $PID)"
    exit 0
  else
    rm -f "$PID_FILE"
  fi
fi

# 构建server
npm run build:server

# 启动server (后台运行)
nohup node dist/weibo-login-server.js > /tmp/playwright-server.log 2>&1 &
SERVER_PID=$!

# 保存PID
echo $SERVER_PID > "$PID_FILE"

# 等待server启动
echo "等待Playwright server启动..."
for i in {1..10}; do
  if curl -s http://localhost:9223 > /dev/null 2>&1; then
    echo "Playwright server已启动 (PID: $SERVER_PID)"
    exit 0
  fi
  sleep 1
done

echo "Playwright server启动超时"
exit 1
