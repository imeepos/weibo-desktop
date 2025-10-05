# Contract: poll_login_status

**Tauri Command**: `poll_login_status`
**Purpose**: 轮询微博API检查二维码扫描和登录状态
**Feature**: 001-cookies

---

## 概述

定期调用此命令检查二维码状态,返回当前状态 (待扫描、已扫描、已确认、已过期) 和可能的cookies数据。前端根据状态更新UI反馈。

---

## Request

### 参数
```typescript
interface PollLoginStatusRequest {
  /// 二维码ID (从 generate_qrcode 获取)
  qr_id: string;
}
```

### Rust函数签名
```rust
#[tauri::command]
async fn poll_login_status(
    qr_id: String,
    state: tauri::State<'_, AppState>
) -> Result<LoginStatusResponse, ApiError>
```

### TypeScript调用
```typescript
import { invoke } from '@tauri-apps/api/tauri';

const response = await invoke<LoginStatusResponse>('poll_login_status', {
  qrId: 'qr_abc123xyz'
});
```

---

## Response

### 成功响应
```typescript
interface LoginStatusResponse {
  /// 当前状态
  status: 'pending' | 'scanned' | 'confirmed' | 'expired';

  /// Cookies数据 (仅在 status === 'confirmed' 时存在)
  cookies?: CookiesData;

  /// 状态更新时间
  updated_at: string;
}
```

### Rust类型
```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct LoginStatusResponse {
    pub status: QrCodeStatus,
    pub cookies: Option<CookiesData>,
    pub updated_at: DateTime<Utc>,
}
```

### 状态说明

#### Pending (待扫描)
```json
{
  "status": "pending",
  "cookies": null,
  "updated_at": "2025-10-05T10:30:00Z"
}
```
**前端行为**: 显示"请使用微博App扫描二维码"

#### Scanned (已扫描)
```json
{
  "status": "scanned",
  "cookies": null,
  "updated_at": "2025-10-05T10:30:15Z"
}
```
**前端行为**: 显示"已扫描,请在手机上确认登录"

#### Confirmed (已确认)
```json
{
  "status": "confirmed",
  "cookies": {
    "uid": "1234567890",
    "cookies": {
      "SUB": "xxx",
      "SUBP": "yyy"
    },
    "fetched_at": "2025-10-05T10:30:30Z",
    "validated_at": "2025-10-05T10:30:32Z",
    "redis_key": "weibo:cookies:1234567890",
    "screen_name": "用户昵称"
  },
  "updated_at": "2025-10-05T10:30:30Z"
}
```
**前端行为**: 显示"登录成功",跳转到主界面或显示cookies信息

#### Expired (已过期)
```json
{
  "status": "expired",
  "cookies": null,
  "updated_at": "2025-10-05T10:33:00Z"
}
```
**前端行为**: 显示"二维码已过期,请刷新重试",提供刷新按钮

---

## Error Cases

### ApiError::QrCodeNotFound
**触发条件**: 提供的 `qr_id` 在微博API中不存在或已被清理

**错误响应**:
```typescript
{
  error: "ApiError::QrCodeNotFound",
  message: "QR code not found: qr_abc123xyz",
  qr_id: "qr_abc123xyz"
}
```

**前端处理**: 提示二维码无效,引导用户重新生成

---

### ApiError::QrCodeExpired
**触发条件**: 二维码已超过有效期 (180秒)

**错误响应**:
```typescript
{
  error: "ApiError::QrCodeExpired",
  message: "QR code expired (generated at 2025-10-05T10:30:00Z, expired at 2025-10-05T10:33:00Z)",
  generated_at: "2025-10-05T10:30:00Z",
  expired_at: "2025-10-05T10:33:00Z"
}
```

**前端处理**: 等同于 `status: 'expired'`,提供刷新按钮

---

### ApiError::NetworkFailed
**触发条件**: 网络请求失败

**错误响应**:
```typescript
{
  error: "ApiError::NetworkFailed",
  message: "Network request failed: connection timeout"
}
```

**前端处理**: 显示网络错误提示,继续轮询 (自动重试)

---

## 轮询策略

### 前端实现
```typescript
async function startPolling(qrId: string) {
  let interval = 2000; // 初始2秒
  const maxInterval = 5000; // 最大5秒

  while (true) {
    try {
      const response = await invoke<LoginStatusResponse>('poll_login_status', { qrId });

      updateUI(response.status, response.cookies);

      // 终态退出轮询
      if (response.status === 'confirmed' || response.status === 'expired') {
        break;
      }

      // Exponential backoff
      interval = Math.min(interval + 1000, maxInterval);
    } catch (error) {
      console.error('Poll failed:', error);
      // 网络错误继续轮询,其他错误退出
      if (error.error !== 'ApiError::NetworkFailed') {
        break;
      }
    }

    await sleep(interval);
  }
}
```

### 后端优化
- 使用条件请求 (If-Modified-Since) 减少无效响应
- 状态变化时立即返回,无变化时等待最多2秒 (long polling)

---

## 日志记录

### 状态变化时记录
```rust
if new_status != previous_status {
    tracing::info!(
        qr_id = %qr_id,
        old_status = ?previous_status,
        new_status = ?new_status,
        "Status changed"
    );
}
```

### 获取到cookies时
```rust
tracing::info!(
    qr_id = %qr_id,
    uid = %cookies.uid,
    cookies_count = %cookies.cookies.len(),
    "Cookies obtained"
);
```

### 避免噪音日志
轮询循环中,**无状态变化时不记录日志**,仅记录状态转换和错误。

---

## 契约测试

### 测试文件
`tests/contract/test_poll_login_status.rs`

### 测试用例
```rust
#[tokio::test]
async fn test_poll_pending_status() {
    let response = poll_login_status("qr_pending".into(), mock_state()).await.unwrap();

    assert_eq!(response.status, QrCodeStatus::Pending);
    assert!(response.cookies.is_none());
}

#[tokio::test]
async fn test_poll_confirmed_status() {
    let response = poll_login_status("qr_confirmed".into(), mock_state()).await.unwrap();

    assert_eq!(response.status, QrCodeStatus::Confirmed);
    assert!(response.cookies.is_some());
    let cookies = response.cookies.unwrap();
    assert!(!cookies.uid.is_empty());
}

#[tokio::test]
async fn test_poll_expired_qrcode() {
    let result = poll_login_status("qr_expired".into(), mock_state()).await;

    assert!(matches!(result, Err(ApiError::QrCodeExpired { .. })));
}
```

---

## 性能要求

- 响应时间: < 1秒 (P95)
- 轮询间隔: 2-5秒 (exponential backoff)
- 最大轮询次数: 60次 (约3分钟,二维码有效期)
- 并发轮询: 最多5个会话同时轮询
