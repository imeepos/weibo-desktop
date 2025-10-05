# 实现说明

## ⚠️ 重要发现: WEIBO_APP_KEY 的使用问题

### 问题描述

当前实现中 `WEIBO_APP_KEY` 被用于调用微博OAuth2 API生成二维码:

```rust
// src-tauri/src/services/weibo_api.rs
let url = "https://api.weibo.com/oauth2/qrcode/generate";
.form(&[("client_id", &self.app_key)])
```

**但是这存在以下问题**:

1. **API端点是示例** - 代码注释已说明这是示例端点,实际使用需调整
2. **微博实际机制** - 微博的扫码登录通常不通过公开的OAuth2 API,而是通过登录页面
3. **App Key获取困难** - 微博开放平台的App Key需要企业认证,个人用户难以获取

### 正确的实现方式

根据规格说明 (spec.md) 的实际需求:"获取微博网站(weibo.com)的登录cookies"

**应该使用 Playwright 方式**:

```typescript
// playwright/src/weibo-login.ts
import { chromium } from 'playwright';

async function loginWeibo() {
  const browser = await chromium.launch({ headless: false });
  const page = await browser.newPage();

  // 1. 访问微博扫码登录页面
  await page.goto('https://weibo.com/login');

  // 2. 点击"扫码登录"标签
  await page.click('text=扫码登录');

  // 3. 获取二维码图片
  const qrImage = await page.locator('.qrcode img').getAttribute('src');

  // 4. 轮询检测登录状态
  await page.waitForURL('https://weibo.com/');

  // 5. 获取cookies
  const cookies = await page.context().cookies();

  return { cookies, qrImage };
}
```

### 推荐的架构调整

#### 方案1: 纯 Playwright 方案 (推荐)

**优点**:
- ✅ 不需要 App Key
- ✅ 使用真实的微博登录流程
- ✅ 可以获取完整的浏览器 cookies
- ✅ 可以处理验证码等复杂场景

**修改点**:
1. 移除 `WEIBO_APP_KEY` 环境变量
2. 修改 `WeiboApiClient` 为 `PlaywrightLoginService`
3. 使用 Playwright 生成二维码和轮询登录
4. `ValidationService` 仍然保留用于验证 cookies

#### 方案2: 混合方案

如果确实有可用的微博OAuth2 API:
- 保留当前的 `WeiboApiClient`
- 但需要提供真实的 API 端点文档
- 明确 App Key 的获取流程

### 当前代码状态

**可以正常编译和测试**,因为:
- 所有测试使用 Mock 服务
- App Key 只在运行时才会被调用

**但实际运行会失败**,因为:
- 示例的 API 端点不存在
- 无效的 App Key 会导致 401 错误

### 建议的后续工作

#### 短期 (修复当前实现)

1. **更新环境变量文档**:
```env
# .env.example
# ⚠️ 注意: 当前微博API端点为示例,实际使用需调整
# 如果使用OAuth2方式,需要从微博开放平台获取App Key (需企业认证)
# 推荐使用 Playwright 方式,无需此配置
WEIBO_APP_KEY=your_app_key_here_or_leave_empty_if_using_playwright
```

2. **添加实现说明**:
在 `README.md` 中说明当前限制和替代方案

#### 长期 (重构为 Playwright 方案)

1. 创建新的 Playwright 登录服务
2. 替换 `WeiboApiClient` 的实现
3. 移除对 App Key 的依赖
4. 更新所有相关文档

### 参考资料

- 微博登录页面: https://weibo.com/login
- Playwright 文档: https://playwright.dev/
- 当前 Playwright 脚本: `playwright/src/validate-cookies.ts` (仅用于验证,可扩展用于登录)

---

**结论**: 当前实现是基于假设存在公开的微博OAuth2 API,但实际情况可能不同。建议切换到 Playwright 方案以获得更可靠的实现。

**生成时间**: 2025-10-05
**状态**: 待决策 - 需要根据实际的微博API可用性选择方案
