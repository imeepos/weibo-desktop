# 🚀 快速开始 - Playwright登录方案

## 一键启动

```bash
# 1. 构建Playwright脚本
cd playwright
pnpm install
pnpm run build
cd ..

# 2. 启动Redis
docker compose up redis -d

# 3. 测试生成二维码
node playwright/dist/weibo-login.js generate

# 4. 启动应用
cd src-tauri
cargo run
```

## 环境变量配置

创建`.env`文件:
```env
REDIS_URL=redis://localhost:6379
PLAYWRIGHT_LOGIN_SCRIPT=./playwright/dist/weibo-login.js
PLAYWRIGHT_VALIDATION_SCRIPT=./playwright/dist/validate-cookies.js
RUST_LOG=info
```

## 核心变更

### ❌ 移除了
- `WEIBO_APP_KEY` - 不再需要微博App Key
- 微博OAuth2 API调用 - 不再依赖不可用的API

### ✅ 新增了
- `playwright/src/weibo-login.ts` - 真实登录页面自动化
- Playwright浏览器会话持久化
- 真实的微博二维码生成

## 工作原理

```
用户请求生成二维码
    ↓
Rust调用Node.js脚本 (weibo-login.js)
    ↓
Playwright启动浏览器访问weibo.com/login
    ↓
提取二维码图片(base64)
    ↓
持久化浏览器会话
    ↓
返回二维码和session_id
    ↓
用户扫码
    ↓
轮询检测登录状态
    ↓
登录成功,提取cookies
```

## 验证安装

```bash
# 检查Playwright浏览器
npx playwright install chromium

# 测试登录脚本
node playwright/dist/weibo-login.js generate

# 应该输出类似:
# {
#   "session_id": "qr_xxx",
#   "qr_image": "base64...",
#   "expires_in": 180
# }
```

## 故障排除

### 问题1: "Cannot find module playwright"
```bash
cd playwright
pnpm install
```

### 问题2: "Browser not installed"
```bash
npx playwright install chromium
```

### 问题3: "Failed to launch browser"
```bash
# WSL2环境可能需要
export DISPLAY=:0
```

### 问题4: Rust编译错误
```bash
cd src-tauri
cargo clean
cargo build
```

## 成功标志

✅ TypeScript编译无错误
✅ Rust编译无错误(只有未使用方法警告)
✅ 可以生成真实的微博二维码
✅ Redis连接正常
✅ 应用启动成功

---

**实施完成**: 2025-10-05 | **分支**: 001-cookies
