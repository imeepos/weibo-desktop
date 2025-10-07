# 微博爬取服务 (Weibo Crawler)

## 概述

微博爬取服务通过WebSocket与Tauri后端通信，执行实际的微博搜索和数据提取。

## 架构

```
┌──────────────┐          ┌──────────────────┐          ┌─────────────┐
│ CrawlService │  WebSocket  │ weibo-crawler.ts │ Playwright │  微博API   │
│   (Rust)     │ ◄────────► │   (Node.js)      │ ───────►  │ (m.weibo.cn)│
└──────────────┘  Port 9224 └──────────────────┘          └─────────────┘
```

## 启动服务

### 1. 编译TypeScript

```bash
cd playwright
pnpm run build
```

### 2. 启动爬取服务

```bash
pnpm run crawler
```

服务将在 `ws://127.0.0.1:9224` 监听连接。

### 3. 验证服务运行

服务启动后会输出：
```
微博爬取服务已启动 - WebSocket端口: 9224
🔗 新的WebSocket连接建立
```

## 消息协议

### 请求格式

```json
{
  "action": "crawl_weibo_search",
  "payload": {
    "keyword": "国庆",
    "startTime": "20251001000000",
    "endTime": "20251001010000",
    "page": 1,
    "cookies": {
      "SUB": "...",
      "SUBP": "..."
    }
  }
}
```

### 响应格式

成功响应：
```json
{
  "success": true,
  "data": {
    "posts": [
      {
        "id": "5008471234567890",
        "text": "帖子内容...",
        "created_at": "Mon Oct 07 12:34:56 +0800 2025",
        "author_uid": "1234567890",
        "author_screen_name": "用户昵称",
        "reposts_count": 123,
        "comments_count": 456,
        "attitudes_count": 789
      }
    ],
    "hasMore": true,
    "totalResults": 1000,
    "captchaDetected": false
  }
}
```

失败响应：
```json
{
  "success": false,
  "error": "错误信息"
}
```

## 关键功能

### 1. 验证码检测

服务会自动检测页面中是否出现"验证码"文本：
- 检测到验证码时会自动截图保存
- 返回 `captchaDetected: true`
- CrawlService会自动暂停任务

### 2. 数据清洗

- 移除HTML标签
- 清理多余空白
- 提取正确的帖子ID (优先使用mid)
- 提取用户UID (支持id/idstr)

### 3. 错误处理

- 请求超时：30秒
- 自动重试：由CrawlService处理（3次，指数退避）
- 连接失败：返回明确错误信息

### 4. 资源管理

- Browser单例：复用同一个Chromium实例
- 会话隔离：每个请求创建独立的BrowserContext
- 自动清理：请求完成后立即关闭context

## 调试

### 启用详细日志

服务默认输出详细的日志：
```
[crawl_1696204800123] 开始爬取: keyword="国庆", page=1
[crawl_1696204800123] 已设置 5 个cookies
[crawl_1696204800123] 请求URL: https://m.weibo.cn/api/container/getIndex?...
[crawl_1696204800123] 响应状态: 200
[crawl_1696204800123] 获取到 10 个cards
[crawl_1696204800123] 提取到 8 条有效帖子
[crawl_1696204800123] 会话已清理
```

### 验证码截图

检测到验证码时，截图保存在：
```
playwright/captcha_<timestamp>.png
```

## 常见问题

### 1. 端口冲突

如果9224端口被占用，修改 `weibo-crawler.ts` 中的 `PORT` 常量：
```typescript
const PORT = 9225; // 修改为其他端口
```

同时修改 `src-tauri/src/services/crawl_service.rs` 中的连接URL：
```rust
let ws_url = "ws://127.0.0.1:9225";
```

### 2. cookies失效

症状：返回空结果或验证码
解决：使用001功能重新扫码登录获取新cookies

### 3. 页面加载超时

症状：请求超过30秒
解决：
- 检查网络连接
- 增加 `REQUEST_TIMEOUT_MS` 值

### 4. Playwright安装

首次运行需要安装Chromium：
```bash
pnpm exec playwright install chromium
```

## 与登录服务的区别

| 特性 | 登录服务 (9223) | 爬取服务 (9224) |
|------|----------------|----------------|
| 用途 | 二维码登录 | 搜索爬取 |
| 持久化 | 会话保持180秒 | 请求即完成 |
| Browser管理 | 延迟清理 | 立即清理 |
| 错误恢复 | 自动重试轮询 | 返回错误 |

## 性能优化

1. **Browser复用**：全局单例避免重复启动
2. **快速响应**：context即用即毁，无资源泄漏
3. **JSON直接解析**：从API返回提取数据，无需DOM解析

## 维护建议

1. 定期更新Playwright版本
2. 监控验证码出现频率
3. 根据实际情况调整超时时间
4. 保持与微博API变化同步
