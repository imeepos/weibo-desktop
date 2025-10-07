# Contract: get_crawl_progress

**Command**: `get_crawl_progress`
**Purpose**: 查询任务的实时进度和统计信息
**Type**: Tauri Command

---

## Request Schema

```typescript
interface GetCrawlProgressRequest {
  /** 任务ID */
  taskId: string;
}
```

### 字段验证

| 字段 | 类型 | 必需 | 验证规则 |
|-----|------|------|---------|
| taskId | string | 是 | 非空,必须存在于Redis中 |

---

## Response Schema

### 成功响应

```typescript
interface CrawlProgress {
  /** 任务ID */
  taskId: string;

  /** 搜索关键字 */
  keyword: string;

  /** 任务状态 */
  status: "Created" | "HistoryCrawling" | "HistoryCompleted" | "IncrementalCrawling" | "Paused" | "Failed";

  /** 事件开始时间 */
  eventStartTime: string;  // ISO 8601

  /** 已爬取的最小帖子时间 */
  minPostTime: string | null;

  /** 已爬取的最大帖子时间 */
  maxPostTime: string | null;

  /** 已爬取帖子总数 */
  crawledCount: number;

  /** 任务创建时间 */
  createdAt: string;

  /** 最后更新时间 */
  updatedAt: string;

  /** 失败原因 (仅当status=Failed时有值) */
  failureReason: string | null;

  /** 检查点信息 (仅当status=Paused或任务正在运行时有值) */
  checkpoint: {
    shardStartTime: string;
    shardEndTime: string;
    currentPage: number;
    direction: "Backward" | "Forward";
    completedShards: number;  // 已完成的时间分片数量
  } | null;

  /** 预估进度百分比 (0-100) */
  estimatedProgress: number;
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
| TASK_NOT_FOUND | "任务 {taskId} 不存在" | Redis中不存在该任务 | 404 |
| STORAGE_ERROR | "查询任务失败: {details}" | Redis操作失败 | 500 |

---

## 业务逻辑

1. **加载任务**:
   - 从Redis读取任务 (`crawl:task:{task_id}`)
   - 验证任务存在

2. **加载检查点** (如果存在):
   - 尝试从Redis读取检查点 (`crawl:checkpoint:{task_id}`)
   - 如果任务正在运行或已暂停,检查点应该存在

3. **计算预估进度**:
   - 历史回溯模式:
     ```
     progress = (now - min_post_time) / (now - event_start_time) * 100
     ```
   - 增量更新模式:
     ```
     progress = 100 (已完成历史回溯)
     ```

4. **返回响应**:
   - 组装完整的进度信息

---

## 示例

### 成功案例: 正在历史回溯

**请求**:
```json
{
  "taskId": "550e8400-e29b-41d4-a716-446655440000"
}
```

**响应**:
```json
{
  "taskId": "550e8400-e29b-41d4-a716-446655440000",
  "keyword": "国庆",
  "status": "HistoryCrawling",
  "eventStartTime": "2025-10-01T00:00:00Z",
  "minPostTime": "2025-10-03T12:00:00Z",
  "maxPostTime": "2025-10-07T12:00:00Z",
  "crawledCount": 1234,
  "createdAt": "2025-10-07T10:00:00Z",
  "updatedAt": "2025-10-07T12:34:56Z",
  "failureReason": null,
  "checkpoint": {
    "shardStartTime": "2025-10-03T00:00:00Z",
    "shardEndTime": "2025-10-04T00:00:00Z",
    "currentPage": 15,
    "direction": "Backward",
    "completedShards": 3
  },
  "estimatedProgress": 67
}
```

### 成功案例: 历史回溯完成

**请求**:
```json
{
  "taskId": "550e8400-e29b-41d4-a716-446655440000"
}
```

**响应**:
```json
{
  "taskId": "550e8400-e29b-41d4-a716-446655440000",
  "keyword": "国庆",
  "status": "HistoryCompleted",
  "eventStartTime": "2025-10-01T00:00:00Z",
  "minPostTime": "2025-10-01T00:00:00Z",
  "maxPostTime": "2025-10-07T12:00:00Z",
  "crawledCount": 12345,
  "createdAt": "2025-10-07T10:00:00Z",
  "updatedAt": "2025-10-07T13:34:56Z",
  "failureReason": null,
  "checkpoint": null,
  "estimatedProgress": 100
}
```

### 成功案例: 任务失败

**请求**:
```json
{
  "taskId": "550e8400-e29b-41d4-a716-446655440000"
}
```

**响应**:
```json
{
  "taskId": "550e8400-e29b-41d4-a716-446655440000",
  "keyword": "国庆",
  "status": "Failed",
  "eventStartTime": "2025-10-01T00:00:00Z",
  "minPostTime": "2025-10-03T12:00:00Z",
  "maxPostTime": "2025-10-07T12:00:00Z",
  "crawledCount": 500,
  "createdAt": "2025-10-07T10:00:00Z",
  "updatedAt": "2025-10-07T11:23:45Z",
  "failureReason": "网络请求失败: Connection timeout after 3 retries",
  "checkpoint": {
    "shardStartTime": "2025-10-03T00:00:00Z",
    "shardEndTime": "2025-10-04T00:00:00Z",
    "currentPage": 8,
    "direction": "Backward",
    "completedShards": 1
  },
  "estimatedProgress": 25
}
```

### 错误案例: 任务不存在

**请求**:
```json
{
  "taskId": "nonexistent_task_id"
}
```

**响应**:
```json
{
  "error": "任务 nonexistent_task_id 不存在",
  "code": "TASK_NOT_FOUND"
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

- `RedisService::load_task(task_id)`: 加载任务
- `RedisService::load_checkpoint(task_id)`: 加载检查点

---

## 测试要点

1. **任务加载**:
   - [ ] 任务不存在时返回TASK_NOT_FOUND
   - [ ] 成功加载任务信息

2. **检查点加载**:
   - [ ] 运行中或暂停的任务包含检查点信息
   - [ ] Created状态的任务checkpoint为null

3. **进度计算**:
   - [ ] 历史回溯模式的进度计算正确
   - [ ] 已完成的任务进度为100

4. **数据完整性**:
   - [ ] 所有时间字段使用ISO 8601格式
   - [ ] Failed状态包含failureReason
   - [ ] 返回数据与Redis存储一致
