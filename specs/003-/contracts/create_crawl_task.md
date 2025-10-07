# Contract: create_crawl_task

**Command**: `create_crawl_task`
**Purpose**: 创建新的爬取任务
**Type**: Tauri Command

---

## Request Schema

```typescript
interface CreateCrawlTaskRequest {
  /** 搜索关键字 */
  keyword: string;

  /** 事件开始时间 (ISO 8601格式) */
  eventStartTime: string;

  /** 使用的微博账号UID (从001-cookies获取) */
  uid: string;
}
```

### 字段验证

| 字段 | 类型 | 必需 | 验证规则 |
|-----|------|------|---------|
| keyword | string | 是 | 非空,去除首尾空格后长度 ≥ 1 |
| eventStartTime | string | 是 | 符合ISO 8601格式,不能是未来时间 |
| uid | string | 是 | 非空,必须存在于Redis中 |

---

## Response Schema

### 成功响应

```typescript
interface CreateCrawlTaskResponse {
  /** 任务ID (UUID v4) */
  taskId: string;

  /** 任务创建时间 (ISO 8601) */
  createdAt: string;
}
```

**HTTP状态码**: 200 OK

### 错误响应

```typescript
interface ErrorResponse {
  /** 错误消息 */
  error: string;

  /** 错误代码 */
  code: string;
}
```

---

## 错误码

| 错误码 | 消息 | 说明 | HTTP状态码 |
|-------|------|------|-----------|
| INVALID_KEYWORD | "关键字不能为空" | keyword验证失败 | 400 |
| INVALID_TIME | "事件开始时间不能是未来时间" | eventStartTime验证失败 | 400 |
| COOKIES_NOT_FOUND | "未找到UID {uid} 的Cookies,请先扫码登录" | Redis中不存在该UID的cookies | 404 |
| COOKIES_EXPIRED | "Cookies可能已过期(验证时间>{days}天),请重新登录" | cookies年龄超过7天 | 403 |
| STORAGE_ERROR | "保存任务失败: {details}" | Redis操作失败 | 500 |

---

## 业务逻辑

1. **验证请求参数**:
   - 检查keyword非空
   - 解析eventStartTime,验证格式和时间有效性

2. **获取cookies**:
   - 调用`RedisService::query_cookies(uid)`
   - 检查cookies是否存在
   - 检查cookies年龄 (validated_at距今不超过7天)

3. **创建任务**:
   - 生成UUID作为task_id
   - 初始化CrawlTask (status=Created)
   - 保存到Redis (`crawl:task:{task_id}`)

4. **返回响应**:
   - 返回task_id和created_at

---

## 示例

### 成功案例

**请求**:
```json
{
  "keyword": "国庆",
  "eventStartTime": "2025-10-01T00:00:00Z",
  "uid": "1234567890"
}
```

**响应**:
```json
{
  "taskId": "550e8400-e29b-41d4-a716-446655440000",
  "createdAt": "2025-10-07T12:34:56Z"
}
```

### 错误案例: Cookies不存在

**请求**:
```json
{
  "keyword": "国庆",
  "eventStartTime": "2025-10-01T00:00:00Z",
  "uid": "nonexistent_uid"
}
```

**响应**:
```json
{
  "error": "未找到UID nonexistent_uid 的Cookies,请先扫码登录",
  "code": "COOKIES_NOT_FOUND"
}
```

### 错误案例: Cookies过期

**请求**:
```json
{
  "keyword": "国庆",
  "eventStartTime": "2025-10-01T00:00:00Z",
  "uid": "1234567890"
}
```

**响应**:
```json
{
  "error": "Cookies可能已过期(验证时间>10天),请重新登录",
  "code": "COOKIES_EXPIRED"
}
```

---

## 前置条件

- Redis服务运行正常
- 指定UID的cookies已通过001-cookies功能保存到Redis
- cookies验证时间不超过7天

---

## 后置条件

- Redis中存在新任务记录 (`crawl:task:{task_id}`)
- 任务状态为Created
- 前端可通过task_id查询任务详情

---

## 依赖

- `RedisService::query_cookies(uid)`: 获取登录态
- `RedisService::save_crawl_task(task)`: 保存任务
- `CrawlTask::new()`: 创建任务模型
- `CrawlTask::validate()`: 验证任务数据

---

## 测试要点

1. **参数验证**:
   - [ ] keyword为空字符串时返回INVALID_KEYWORD
   - [ ] eventStartTime格式错误时返回INVALID_TIME
   - [ ] eventStartTime是未来时间时返回INVALID_TIME

2. **Cookies检查**:
   - [ ] uid不存在时返回COOKIES_NOT_FOUND
   - [ ] cookies年龄>7天时返回COOKIES_EXPIRED

3. **任务创建**:
   - [ ] 成功创建任务,返回有效UUID
   - [ ] 任务保存到Redis,status=Created
   - [ ] 多次调用生成不同的task_id

4. **错误处理**:
   - [ ] Redis连接失败时返回STORAGE_ERROR
   - [ ] 返回的错误消息包含足够的上下文信息
