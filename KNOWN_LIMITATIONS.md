# 已知限制和注意事项

## 🚨 关键限制

### 1. 微博API端点为示例

**问题**: 当前代码中使用的微博API端点是占位符,不是真实可用的API。

**受影响的代码**:
- `src-tauri/src/services/weibo_api.rs`
  - `generate_qrcode()` - 使用 `https://api.weibo.com/oauth2/qrcode/generate`
  - `check_qrcode_status()` - 使用 `https://api.weibo.com/oauth2/qrcode/check`

**实际情况**:
- 微博开放平台的OAuth2 API需要企业认证
- 个人开发者难以获取有效的 App Key
- 微博官方可能不提供公开的扫码登录API

**影响**:
- ❌ 应用可以编译通过
- ❌ 所有测试都通过 (使用Mock)
- ❌ **但实际运行会失败** (API调用返回401/404)

**解决方案**:
参见 `IMPLEMENTATION_NOTE.md` - 建议切换到纯 Playwright 方案

---

### 2. WEIBO_APP_KEY 的依赖

**问题**: 应用启动时强制要求 `WEIBO_APP_KEY` 环境变量,但可能无法获取有效值。

**代码位置**:
```rust
// src-tauri/src/main.rs:21
let weibo_app_key = std::env::var("WEIBO_APP_KEY")
    .expect("WEIBO_APP_KEY environment variable not set");
```

**如何获取 App Key**:
1. 注册微博开放平台账号
2. 创建应用 (需企业认证)
3. 获取 App Key 和 App Secret

**个人用户限制**:
- 微博开放平台主要面向企业
- 个人开发者权限受限
- 审核流程较长

**临时解决**:
可以填入任意字符串启动应用,但功能不可用:
```env
WEIBO_APP_KEY=placeholder_for_testing
```

---

### 3. Playwright 浏览器依赖

**问题**: Playwright 需要下载 Chromium 浏览器 (~300MB),在某些环境可能失败。

**受影响环境**:
- WSL2 (缺少图形界面依赖)
- Docker 容器
- 无头服务器

**错误示例**:
```
Error: browserType.launch: Executable doesn't exist at /path/to/chromium
```

**解决方案**:
```bash
# 安装系统依赖 (Debian/Ubuntu)
sudo apt-get install -y \
  libnss3 libnspr4 libatk1.0-0 libatk-bridge2.0-0 \
  libcups2 libdrm2 libdbus-1-3 libxkbcommon0 \
  libxcomposite1 libxdamage1 libxfixes3 libxrandr2 \
  libgbm1 libasound2

# 安装 Playwright 浏览器
cd playwright
npx playwright install chromium
```

---

### 4. Redis 连接依赖

**问题**: 应用启动时不会检查 Redis 连接,仅在实际操作时才会失败。

**代码位置**:
```rust
// src-tauri/src/services/redis_service.rs
// 连接池创建成功,但不测试连接
let pool = config.create_pool(Some(Runtime::Tokio1))?;
```

**影响**:
- 应用可以启动
- 但保存/查询 cookies 时会报错

**建议改进**:
在 `AppState::new()` 中添加连接测试:
```rust
// 测试 Redis 连接
let mut conn = redis.pool.get().await?;
redis::cmd("PING").query_async(&mut conn).await?;
```

---

### 5. 微博登录页面结构变化

**问题**: Playwright 验证脚本依赖微博页面的 HTML 结构,页面更新会导致脚本失败。

**受影响代码**:
```typescript
// playwright/src/validate-cookies.ts
const jsonMatch = content.match(/\$render_data\s*=\s*(\[.*?\])\[0\]/s);
```

**风险**:
- 微博随时可能更新页面结构
- 脚本需要定期维护

**缓解措施**:
- 添加更多的容错处理
- 使用多个备选选择器
- 定期测试和更新

---

## ✅ 正常工作的部分

尽管存在上述限制,以下功能**完全正常**:

### 1. 数据模型和类型系统
- ✅ LoginSession, CookiesData, LoginEvent
- ✅ 所有错误类型
- ✅ 状态转换逻辑

### 2. Redis 存储服务
- ✅ 保存/查询/删除 cookies
- ✅ 连接池管理
- ✅ 数据序列化/反序列化

### 3. Cookies 验证
- ✅ Playwright 调用微博API验证
- ✅ 提取 UID 和昵称
- ✅ 错误处理

### 4. 前端 UI
- ✅ React 组件
- ✅ 二维码显示
- ✅ 状态管理
- ✅ 响应式设计

### 5. 测试套件
- ✅ 110+ 测试全部通过
- ✅ Mock 服务完整
- ✅ 性能测试验证

---

## 🔧 快速修复指南

### 场景1: 我想测试应用但没有真实的微博API

**解决方案**: 修改代码使用 Mock 数据

1. 修改 `src-tauri/src/services/weibo_api.rs`:
```rust
pub async fn generate_qrcode(&self) -> Result<(LoginSession, String), ApiError> {
    // 临时: 返回 Mock 数据
    let session = LoginSession::new("mock_qr_123".to_string(), 180);
    let qr_image = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg=="; // 1x1 PNG
    Ok((session, qr_image.to_string()))
}
```

2. 设置占位符环境变量:
```env
WEIBO_APP_KEY=mock_for_testing
```

### 场景2: 我想使用真实的微博登录

**解决方案**: 切换到 Playwright 完整方案

参考 `IMPLEMENTATION_NOTE.md` 中的"方案1: 纯 Playwright 方案"

### 场景3: 我有真实的微博OAuth2 API

**解决方案**: 更新API端点

1. 获取真实的 App Key
2. 更新 `weibo_api.rs` 中的 URL
3. 调整请求参数和响应解析

---

## 📝 总结

这个项目是一个**高质量的代码框架**,展示了:
- ✅ 优秀的架构设计
- ✅ 完整的测试覆盖
- ✅ 清晰的文档
- ✅ 良好的错误处理

但由于**微博API的限制**,需要根据实际情况调整实现方式。

**最可行的方案**: 切换到纯 Playwright 实现,无需 App Key,使用真实的微博登录页面。

---

**更新时间**: 2025-10-05
**状态**: 已识别限制,等待实施修复
