# Playwright 服务架构

## 统一服务架构 (推荐)

### 新架构: weibo-service.ts (端口 9223)

统一的 WebSocket 服务器,通过 `action` 字段路由请求到不同处理器。

#### 启动服务
```bash
pnpm run service
```

#### 消息协议

**统一请求格式**:
```typescript
{
  action: 'generate_qrcode' | 'crawl_weibo_search' | 'ping',
  payload?: any  // 根据action不同而不同
}
```

**统一响应格式**:
```typescript
{
  success: boolean,
  data?: any,
  error?: string,
  timestamp: number
}
```

#### 支持的 Actions

1. **generate_qrcode** - 生成登录二维码
   - 请求: `{ action: 'generate_qrcode' }`
   - 响应: 事件流 (qrcode_generated, status_update, login_confirmed, error)

2. **crawl_weibo_search** - 爬取微博搜索
   - 请求:
     ```typescript
     {
       action: 'crawl_weibo_search',
       payload: {
         keyword: string,
         page: number,
         cookies: Record<string, string>,
         startTime?: string,  // YYYYMMDDhhmmss
         endTime?: string     // YYYYMMDDhhmmss
       }
     }
     ```
   - 响应: `{ success: true, data: { posts, hasMore, totalResults, captchaDetected } }`

3. **ping** - 心跳检测
   - 请求: `{ action: 'ping' }`
   - 响应: `{ success: true, data: { type: 'pong', timestamp } }`

### 架构优势

1. **单一入口**: 只需要一个 WebSocket 服务器 (端口 9223)
2. **清晰路由**: 通过 action 字段路由到不同处理器
3. **统一管理**: 浏览器生命周期统一管理
4. **职责分离**: 处理器模块只关注业务逻辑
5. **向后兼容**: 支持旧的 `type` 字段自动映射到 `action`

### 目录结构

```
playwright/
├── src/
│   ├── weibo-service.ts           # 统一服务入口
│   ├── browser.ts                 # 浏览器实例管理
│   ├── types.ts                   # 共享类型定义
│   └── handlers/
│       ├── login-handler.ts       # 登录处理器
│       └── crawler-handler.ts     # 爬虫处理器
├── package.json
└── tsconfig.json
```

## 旧架构 (保留以便兼容)

### weibo-login-server.ts (端口 9223)
独立的登录服务器,处理二维码登录。

启动: `pnpm run server`

### weibo-crawler.ts (端口 9224)
独立的爬虫服务器,处理微博搜索爬取。

启动: `pnpm run crawler`

---

**推荐使用新的统一服务 `weibo-service.ts`,旧服务保留以便迁移期间兼容。**
