# API 文档

本文档说明前端可调用的所有Tauri命令。

## 类型定义

参见 `src/types/weibo.ts`

## Tauri Commands

### 1. generate_qrcode

生成微博登录二维码。

**函数签名**:
```typescript
invoke<GenerateQrcodeResponse>('generate_qrcode')
```

**参数**: 无

**返回值**:
```typescript
interface GenerateQrcodeResponse {
  session: LoginSession;
  qr_image: string; // base64编码的PNG图片
}
```

**示例**:
```typescript
import { invoke } from '@tauri-apps/api/tauri';

const response = await invoke<GenerateQrcodeResponse>('generate_qrcode');
console.log('二维码ID:', response.session.qr_id);
console.log('过期时间:', response.session.expires_at);
```

**错误**:
- `"Failed to generate QR code: NetworkFailed"` - 网络错误
- `"Failed to generate QR code: RateLimited"` - 请求过于频繁

---

### 2. poll_login_status

轮询二维码登录状态。

**函数签名**:
```typescript
invoke<PollStatusResponse>('poll_login_status', { qrId: string })
```

**参数**:
```typescript
{
  qrId: string; // 二维码ID (来自generate_qrcode)
}
```

**返回值**:
```typescript
interface PollStatusResponse {
  event: LoginEvent;
  is_final: boolean; // true表示终态(成功/失败/过期)
}
```

**示例**:
```typescript
const response = await invoke<PollStatusResponse>('poll_login_status', {
  qrId: 'qr_abc123'
});

if (response.event.event_type === 'validation_success') {
  console.log('登录成功!', response.event.details.screen_name);
}
```

**轮询建议**:
- 间隔: 3秒
- 直到 `is_final === true`
- 最长轮询3分钟

---

### 3. save_cookies

手动保存Cookies (通常由poll_login_status自动调用)。

**函数签名**:
```typescript
invoke<SaveCookiesResponse>('save_cookies', {
  request: SaveCookiesRequest
})
```

**参数**:
```typescript
interface SaveCookiesRequest {
  uid: string;
  cookies: Record<string, string>;
  screen_name?: string;
}
```

**返回值**:
```typescript
interface SaveCookiesResponse {
  success: boolean;
  redis_key: string;
  validation_duration_ms: number;
  is_overwrite: boolean;
}
```

**示例**:
```typescript
const response = await invoke<SaveCookiesResponse>('save_cookies', {
  request: {
    uid: '1234567890',
    cookies: { SUB: 'xxx', SUBP: 'yyy' },
    screen_name: '用户昵称'
  }
});

console.log('验证耗时:', response.validation_duration_ms, 'ms');
```

**错误**:
- `"Validation failed: ProfileApiFailed"` - Cookies无效
- `"Failed to save: RedisConnectionFailed"` - Redis连接失败

---

### 4. query_cookies

查询已保存的Cookies。

**函数签名**:
```typescript
invoke<CookiesData>('query_cookies', { uid: string })
```

**参数**:
```typescript
{
  uid: string; // 微博用户ID
}
```

**返回值**:
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

**示例**:
```typescript
const cookies = await invoke<CookiesData>('query_cookies', {
  uid: '1234567890'
});

console.log('Cookies:', cookies.cookies);
console.log('获取时间:', new Date(cookies.fetched_at));
```

**错误**:
- `"Query failed: NotFound"` - 该UID的Cookies不存在

---

### 5. delete_cookies

删除已保存的Cookies。

**函数签名**:
```typescript
invoke<void>('delete_cookies', { uid: string })
```

**参数**:
```typescript
{
  uid: string; // 微博用户ID
}
```

**返回值**: 无

**示例**:
```typescript
await invoke('delete_cookies', { uid: '1234567890' });
console.log('删除成功');
```

---

### 6. list_all_uids

列出所有已保存的UID。

**函数签名**:
```typescript
invoke<string[]>('list_all_uids')
```

**参数**: 无

**返回值**: `string[]` - UID数组

**示例**:
```typescript
const uids = await invoke<string[]>('list_all_uids');
console.log('已保存', uids.length, '个账户');
```

---

## 事件类型

### LoginEventType

```typescript
enum LoginEventType {
  QrCodeGenerated = 'qr_code_generated',
  QrCodeScanned = 'qr_code_scanned',
  ConfirmedSuccess = 'confirmed_success',
  ValidationSuccess = 'validation_success',
  QrCodeExpired = 'qr_code_expired',
  Error = 'error',
}
```

### 事件详情字段

不同事件类型的 `details` 字段内容:

**QrCodeGenerated**:
```json
{ "expires_in": 180 }
```

**QrCodeScanned**:
```json
{}
```

**ConfirmedSuccess**:
```json
{ "uid": "1234567890" }
```

**ValidationSuccess**:
```json
{
  "uid": "1234567890",
  "screen_name": "用户昵称"
}
```

**QrCodeExpired**:
```json
{}
```

**Error**:
```json
{ "error": "错误消息" }
```

---

## 错误处理最佳实践

```typescript
try {
  const response = await invoke<GenerateQrcodeResponse>('generate_qrcode');
  // 成功处理
} catch (error) {
  // error 是字符串类型
  if (error.includes('NetworkFailed')) {
    // 网络错误,提示用户检查连接
  } else if (error.includes('RateLimited')) {
    // 限流,建议60秒后重试
  } else {
    // 其他错误
    console.error('未知错误:', error);
  }
}
```

---

## 性能指标

- `generate_qrcode`: < 500ms
- `poll_login_status`: < 1s (单次轮询)
- `save_cookies`: < 3s (含验证)
- `query_cookies`: < 100ms
- `delete_cookies`: < 100ms
