# Quickstart: 微博扫码登录手动测试指南

**Feature**: 001-cookies
**Date**: 2025-10-05
**Purpose**: 验收测试场景,确保功能符合规格要求

---

## 前置条件

### 环境准备
1. **安装依赖**:
   ```bash
   cd /workspace/desktop
   pnpm install
   cd src-tauri && cargo build
   cd ../playwright && pnpm install
   ```

2. **启动Redis** (Docker):
   ```bash
   docker run -d -p 6379:6379 --name weibo-redis redis:7-alpine
   ```

3. **配置应用**:
   创建 `src-tauri/config.json`:
   ```json
   {
     "weibo_client_id": "YOUR_APP_ID",
     "redis_url": "redis://localhost:6379"
   }
   ```

4. **启动应用**:
   ```bash
   pnpm tauri dev
   ```

### 测试工具
- 微博移动端App (iOS或Android)
- Redis客户端 (如 RedisInsight 或 redis-cli)
- 日志查看工具 (tail -f logs/weibo-login.log)

---

## 场景1: 生成二维码

**对应需求**: FR-001

### 操作步骤
1. 打开应用,点击"微博登录"按钮
2. 点击"生成二维码"按钮

### 预期结果
- ✅ 二维码图片在2秒内显示
- ✅ 显示倒计时 (180秒)
- ✅ 二维码清晰可扫描
- ✅ 控制台无错误日志

### 验证命令
```bash
# 查看日志
tail -f logs/weibo-login.log | grep "QR code generated"
```

**日志应包含**:
```json
{
  "level": "INFO",
  "fields": {
    "qr_id": "qr_...",
    "expires_in": 180
  },
  "message": "QR code generated successfully"
}
```

### 异常场景
- **网络断开**: 显示"网络错误,请检查连接",提供重试按钮
- **API限流**: 显示"请求过于频繁,请60秒后重试",按钮禁用60秒

---

## 场景2: 扫描二维码

**对应需求**: FR-002, FR-003

### 操作步骤
1. 完成场景1,生成二维码
2. 使用微博App扫描二维码 (**不要点击确认**)

### 预期结果
- ✅ 应用在3秒内检测到扫码事件
- ✅ 状态变更为"已扫描,请在手机上确认登录"
- ✅ 二维码图片保持显示或半透明

### 验证命令
```bash
# 查看轮询日志
tail -f logs/weibo-login.log | grep "Status changed"
```

**日志应包含**:
```json
{
  "level": "INFO",
  "fields": {
    "qr_id": "qr_...",
    "old_status": "Pending",
    "new_status": "Scanned"
  },
  "message": "Status changed"
}
```

### 前端验证
打开浏览器开发者工具,观察事件:
```javascript
// 应收到事件
{
  event: 'login-status',
  payload: {
    status: 'scanned',
    cookies: null,
    updated_at: '2025-10-05T10:30:15Z'
  }
}
```

---

## 场景3: 确认登录并获取Cookies

**对应需求**: FR-004, FR-005, FR-006

### 操作步骤
1. 完成场景2,扫描二维码
2. 在微博App中点击"确认登录"

### 预期结果
- ✅ 应用在5秒内检测到确认事件
- ✅ 显示"正在验证..."
- ✅ Playwright验证在2秒内完成
- ✅ 显示"登录成功"和用户昵称
- ✅ Cookies保存到Redis

### 验证命令

#### 1. 查看确认日志
```bash
tail -f logs/weibo-login.log | grep "Cookies obtained"
```

**日志应包含**:
```json
{
  "level": "INFO",
  "fields": {
    "qr_id": "qr_...",
    "uid": "1234567890",
    "cookies_count": 5
  },
  "message": "Cookies obtained"
}
```

#### 2. 查看验证日志
```bash
tail -f logs/weibo-login.log | grep "Validation successful"
```

**日志应包含**:
```json
{
  "level": "INFO",
  "fields": {
    "uid": "1234567890",
    "validation_duration_ms": 350,
    "screen_name": "用户昵称"
  },
  "message": "Validation successful"
}
```

#### 3. 验证Redis存储
```bash
redis-cli HGETALL weibo:cookies:1234567890
```

**输出应包含**:
```
1) "cookies"
2) "{\"SUB\":\"xxx\",\"SUBP\":\"yyy\"}"
3) "fetched_at"
4) "1728123030"
5) "validated_at"
6) "1728123032"
7) "screen_name"
8) "用户昵称"
```

#### 4. 检查TTL
```bash
redis-cli TTL weibo:cookies:1234567890
```

**输出**: `2592000` (30天,单位秒)

---

## 场景4: 查询Cookies

**对应需求**: FR-007

### 操作步骤
1. 完成场景3,保存cookies
2. 在应用中点击"我的账户"或"Cookies管理"
3. 选择刚登录的账户,点击"查看Cookies"

### 预期结果
- ✅ 在100ms内显示Cookies详情
- ✅ 显示用户昵称、UID、获取时间
- ✅ Cookies键值对以表格形式展示
- ✅ 提供"复制"和"导出"按钮

### 前端验证
```javascript
// 调用查询命令
const cookiesData = await invoke('query_cookies', { uid: '1234567890' });

console.log(cookiesData);
// 输出:
// {
//   uid: '1234567890',
//   cookies: { SUB: 'xxx', SUBP: 'yyy' },
//   fetched_at: '2025-10-05T10:30:30Z',
//   validated_at: '2025-10-05T10:30:32Z',
//   redis_key: 'weibo:cookies:1234567890',
//   screen_name: '用户昵称'
// }
```

### 验证命令
```bash
# 查看查询日志 (DEBUG级别)
RUST_LOG=debug tail -f logs/weibo-login.log | grep "Cookies queried"
```

---

## 场景5: 重复登录覆盖

**对应需求**: FR-012

### 操作步骤
1. 完成场景3,第一次登录成功
2. **使用同一微博账户**再次执行场景1-3
3. 确认第二次登录

### 预期结果
- ✅ 第二次登录流程正常完成
- ✅ 日志显示 `is_overwrite: true`
- ✅ Redis中只保留最新的cookies
- ✅ 旧的cookies被完全替换

### 验证命令

#### 1. 第一次登录后记录cookies
```bash
redis-cli HGET weibo:cookies:1234567890 cookies > /tmp/cookies_v1.txt
```

#### 2. 第二次登录后对比
```bash
redis-cli HGET weibo:cookies:1234567890 cookies > /tmp/cookies_v2.txt
diff /tmp/cookies_v1.txt /tmp/cookies_v2.txt
```

**预期**: 内容不同 (cookies已更新)

#### 3. 查看覆盖日志
```bash
tail -f logs/weibo-login.log | grep "is_overwrite"
```

**日志应包含**:
```json
{
  "level": "INFO",
  "fields": {
    "uid": "1234567890",
    "redis_key": "weibo:cookies:1234567890",
    "is_overwrite": true
  },
  "message": "Cookies saved to Redis"
}
```

---

## 场景6: 二维码过期处理

**对应需求**: FR-008

### 操作步骤
1. 生成二维码
2. **等待180秒不扫描**
3. 观察应用状态

### 预期结果
- ✅ 倒计时归零时自动检测过期
- ✅ 显示"二维码已过期"
- ✅ 提供"刷新"按钮
- ✅ 点击刷新后生成新二维码

### 验证命令
```bash
# 查看过期日志
tail -f logs/weibo-login.log | grep "expired"
```

**日志应包含**:
```json
{
  "level": "WARN",
  "fields": {
    "qr_id": "qr_...",
    "event_type": "QrCodeExpired"
  },
  "message": "QR code expired"
}
```

### 前端验证
```javascript
// 应收到过期事件
{
  event: 'login-status',
  payload: {
    status: 'expired',
    cookies: null
  }
}
```

---

## 场景7: Cookies验证失败

**对应需求**: FR-013

### 操作步骤
1. **手动构造无效cookies** (通过开发者工具或直接调用save_cookies)
2. 尝试保存

### 预期结果
- ✅ Playwright验证返回失败 (401或403)
- ✅ 显示"Cookies无效,请重新登录"
- ✅ **不保存到Redis**
- ✅ 日志记录验证失败原因

### 测试代码
```javascript
// 在浏览器控制台执行
await invoke('save_cookies', {
  uid: '9999999999',
  cookies: {
    'SUB': 'invalid_token',
    'SUBP': 'invalid_subp'
  }
});
```

### 预期错误
```javascript
Error: ValidationError::ProfileApiFailed
{
  message: "Profile API call failed with status 401",
  cookies_sample: "SUB, SUBP",
  api_response: {
    status: 401,
    message: "Invalid credentials"
  }
}
```

### 验证Redis未保存
```bash
redis-cli EXISTS weibo:cookies:9999999999
# 输出: (integer) 0
```

### 验证日志
```bash
tail -f logs/weibo-login.log | grep "Validation failed"
```

**日志应包含**:
```json
{
  "level": "WARN",
  "fields": {
    "uid": "9999999999",
    "cookies_sample": "SUB, SUBP",
    "api_status": 401
  },
  "message": "Validation failed"
}
```

---

## 场景8: 网络中断恢复

**对应需求**: FR-010

### 操作步骤
1. 生成二维码,开始轮询
2. **断开网络连接** (禁用WiFi或拔网线)
3. **等待10秒**
4. **恢复网络连接**
5. 扫描并确认登录

### 预期结果
- ✅ 网络中断时显示"网络连接中断"
- ✅ 轮询自动重试 (不崩溃)
- ✅ 网络恢复后自动继续轮询
- ✅ 登录流程正常完成

### 验证日志
```bash
tail -f logs/weibo-login.log | grep -E "NetworkFailed|Poll"
```

**应看到重试日志**:
```json
{
  "level": "ERROR",
  "fields": {
    "error": "NetworkFailed",
    "retry_count": 1
  },
  "message": "Poll failed, retrying..."
}
```

---

## 场景9: Redis连接失败

**对应需求**: FR-010

### 操作步骤
1. 完成登录,获取到cookies
2. **停止Redis服务**:
   ```bash
   docker stop weibo-redis
   ```
3. 尝试保存cookies或查询

### 预期结果
- ✅ 显示"存储服务不可用,请稍后重试"
- ✅ **应用不崩溃**
- ✅ 提供重试按钮
- ✅ 日志记录详细错误信息

### 验证错误处理
```javascript
// 应捕获错误
try {
  await invoke('save_cookies', { /* ... */ });
} catch (error) {
  console.error(error);
  // {
  //   error: "StorageError::RedisConnectionFailed",
  //   message: "Failed to get Redis connection: pool timeout",
  //   endpoint: "redis://localhost:6379",
  //   retry_count: 3
  // }
}
```

### 恢复测试
```bash
# 重启Redis
docker start weibo-redis

# 点击重试按钮,应成功保存
```

---

## 场景10: 多账户管理

**对应需求**: FR-011

### 操作步骤
1. 使用**微博账户A**完成登录
2. 使用**微博账户B**完成登录
3. 使用**微博账户C**完成登录
4. 查看账户列表

### 预期结果
- ✅ 三个账户独立存储,互不干扰
- ✅ 每个账户有独立的Redis key
- ✅ 账户列表显示所有登录账户
- ✅ 可分别查询、删除各账户cookies

### 验证Redis存储
```bash
redis-cli KEYS "weibo:cookies:*"
# 输出:
# 1) "weibo:cookies:1111111111"  # 账户A
# 2) "weibo:cookies:2222222222"  # 账户B
# 3) "weibo:cookies:3333333333"  # 账户C
```

### 验证数据隔离
```bash
# 查询账户A的cookies
redis-cli HGET weibo:cookies:1111111111 cookies

# 查询账户B的cookies
redis-cli HGET weibo:cookies:2222222222 cookies

# 应返回不同的数据
```

---

## 性能验证

### 响应时间测试

#### 二维码生成
```bash
# 使用time命令测试
time curl -X POST http://localhost:1420/invoke/generate_qrcode
# 应 < 500ms
```

#### 状态轮询
```bash
# 记录轮询延迟
tail -f logs/weibo-login.log | grep -o 'poll_duration_ms":[0-9]*'
# 应 < 1000ms
```

#### Cookies验证
```bash
# 查看验证耗时
tail -f logs/weibo-login.log | grep -o 'validation_duration_ms":[0-9]*'
# 应 < 2000ms
```

#### Redis操作
```bash
# 使用redis-cli --latency测试
redis-cli --latency
# 应 < 1ms (本地)
```

---

## 日志完整性检查

### 检查必需日志事件
```bash
# 统计登录流程的所有事件
grep -E "Generated|Scanned|ConfirmedSuccess|ValidationSuccess" logs/weibo-login.log | wc -l
# 应至少有4行 (完整流程)
```

### 检查日志格式
```bash
# 验证JSON格式
tail -n 100 logs/weibo-login.log | jq . > /dev/null
# 应无错误输出
```

### 检查敏感数据泄漏
```bash
# 确保cookies值不在日志中
grep -i "SUB.*=.*" logs/weibo-login.log
# 应无输出 (仅记录key名称,不记录值)
```

---

## 清理操作

### 测试后清理
```bash
# 删除所有测试cookies
redis-cli KEYS "weibo:cookies:*" | xargs redis-cli DEL

# 清空日志
> logs/weibo-login.log

# 停止Redis
docker stop weibo-redis && docker rm weibo-redis
```

---

## 验收标准

所有场景通过,且满足:
- ✅ 无崩溃或未捕获异常
- ✅ 所有性能指标达标
- ✅ 日志结构化且无敏感数据泄漏
- ✅ 错误处理优雅,用户体验友好
- ✅ Redis数据完整且隔离正确

**通过标准**: 10个场景全部PASS + 性能验证 + 日志检查无问题
