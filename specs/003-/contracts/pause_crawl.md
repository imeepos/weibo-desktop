# Contract: pause_crawl

**Command**: `pause_crawl`
**Purpose**: 暂停正在运行的爬取任务
**Type**: Tauri Command

---

## Request Schema

```typescript
interface PauseCrawlRequest {
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
interface PauseCrawlResponse {
  /** 确认消息 */
  message: string;

  /** 暂停时的检查点信息 */
  checkpoint: {
    shardStartTime: string;  // ISO 8601
    shardEndTime: string;
    currentPage: number;
    crawledCount: number;
  };
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
| INVALID_STATUS | "任务状态 {status} 无法暂停,仅支持HistoryCrawling/IncrementalCrawling" | 任务未在运行 | 400 |
| STORAGE_ERROR | "暂停任务失败: {details}" | Redis操作失败 | 500 |

---

## 业务逻辑

1. **加载任务**:
   - 从Redis读取任务 (`crawl:task:{task_id}`)
   - 验证任务存在

2. **状态检查**:
   - 检查当前状态是否允许暂停:
     - `HistoryCrawling` ✅
     - `IncrementalCrawling` ✅
     - 其他状态 ❌

3. **停止后台任务**:
   - 向后台爬取任务发送取消信号 (tokio CancellationToken)
   - 等待当前页爬取完成 (最多等待10秒)
   - 保存最后一次检查点到Redis

4. **状态转换**:
   - 转换到 `Paused` 状态
   - 更新 `updated_at` 时间戳

5. **返回响应**:
   - 返回确认消息和检查点信息

---

## 示例

### 成功案例

**请求**:
```json
{
  "taskId": "550e8400-e29b-41d4-a716-446655440000"
}
```

**响应**:
```json
{
  "message": "任务已暂停,可通过start_crawl恢复",
  "checkpoint": {
    "shardStartTime": "2025-10-06T00:00:00Z",
    "shardEndTime": "2025-10-07T00:00:00Z",
    "currentPage": 15,
    "crawledCount": 300
  }
}
```

### 错误案例: 任务未运行

**请求**:
```json
{
  "taskId": "550e8400-e29b-41d4-a716-446655440000"
}
```

**响应** (当任务status=Created时):
```json
{
  "error": "任务状态 Created 无法暂停,仅支持HistoryCrawling/IncrementalCrawling",
  "code": "INVALID_STATUS"
}
```

---

## 前置条件

- 任务已启动 (status=HistoryCrawling或IncrementalCrawling)
- Redis服务运行正常

---

## 后置条件

- 任务状态转换到Paused
- 检查点已保存到Redis
- 后台异步任务已停止
- 前端停止接收crawl-progress事件

---

## 依赖

- `RedisService::load_task(task_id)`: 加载任务
- `RedisService::save_checkpoint(checkpoint)`: 保存检查点
- `CrawlService::cancel_crawl(task_id)`: 取消后台任务
- `CrawlTask::transition_to(Paused)`: 状态转换

---

## 测试要点

1. **任务加载**:
   - [ ] 任务不存在时返回TASK_NOT_FOUND

2. **状态验证**:
   - [ ] HistoryCrawling状态可以暂停
   - [ ] IncrementalCrawling状态可以暂停
   - [ ] Created/Paused/HistoryCompleted状态无法暂停

3. **检查点保存**:
   - [ ] 暂停时保存当前爬取位置
   - [ ] 检查点包含时间范围和页码
   - [ ] 恢复后从检查点继续

4. **后台任务停止**:
   - [ ] 等待当前页完成后停止
   - [ ] 不会丢失已爬取的数据
