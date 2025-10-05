# Contract: query_cookies

**Tauri Command**: `query_cookies`
**Purpose**: 从Redis查询指定用户的cookies
**Feature**: 001-cookies

---

## 概述

根据微博用户ID从Redis读取已保存的cookies数据,用于展示、导出或后续API调用。不进行验证,直接返回存储的数据。

---

## Request

### 参数
```typescript
interface QueryCookiesRequest {
  /// 微博用户ID
  uid: string;
}
```

### Rust函数签名
```rust
#[tauri::command]
async fn query_cookies(
    uid: String,
    state: tauri::State<'_, AppState>
) -> Result<CookiesData, StorageError>
```

### TypeScript调用
```typescript
import { invoke } from '@tauri-apps/api/tauri';

const cookiesData = await invoke<CookiesData>('query_cookies', {
  uid: '1234567890'
});
```

---

## Response

### 成功响应
返回完整的 `CookiesData` 对象

```typescript
interface CookiesData {
  uid: string;
  cookies: Record<string, string>;
  fetched_at: string; // ISO 8601
  validated_at: string;
  redis_key: string;
  screen_name?: string;
}
```

### 示例
```json
{
  "uid": "1234567890",
  "cookies": {
    "SUB": "xxx",
    "SUBP": "yyy",
    "_T_WM": "zzz"
  },
  "fetched_at": "2025-10-05T10:30:30Z",
  "validated_at": "2025-10-05T10:30:32Z",
  "redis_key": "weibo:cookies:1234567890",
  "screen_name": "用户昵称"
}
```

---

## Error Cases

### StorageError::NotFound
**触发条件**: Redis中不存在该UID的cookies (从未登录或已过期删除)

**错误响应**:
```typescript
{
  error: "StorageError::NotFound",
  message: "Cookies not found for UID: 1234567890",
  uid: "1234567890"
}
```

**前端处理**: 提示"未找到该账户的登录信息,请先登录",跳转到登录页面

---

### StorageError::RedisConnectionFailed
**触发条件**: 无法连接到Redis

**错误响应**:
```typescript
{
  error: "StorageError::RedisConnectionFailed",
  message: "Failed to get Redis connection: pool timeout",
  endpoint: "redis://remote-server:6379",
  retry_count: 3
}
```

**前端处理**: 提示"存储服务不可用",提供重试按钮

---

### StorageError::SerializationError
**触发条件**: Redis中的数据格式损坏,无法反序列化

**错误响应**:
```typescript
{
  error: "StorageError::SerializationError",
  message: "Failed to deserialize cookies from Redis",
  redis_key: "weibo:cookies:1234567890"
}
```

**前端处理**: 提示"数据已损坏,请重新登录",删除本地缓存

---

## 实现逻辑

### Redis查询
```rust
async fn query_cookies(
    uid: String,
    state: tauri::State<'_, AppState>
) -> Result<CookiesData, StorageError> {
    let mut conn = state.redis_pool.get().await?;
    let redis_key = format!("weibo:cookies:{}", uid);

    // 检查key是否存在
    let exists: bool = redis::cmd("EXISTS")
        .arg(&redis_key)
        .query_async(&mut conn)
        .await?;

    if !exists {
        return Err(StorageError::NotFound(uid));
    }

    // 获取Hash所有字段
    let data: HashMap<String, String> = redis::cmd("HGETALL")
        .arg(&redis_key)
        .query_async(&mut conn)
        .await?;

    // 反序列化
    let cookies: HashMap<String, String> = serde_json::from_str(
        data.get("cookies").ok_or(StorageError::MissingField("cookies"))?
    )?;

    let fetched_at = DateTime::from_timestamp(
        data.get("fetched_at")
            .and_then(|s| s.parse::<i64>().ok())
            .ok_or(StorageError::MissingField("fetched_at"))?,
        0
    ).ok_or(StorageError::InvalidTimestamp)?;

    let validated_at = DateTime::from_timestamp(
        data.get("validated_at")
            .and_then(|s| s.parse::<i64>().ok())
            .ok_or(StorageError::MissingField("validated_at"))?,
        0
    ).ok_or(StorageError::InvalidTimestamp)?;

    Ok(CookiesData {
        uid,
        cookies,
        fetched_at,
        validated_at,
        redis_key,
        screen_name: data.get("screen_name").cloned(),
    })
}
```

---

## 日志记录

### 查询成功
```rust
tracing::debug!(
    uid = %uid,
    redis_key = %redis_key,
    cookies_count = %cookies_data.cookies.len(),
    "Cookies queried successfully"
);
```

### 未找到数据
```rust
tracing::warn!(
    uid = %uid,
    redis_key = %redis_key,
    "Cookies not found"
);
```

### Redis错误
```rust
tracing::error!(
    uid = %uid,
    error = ?e,
    "Failed to query cookies from Redis"
);
```

---

## 契约测试

### 测试文件
`tests/contract/test_query_cookies.rs`

### 测试用例
```rust
#[tokio::test]
async fn test_query_existing_cookies() {
    // 先保存数据
    let uid = "1234567890";
    save_test_cookies(uid, mock_state()).await.unwrap();

    // 查询
    let cookies_data = query_cookies(uid.into(), mock_state()).await.unwrap();

    assert_eq!(cookies_data.uid, uid);
    assert!(!cookies_data.cookies.is_empty());
    assert_eq!(cookies_data.redis_key, format!("weibo:cookies:{}", uid));
}

#[tokio::test]
async fn test_query_nonexistent_cookies() {
    let result = query_cookies("9999999999".into(), mock_state()).await;

    assert!(matches!(result, Err(StorageError::NotFound(_))));
}

#[tokio::test]
async fn test_query_corrupted_data() {
    // 插入损坏的数据
    insert_corrupted_data("1234567890", mock_state()).await;

    let result = query_cookies("1234567890".into(), mock_state()).await;

    assert!(matches!(result, Err(StorageError::SerializationError(_))));
}
```

---

## 性能要求

- 响应时间: < 100ms (P95)
- Redis操作超时: 5秒
- 并发查询: 支持最多50个并发请求 (受连接池大小限制)

---

## 扩展功能 (未来)

### 批量查询
```rust
#[tauri::command]
async fn query_all_cookies(
    state: tauri::State<'_, AppState>
) -> Result<Vec<CookiesData>, StorageError>
```

返回所有已保存的cookies,用于账户列表展示。

### Cookies有效性检查
在查询时可选择验证cookies是否仍然有效:
```typescript
interface QueryCookiesRequest {
  uid: string;
  validate?: boolean; // 是否调用Playwright验证
}
```

### 导出功能
提供导出为JSON或CSV格式的功能,方便用户备份。

---

## 安全考虑

- 查询操作不记录完整cookies到日志
- 前端展示时可选择隐藏cookies值,仅显示key和元数据
- 考虑添加访问频率限制,防止恶意查询
