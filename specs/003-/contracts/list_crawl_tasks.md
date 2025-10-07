# Contract: list_crawl_tasks

**Command**: `list_crawl_tasks`
**Purpose**: 列出所有爬取任务及其状态
**Type**: Tauri Command

---

## Request Schema

```typescript
interface ListCrawlTasksRequest {
  /** 可选: 按状态过滤 */
  status?: "Created" | "HistoryCrawling" | "HistoryCompleted" | "IncrementalCrawling" | "Paused" | "Failed";

  /** 可选: 排序字段 */
  sortBy?: "createdAt" | "updatedAt" | "crawledCount";

  /** 可选: 排序方向 */
  sortOrder?: "asc" | "desc";
}
```

### 字段验证

| 字段 | 类型 | 必需 | 验证规则 |
|-----|------|------|---------|
| status | string | 否 | 必须是有效的CrawlStatus |
| sortBy | string | 否 | 必须是有效的排序字段 |
| sortOrder | string | 否 | 必须是"asc"或"desc" |

---

## Response Schema

### 成功响应

```typescript
interface ListCrawlTasksResponse {
  /** 任务列表 */
  tasks: CrawlTaskSummary[];

  /** 总任务数 */
  total: number;
}

interface CrawlTaskSummary {
  taskId: string;
  keyword: string;
  status: string;
  eventStartTime: string;
  crawledCount: number;
  createdAt: string;
  updatedAt: string;
  failureReason: string | null;
}
```

**HTTP状态码**: 200 OK

### 错误响应

```typescript
interface ErrorResponse {
  error: string;
  code: string;
}
```

---

## 错误码

| 错误码 | 消息 | 说明 | HTTP状态码 |
|-------|------|------|-----------|
| STORAGE_ERROR | "查询任务列表失败: {details}" | Redis操作失败 | 500 |

---

## 业务逻辑

1. **查询所有任务**:
   - 使用Redis `KEYS crawl:task:*` 获取所有任务键
   - 使用`MGET`批量读取任务数据

2. **过滤** (如果指定status):
   - 过滤掉不匹配的任务

3. **排序**:
   - 根据sortBy和sortOrder排序
   - 默认按created_at降序

4. **返回响应**:
   - 返回任务摘要列表

---

## 示例

### 成功案例: 列出所有任务

**请求**:
```json
{}
```

**响应**:
```json
{
  "tasks": [
    {
      "taskId": "550e8400-e29b-41d4-a716-446655440000",
      "keyword": "国庆",
      "status": "HistoryCrawling",
      "eventStartTime": "2025-10-01T00:00:00Z",
      "crawledCount": 1234,
      "createdAt": "2025-10-07T10:00:00Z",
      "updatedAt": "2025-10-07T12:34:56Z",
      "failureReason": null
    },
    {
      "taskId": "660e8400-e29b-41d4-a716-446655440001",
      "keyword": "中秋",
      "status": "HistoryCompleted",
      "eventStartTime": "2025-09-15T00:00:00Z",
      "crawledCount": 5678,
      "createdAt": "2025-09-20T08:00:00Z",
      "updatedAt": "2025-09-21T10:00:00Z",
      "failureReason": null
    }
  ],
  "total": 2
}
```

### 成功案例: 按状态过滤

**请求**:
```json
{
  "status": "Failed"
}
```

**响应**:
```json
{
  "tasks": [
    {
      "taskId": "770e8400-e29b-41d4-a716-446655440002",
      "keyword": "春节",
      "status": "Failed",
      "eventStartTime": "2025-02-01T00:00:00Z",
      "crawledCount": 100,
      "createdAt": "2025-02-10T10:00:00Z",
      "updatedAt": "2025-02-10T11:00:00Z",
      "failureReason": "网络请求失败: Connection timeout"
    }
  ],
  "total": 1
}
```

---

## 前置条件

- Redis服务运行正常

---

## 后置条件

- 无副作用 (只读操作)

---

## 依赖

- `RedisService::list_all_tasks()`: 列出所有任务

---

## 测试要点

1. **任务列表**:
   - [ ] 无任务时返回空数组
   - [ ] 返回所有任务的摘要信息

2. **过滤**:
   - [ ] 按状态过滤正确
   - [ ] 不指定status时返回所有任务

3. **排序**:
   - [ ] 按创建时间排序正确
   - [ ] 按更新时间排序正确
   - [ ] 按爬取数量排序正确

4. **数据完整性**:
   - [ ] 所有时间字段使用ISO 8601格式
   - [ ] Failed状态包含failureReason
