# Contract: start_crawl

**Command**: `start_crawl`
**Purpose**: 启动爬取任务 (历史回溯或增量更新)
**Type**: Tauri Command (异步执行,通过事件推送进度)

---

## Request Schema

```typescript
interface StartCrawlRequest {
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
interface StartCrawlResponse {
  /** 确认消息 */
  message: string;

  /** 爬取方向 */
  direction: "Backward" | "Forward";
}
```

**说明**:
- 命令立即返回,实际爬取在后台执行
- 进度通过Tauri事件推送 (见"事件推送"章节)

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
| INVALID_STATUS | "任务状态 {status} 无法启动,仅支持Created/Paused/HistoryCompleted" | 状态机限制 | 400 |
| ALREADY_RUNNING | "已有任务正在运行,请先暂停或等待完成" | 并发限制 | 409 |
| STORAGE_ERROR | "启动任务失败: {details}" | Redis操作失败 | 500 |

---

## 业务逻辑

1. **加载任务**:
   - 从Redis读取任务 (`crawl:task:{task_id}`)
   - 验证任务存在

2. **状态检查**:
   - 检查当前状态是否允许启动:
     - `Created` → 启动历史回溯
     - `Paused` → 恢复上次爬取
     - `HistoryCompleted` → 启动增量更新
   - 如果有其他任务正在运行 (status=HistoryCrawling/IncrementalCrawling),返回ALREADY_RUNNING

3. **状态转换**:
   - `Created` → `HistoryCrawling`
   - `Paused` → 恢复到上次的活跃状态 (从检查点读取)
   - `HistoryCompleted` → `IncrementalCrawling`

4. **启动后台任务**:
   - 根据状态选择爬取方向:
     - `HistoryCrawling`: Backward (从现在到event_start_time)
     - `IncrementalCrawling`: Forward (从max_post_time到现在)
   - 创建异步任务 (tokio::spawn)

5. **返回响应**:
   - 立即返回成功消息
   - 后台任务通过事件推送进度

---

## 事件推送

爬取过程中通过Tauri事件推送进度:

### 事件: crawl-progress

**Payload**:
```typescript
interface CrawlProgressEvent {
  taskId: string;
  status: "HistoryCrawling" | "IncrementalCrawling";
  currentTimeRange: {
    start: string;  // ISO 8601
    end: string;
  };
  currentPage: number;
  crawledCount: number;
  timestamp: string;
}
```

**频率**: 每页爬取完成后推送

### 事件: crawl-completed

**Payload**:
```typescript
interface CrawlCompletedEvent {
  taskId: string;
  finalStatus: "HistoryCompleted" | "IncrementalCrawling";
  totalCrawled: number;
  duration: number;  // 秒
  timestamp: string;
}
```

**触发时机**: 历史回溯完成或增量更新完成

### 事件: crawl-error

**Payload**:
```typescript
interface CrawlErrorEvent {
  taskId: string;
  error: string;
  errorCode: "CAPTCHA_DETECTED" | "NETWORK_ERROR" | "STORAGE_ERROR";
  timestamp: string;
}
```

**触发时机**: 检测到验证码、网络错误或Redis错误

---

## 示例

### 成功案例: 启动历史回溯

**请求**:
```json
{
  "taskId": "550e8400-e29b-41d4-a716-446655440000"
}
```

**响应**:
```json
{
  "message": "任务已启动,开始历史回溯",
  "direction": "Backward"
}
```

**事件流**:
```javascript
// 1. 每页进度
emit("crawl-progress", {
  taskId: "550e8400-e29b-41d4-a716-446655440000",
  status: "HistoryCrawling",
  currentTimeRange: {
    start: "2025-10-06T00:00:00Z",
    end: "2025-10-07T00:00:00Z"
  },
  currentPage: 15,
  crawledCount: 300,
  timestamp: "2025-10-07T12:35:00Z"
});

// 2. 完成
emit("crawl-completed", {
  taskId: "550e8400-e29b-41d4-a716-446655440000",
  finalStatus: "HistoryCompleted",
  totalCrawled: 12345,
  duration: 3600,
  timestamp: "2025-10-07T13:34:56Z"
});
```

### 成功案例: 恢复暂停的任务

**请求**:
```json
{
  "taskId": "550e8400-e29b-41d4-a716-446655440000"
}
```

**响应**:
```json
{
  "message": "任务已恢复,从检查点继续爬取",
  "direction": "Backward"
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

### 错误案例: 状态不允许启动

**请求**:
```json
{
  "taskId": "550e8400-e29b-41d4-a716-446655440000"
}
```

**响应** (当任务status=Failed时):
```json
{
  "error": "任务状态 Failed 无法启动,仅支持Created/Paused/HistoryCompleted",
  "code": "INVALID_STATUS"
}
```

---

## 前置条件

- 任务已创建 (status=Created/Paused/HistoryCompleted)
- Redis服务运行正常
- Playwright server运行正常
- 没有其他任务正在运行

---

## 后置条件

- 任务状态转换到HistoryCrawling或IncrementalCrawling
- 后台异步任务开始执行
- 前端开始接收crawl-progress事件

---

## 依赖

- `RedisService::load_task(task_id)`: 加载任务
- `RedisService::load_checkpoint(task_id)`: 加载检查点 (恢复时)
- `CrawlService::start_history_crawl(task)`: 启动历史回溯
- `CrawlService::start_incremental_crawl(task)`: 启动增量更新
- `CrawlTask::transition_to(status)`: 状态转换

---

## 测试要点

1. **任务加载**:
   - [ ] 任务不存在时返回TASK_NOT_FOUND
   - [ ] 成功加载已创建的任务

2. **状态验证**:
   - [ ] Created状态可以启动
   - [ ] Paused状态可以恢复
   - [ ] HistoryCompleted状态可以启动增量更新
   - [ ] Failed状态无法启动,返回INVALID_STATUS

3. **并发控制**:
   - [ ] 已有任务运行时,启动第二个任务返回ALREADY_RUNNING

4. **事件推送**:
   - [ ] 每页爬取后推送crawl-progress
   - [ ] 历史回溯完成后推送crawl-completed
   - [ ] 检测到验证码后推送crawl-error

5. **恢复机制**:
   - [ ] 从Paused恢复时,加载检查点
   - [ ] 从检查点的下一页继续爬取
   - [ ] 不重复爬取已完成的时间分片
