# Contract: save_cookies

**Tauri Command**: `save_cookies`
**Purpose**: 验证并保存cookies到Redis
**Feature**: 001-cookies

---

## 概述

接收从微博API获取的cookies,通过Playwright调用微博资料API验证有效性,验证通过后存储到Redis。如果同一账户已有cookies,直接覆盖更新。

---

## Request

### 参数
```typescript
interface SaveCookiesRequest {
  /// 微博用户ID
  uid: string;

  /// Cookie键值对
  cookies: Record<string, string>;

  /// 用户昵称 (可选,用于UI展示)
  screen_name?: string;
}
```

### Rust函数签名
```rust
#[tauri::command]
async fn save_cookies(
    uid: String,
    cookies: HashMap<String, String>,
    screen_name: Option<String>,
    state: tauri::State<'_, AppState>
) -> Result<SaveCookiesResponse, SaveCookiesError>
```

### TypeScript调用
```typescript
import { invoke } from '@tauri-apps/api/tauri';

const response = await invoke<SaveCookiesResponse>('save_cookies', {
  uid: '1234567890',
  cookies: {
    'SUB': 'xxx',
    'SUBP': 'yyy'
  },
  screenName: '用户昵称'
});
```

---

## Response

### 成功响应
```typescript
interface SaveCookiesResponse {
  /// 是否保存成功
  success: true;

  /// Redis存储key
  redis_key: string;

  /// 验证耗时 (毫秒)
  validation_duration_ms: number;

  /// 是否为覆盖更新 (true=覆盖旧数据, false=新插入)
  is_overwrite: boolean;
}
```

### Rust类型
```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct SaveCookiesResponse {
    pub success: bool,
    pub redis_key: String,
    pub validation_duration_ms: u64,
    pub is_overwrite: bool,
}
```

### 示例
```json
{
  "success": true,
  "redis_key": "weibo:cookies:1234567890",
  "validation_duration_ms": 350,
  "is_overwrite": false
}
```

---

## Error Cases

### ValidationError::ProfileApiFailed
**触发条件**: Cookies无效,调用微博资料API失败 (401、403状态码或返回错误)

**错误响应**:
```typescript
{
  error: "ValidationError::ProfileApiFailed",
  message: "Profile API call failed with status 401",
  cookies_sample: "SUB, SUBP, _T_WM", // 脱敏,仅显示key名称
  api_response: {
    status: 401,
    message: "Invalid credentials"
  }
}
```

**前端处理**: 提示"Cookies无效,请重新登录",清除本地缓存

---

### ValidationError::MissingCookie
**触发条件**: 缺少必需的cookie字段 (如 SUB, SUBP)

**错误响应**:
```typescript
{
  error: "ValidationError::MissingCookie",
  message: "Missing required cookie: SUB",
  cookie_name: "SUB"
}
```

**前端处理**: 提示系统错误,引导重新登录

---

### StorageError::RedisConnectionFailed
**触发条件**: 无法连接到Redis或连接池耗尽

**错误响应**:
```typescript
{
  error: "StorageError::RedisConnectionFailed",
  message: "Failed to get Redis connection: pool timeout",
  endpoint: "redis://remote-server:6379",
  retry_count: 3
}
```

**前端处理**: 提示"存储服务不可用,请稍后重试",可选择稍后自动重试

---

### StorageError::SerializationError
**触发条件**: Cookies序列化为JSON失败 (不太可能发生)

**错误响应**:
```typescript
{
  error: "StorageError::SerializationError",
  message: "Failed to serialize cookies to JSON"
}
```

**前端处理**: 提示系统错误,记录日志

---

## 处理流程

### 1. 数据验证
```rust
// 验证输入数据
let cookies_data = CookiesData::new(uid, cookies);
cookies_data.validate()?; // 检查必需字段
```

### 2. Cookies验证
```rust
// 调用Playwright验证
let validation_start = Instant::now();
let is_valid = validate_cookies_via_playwright(&cookies_data).await?;

if !is_valid {
    return Err(ValidationError::ProfileApiFailed { /* ... */ });
}

let validation_duration = validation_start.elapsed();
```

### 3. 检查是否覆盖
```rust
// 检查Redis是否已有数据
let redis_key = format!("weibo:cookies:{}", uid);
let exists = redis::cmd("EXISTS")
    .arg(&redis_key)
    .query_async(&mut conn)
    .await?;

let is_overwrite = exists == 1;
```

### 4. 保存到Redis
```rust
// 覆盖或新增
redis::cmd("HSET")
    .arg(&redis_key)
    .arg("cookies")
    .arg(serde_json::to_string(&cookies_data.cookies)?)
    .arg("fetched_at")
    .arg(cookies_data.fetched_at.timestamp())
    .arg("validated_at")
    .arg(cookies_data.validated_at.timestamp())
    .query_async(&mut conn)
    .await?;

// 设置30天过期
redis::cmd("EXPIRE")
    .arg(&redis_key)
    .arg(30 * 24 * 3600)
    .query_async(&mut conn)
    .await?;
```

---

## 日志记录

### 验证开始
```rust
tracing::info!(
    uid = %uid,
    cookies_count = %cookies.len(),
    "Starting cookies validation"
);
```

### 验证成功
```rust
tracing::info!(
    uid = %uid,
    validation_duration_ms = %validation_duration.as_millis(),
    screen_name = %screen_name.as_deref().unwrap_or("unknown"),
    "Validation successful"
);
```

### 验证失败
```rust
tracing::warn!(
    uid = %uid,
    cookies_sample = %cookies_data.sample_for_logging(),
    api_status = %response.status(),
    "Validation failed"
);
```

### 保存成功
```rust
tracing::info!(
    uid = %uid,
    redis_key = %redis_key,
    is_overwrite = %is_overwrite,
    "Cookies saved to Redis"
);
```

---

## 契约测试

### 测试文件
`tests/contract/test_save_cookies.rs`

### 测试用例
```rust
#[tokio::test]
async fn test_save_valid_cookies() {
    let cookies = hashmap! {
        "SUB".to_string() => "valid_token".to_string(),
        "SUBP".to_string() => "valid_subp".to_string(),
    };

    let response = save_cookies(
        "1234567890".into(),
        cookies,
        Some("测试用户".into()),
        mock_state()
    ).await.unwrap();

    assert!(response.success);
    assert_eq!(response.redis_key, "weibo:cookies:1234567890");
    assert!(!response.is_overwrite); // 首次保存
}

#[tokio::test]
async fn test_save_invalid_cookies() {
    let cookies = hashmap! {
        "SUB".to_string() => "invalid_token".to_string(),
    };

    let result = save_cookies(
        "1234567890".into(),
        cookies,
        None,
        mock_state()
    ).await;

    assert!(matches!(result, Err(ValidationError::ProfileApiFailed { .. })));
}

#[tokio::test]
async fn test_save_overwrite_existing() {
    // 先保存一次
    save_cookies("1234567890".into(), valid_cookies(), None, mock_state()).await.unwrap();

    // 再次保存同一UID
    let response = save_cookies(
        "1234567890".into(),
        new_valid_cookies(),
        None,
        mock_state()
    ).await.unwrap();

    assert!(response.is_overwrite); // 应为覆盖
}
```

---

## 性能要求

- 总响应时间: < 3秒 (P95,包含Playwright验证)
- Playwright验证: < 2秒
- Redis操作: < 100ms
- 重试策略: Playwright失败时重试1次,Redis失败时重试3次

---

## 安全考虑

- Cookies在日志中脱敏,仅记录key名称
- 使用 `tracing::instrument(skip_all)` 避免自动记录cookies
- Redis连接使用TLS (生产环境)
- Playwright运行在隔离进程,崩溃不影响主应用
