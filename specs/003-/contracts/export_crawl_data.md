# Contract: export_crawl_data

**Command**: `export_crawl_data`
**Purpose**: 导出爬取数据为JSON或CSV格式
**Type**: Tauri Command

---

## Request Schema

```typescript
interface ExportCrawlDataRequest {
  /** 任务ID */
  taskId: string;

  /** 导出格式 */
  format: "json" | "csv";

  /** 可选: 时间范围过滤 */
  timeRange?: {
    start: string;  // ISO 8601
    end: string;
  };
}
```

### 字段验证

| 字段 | 类型 | 必需 | 验证规则 |
|-----|------|------|---------|
| taskId | string | 是 | 非空,必须存在于Redis中 |
| format | string | 是 | 必须是"json"或"csv" |
| timeRange | object | 否 | start必须早于end |

---

## Response Schema

### 成功响应

```typescript
interface ExportCrawlDataResponse {
  /** 导出文件路径 (绝对路径) */
  filePath: string;

  /** 导出的帖子数量 */
  exportedCount: number;

  /** 文件大小 (字节) */
  fileSize: number;

  /** 导出时间 */
  exportedAt: string;  // ISO 8601
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
| NO_DATA | "任务 {taskId} 尚无数据可导出" | crawled_count=0 | 400 |
| INVALID_FORMAT | "不支持的导出格式: {format}" | format验证失败 | 400 |
| FILE_SYSTEM_ERROR | "写入文件失败: {details}" | 文件系统操作失败 | 500 |
| STORAGE_ERROR | "读取帖子数据失败: {details}" | Redis操作失败 | 500 |

---

## 业务逻辑

1. **加载任务**:
   - 从Redis读取任务 (`crawl:task:{task_id}`)
   - 验证任务存在
   - 检查crawled_count > 0

2. **读取帖子数据**:
   - 如果指定timeRange,使用`ZRANGEBYSCORE`按时间范围查询
   - 否则使用`ZRANGE`读取所有帖子
   - 反序列化JSON为WeiboPost对象

3. **序列化数据**:
   - **JSON格式**:
     ```json
     {
       "task_id": "...",
       "keyword": "...",
       "exported_at": "...",
       "total_posts": 12345,
       "posts": [...]
     }
     ```
   - **CSV格式**:
     ```csv
     post_id,text,created_at,author_uid,author_screen_name,reposts,comments,likes,crawled_at
     ...
     ```

4. **写入文件**:
   - 文件名: `weibo_{taskId}_{timestamp}.{format}`
   - 保存到系统下载目录 (tauri::api::path::download_dir)

5. **返回响应**:
   - 返回文件绝对路径和统计信息

---

## 示例

### 成功案例: 导出JSON

**请求**:
```json
{
  "taskId": "550e8400-e29b-41d4-a716-446655440000",
  "format": "json"
}
```

**响应**:
```json
{
  "filePath": "/Users/username/Downloads/weibo_550e8400_1696204800.json",
  "exportedCount": 12345,
  "fileSize": 5242880,
  "exportedAt": "2025-10-07T14:00:00Z"
}
```

### 成功案例: 导出CSV (带时间范围)

**请求**:
```json
{
  "taskId": "550e8400-e29b-41d4-a716-446655440000",
  "format": "csv",
  "timeRange": {
    "start": "2025-10-01T00:00:00Z",
    "end": "2025-10-03T23:59:59Z"
  }
}
```

**响应**:
```json
{
  "filePath": "/Users/username/Downloads/weibo_550e8400_1696204800.csv",
  "exportedCount": 5678,
  "fileSize": 1048576,
  "exportedAt": "2025-10-07T14:00:00Z"
}
```

### 错误案例: 无数据

**请求**:
```json
{
  "taskId": "550e8400-e29b-41d4-a716-446655440000",
  "format": "json"
}
```

**响应** (当crawled_count=0时):
```json
{
  "error": "任务 550e8400-e29b-41d4-a716-446655440000 尚无数据可导出",
  "code": "NO_DATA"
}
```

### 错误案例: 不支持的格式

**请求**:
```json
{
  "taskId": "550e8400-e29b-41d4-a716-446655440000",
  "format": "xml"
}
```

**响应**:
```json
{
  "error": "不支持的导出格式: xml",
  "code": "INVALID_FORMAT"
}
```

---

## 导出文件示例

### JSON格式

```json
{
  "task_id": "550e8400-e29b-41d4-a716-446655440000",
  "keyword": "国庆",
  "exported_at": "2025-10-07T14:00:00Z",
  "total_posts": 2,
  "posts": [
    {
      "id": "5008471234567890",
      "task_id": "550e8400-e29b-41d4-a716-446655440000",
      "text": "今天国庆真热闹啊!",
      "created_at": "2025-10-01T12:34:56Z",
      "author_uid": "1234567890",
      "author_screen_name": "张三",
      "reposts_count": 123,
      "comments_count": 456,
      "attitudes_count": 789,
      "crawled_at": "2025-10-07T10:30:00Z"
    },
    {
      "id": "5008471234567891",
      "task_id": "550e8400-e29b-41d4-a716-446655440000",
      "text": "国庆快乐!",
      "created_at": "2025-10-02T08:00:00Z",
      "author_uid": "9876543210",
      "author_screen_name": "李四",
      "reposts_count": 50,
      "comments_count": 100,
      "attitudes_count": 200,
      "crawled_at": "2025-10-07T10:35:00Z"
    }
  ]
}
```

### CSV格式

```csv
post_id,text,created_at,author_uid,author_screen_name,reposts,comments,likes,crawled_at
5008471234567890,"今天国庆真热闹啊!",2025-10-01T12:34:56Z,1234567890,张三,123,456,789,2025-10-07T10:30:00Z
5008471234567891,"国庆快乐!",2025-10-02T08:00:00Z,9876543210,李四,50,100,200,2025-10-07T10:35:00Z
```

---

## 前置条件

- 任务已爬取至少1条帖子
- Redis服务运行正常
- 下载目录可写

---

## 后置条件

- 导出文件已保存到下载目录
- 无副作用 (Redis数据不变)

---

## 依赖

- `RedisService::load_task(task_id)`: 加载任务
- `RedisService::get_posts_by_time_range(task_id, start, end)`: 读取帖子
- `serde_json::to_string_pretty()`: JSON序列化
- `csv::Writer`: CSV序列化
- `tauri::api::path::download_dir()`: 获取下载目录

---

## 测试要点

1. **任务加载**:
   - [ ] 任务不存在时返回TASK_NOT_FOUND
   - [ ] 无数据时返回NO_DATA

2. **格式验证**:
   - [ ] 支持json格式
   - [ ] 支持csv格式
   - [ ] 不支持的格式返回INVALID_FORMAT

3. **数据导出**:
   - [ ] JSON格式正确,可解析
   - [ ] CSV格式正确,兼容Excel
   - [ ] 所有字段完整导出

4. **时间范围过滤**:
   - [ ] 指定时间范围时仅导出范围内帖子
   - [ ] 不指定时导出所有帖子

5. **文件系统**:
   - [ ] 文件保存到下载目录
   - [ ] 文件名包含task_id和时间戳
   - [ ] 返回的文件路径正确
