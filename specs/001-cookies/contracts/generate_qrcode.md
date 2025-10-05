# Contract: generate_qrcode

**Tauri Command**: `generate_qrcode`
**Purpose**: 调用微博API生成登录二维码
**Feature**: 001-cookies

---

## 概述

生成新的微博扫码登录二维码,返回二维码ID、图片数据和过期时间。前端使用此命令启动登录流程。

---

## Request

### 参数
无参数 (使用应用配置中的 `client_id`)

### Rust函数签名
```rust
#[tauri::command]
async fn generate_qrcode(
    state: tauri::State<'_, AppState>
) -> Result<QrCodeResponse, ApiError>
```

### TypeScript调用
```typescript
import { invoke } from '@tauri-apps/api/tauri';

const response = await invoke<QrCodeResponse>('generate_qrcode');
```

---

## Response

### 成功响应
```typescript
interface QrCodeResponse {
  /// 二维码唯一标识,用于后续轮询
  qr_id: string;

  /// Base64编码的二维码图片 (PNG格式)
  qr_image: string;

  /// 二维码过期时间 (ISO 8601格式)
  expires_at: string;

  /// 有效期秒数 (通常为180秒)
  expires_in: number;
}
```

### Rust类型
```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct QrCodeResponse {
    pub qr_id: String,
    pub qr_image: String,  // Base64 PNG
    pub expires_at: DateTime<Utc>,
    pub expires_in: u64,
}
```

### 示例
```json
{
  "qr_id": "qr_abc123xyz",
  "qr_image": "iVBORw0KGgoAAAANSUhEUgAA...",
  "expires_at": "2025-10-05T10:33:45Z",
  "expires_in": 180
}
```

---

## Error Cases

### ApiError::NetworkFailed
**触发条件**: 网络请求失败 (超时、DNS解析失败、连接中断)

**错误响应**:
```typescript
{
  error: "ApiError::NetworkFailed",
  message: "Network request failed: connection timeout"
}
```

**前端处理**: 提示用户检查网络,提供重试按钮

---

### ApiError::InvalidResponse
**触发条件**: 微博API返回非预期格式 (JSON解析失败、缺少必需字段)

**错误响应**:
```typescript
{
  error: "ApiError::InvalidResponse",
  message: "Invalid API response format"
}
```

**前端处理**: 提示系统错误,记录错误日志供调试

---

### ApiError::RateLimitExceeded
**触发条件**: 微博API返回429状态码 (请求过于频繁)

**错误响应**:
```typescript
{
  error: "ApiError::RateLimitExceeded",
  message: "API rate limit exceeded (retry after 60s)",
  retry_after: 60
}
```

**前端处理**: 显示倒计时,禁用生成按钮 `retry_after` 秒

---

## 日志记录

### 成功路径
```rust
tracing::info!(
    qr_id = %response.qr_id,
    expires_in = %response.expires_in,
    "QR code generated successfully"
);
```

### 失败路径
```rust
tracing::error!(
    error = ?e,
    "Failed to generate QR code"
);
```

---

## 契约测试

### 测试文件
`tests/contract/test_generate_qrcode.rs`

### 测试用例
```rust
#[tokio::test]
async fn test_generate_qrcode_success() {
    let response = generate_qrcode(mock_state()).await.unwrap();

    assert!(!response.qr_id.is_empty());
    assert!(!response.qr_image.is_empty());
    assert!(response.expires_in > 0);
    assert!(response.expires_at > Utc::now());
}

#[tokio::test]
async fn test_generate_qrcode_network_failure() {
    // Mock网络失败
    let result = generate_qrcode(mock_failing_state()).await;

    assert!(matches!(result, Err(ApiError::NetworkFailed(_))));
}
```

---

## 性能要求

- 响应时间: < 500ms (P95)
- 重试策略: 失败时最多重试3次,间隔1s, 2s, 4s (exponential backoff)
- 超时设置: 单次请求超时5秒

---

## 安全考虑

- `client_id` 存储在应用配置中,不暴露给前端
- 二维码图片使用Base64编码,避免临时文件泄漏
- 日志中不记录完整API响应,仅记录 `qr_id`
