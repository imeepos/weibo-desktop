# 部署指南

本文档说明如何部署微博扫码登录应用。

## 环境要求

### 开发环境
- Node.js 20+
- Rust 1.75+
- pnpm 8+
- Docker (用于Redis)

### 运行环境
- Redis 7+ (远程或本地)
- 微博开放平台App Key

## 环境变量配置

创建 `.env` 文件:

```env
# Redis连接URL
REDIS_URL=redis://localhost:6379

# 微博App Key (从微博开放平台获取)
WEIBO_APP_KEY=your_app_key_here

# Playwright脚本路径
PLAYWRIGHT_SCRIPT_PATH=./playwright/dist/validate-cookies.js

# 日志级别
RUST_LOG=info
```

## 安装依赖

### 1. 前端依赖
```bash
pnpm install
```

### 2. Rust依赖
```bash
cd src-tauri
cargo build --release
```

### 3. Playwright依赖
```bash
cd playwright
pnpm install
pnpm run build
```

## 启动Redis

### 使用Docker (推荐)
```bash
docker run -d \
  --name weibo-redis \
  -p 6379:6379 \
  redis:7-alpine
```

### 使用Docker Compose
```bash
# 创建 docker-compose.yml
version: '3.8'
services:
  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"
    volumes:
      - redis-data:/data
volumes:
  redis-data:
```

启动:
```bash
docker-compose up -d
```

## 开发模式

```bash
# 启动开发服务器
pnpm tauri dev
```

这将:
1. 启动Vite开发服务器 (前端热重载)
2. 编译Rust后端
3. 启动Tauri窗口

## 生产构建

```bash
# 构建所有平台
pnpm tauri build

# 仅构建特定平台
pnpm tauri build --target x86_64-pc-windows-msvc  # Windows
pnpm tauri build --target x86_64-apple-darwin     # macOS Intel
pnpm tauri build --target aarch64-apple-darwin    # macOS Apple Silicon
pnpm tauri build --target x86_64-unknown-linux-gnu # Linux
```

构建产物位置:
- Windows: `src-tauri/target/release/bundle/msi/`
- macOS: `src-tauri/target/release/bundle/dmg/`
- Linux: `src-tauri/target/release/bundle/appimage/`

## 生产环境注意事项

### 1. Redis安全
```bash
# 使用密码保护
REDIS_URL=redis://:password@remote-host:6379

# 使用TLS
REDIS_URL=rediss://remote-host:6379
```

### 2. 日志管理
- 日志文件位置: `logs/weibo-login.log`
- 自动按天轮转
- 保留30天

### 3. Playwright依赖
在WSL2或无头服务器上需要安装系统依赖:
```bash
# Debian/Ubuntu
sudo apt-get install -y \
  libnss3 libnspr4 libatk1.0-0 libatk-bridge2.0-0 \
  libcups2 libdrm2 libdbus-1-3 libxkbcommon0 \
  libxcomposite1 libxdamage1 libxfixes3 libxrandr2 \
  libgbm1 libasound2
```

## 故障排查

### Redis连接失败
```bash
# 检查Redis是否运行
docker ps | grep redis

# 测试连接
redis-cli -h localhost -p 6379 ping
```

### Playwright失败
```bash
# 检查Chromium安装
npx playwright install chromium

# 测试脚本
node playwright/dist/validate-cookies.js '{"SUB":"test"}'
```

### 构建失败
```bash
# 清理缓存
cargo clean
rm -rf node_modules
pnpm install
```

## 性能优化

### 1. Redis优化
```bash
# 在docker-compose.yml中添加
redis:
  command: redis-server --maxmemory 256mb --maxmemory-policy allkeys-lru
```

### 2. Rust编译优化
已在 `Cargo.toml` 中配置:
```toml
[profile.release]
opt-level = "z"  # 优化大小
lto = true       # 链接时优化
strip = true     # 移除调试符号
```

## 监控

### 查看日志
```bash
# 实时查看
tail -f logs/weibo-login.log

# 查看错误
grep ERROR logs/weibo-login.log

# 查看最近100条
tail -n 100 logs/weibo-login.log
```

### Redis监控
```bash
# 查看所有keys
redis-cli KEYS "weibo:cookies:*"

# 查看内存使用
redis-cli INFO memory
```
