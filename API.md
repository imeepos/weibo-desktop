# API 文档

本文档说明前端可调用的所有Tauri命令。

## 类型定义

参见 `src/types/weibo.ts`

## Tauri Commands

### 1. generate_qrcode

生成微博登录二维码。

**函数签名**:
```typescript
invoke<QrCodeResponse>('generate_qrcode')
```

**参数**: 无

**返回值**:
```typescript
interface QrCodeResponse {
  qr_id: string;          // 二维码唯一标识,用于后续轮询
  qr_image: string;       // Base64编码的PNG图片,可直接用于<img src={...} />
  expires_at: string;     // 过期时间 (ISO 8601格式)
  expires_in: number;     // 有效期秒数 (通常为180秒)
}
```

**示例**:
```typescript
import { invoke } from '@tauri-apps/api/tauri';

const response = await invoke<QrCodeResponse>('generate_qrcode');
console.log('二维码ID:', response.qr_id);
console.log('有效期:', response.expires_in, '秒');

// 显示二维码
<img src={`data:image/png;base64,${response.qr_image}`} />
```

**错误**:
- `NetworkFailed` - 网络错误
- `InvalidResponse` - 微博API响应异常
- `RateLimitExceeded` - 请求过于频繁

---

### 2. poll_login_status

轮询二维码登录状态。

**函数签名**:
```typescript
invoke<LoginStatusResponse>('poll_login_status', { qrId: string })
```

**参数**:
```typescript
{
  qrId: string; // 二维码ID (来自generate_qrcode的qr_id字段)
}
```

**返回值**:
```typescript
interface LoginStatusResponse {
  status: 'pending' | 'scanned' | 'confirmed' | 'expired';
  cookies?: CookiesData;  // 仅在 status === 'confirmed' 时存在
  updated_at: string;     // 状态更新时间 (ISO 8601)
}
```

**状态说明**:
- `pending` - 待扫描
- `scanned` - 已扫描,待确认
- `confirmed` - 已确认,登录成功 (包含cookies字段)
- `expired` - 二维码已过期

**示例**:
```typescript
const response = await invoke<LoginStatusResponse>('poll_login_status', {
  qrId: 'qr_abc123'
});

// 根据状态处理
switch (response.status) {
  case 'pending':
    console.log('等待扫码...');
    break;
  case 'scanned':
    console.log('已扫码,等待确认...');
    break;
  case 'confirmed':
    console.log('登录成功!', response.cookies?.screen_name);
    // cookies 已自动验证并保存到Redis
    break;
  case 'expired':
    console.log('二维码已过期,请重新生成');
    break;
}
```

**轮询建议**:
- 间隔: 2-5秒 (可使用exponential backoff优化)
- 终止条件: `status === 'confirmed'` 或 `status === 'expired'`
- 最长轮询时间: 3分钟 (与二维码有效期一致)

**自动化流程**:
当 `status === 'confirmed'` 时,后端已自动完成:
1. Playwright验证cookies有效性
2. 保存到Redis (30天TTL)
3. 返回完整的CookiesData

**错误**:
- `QrCodeNotFound` - 二维码不存在
- `QrCodeExpired` - 二维码已过期
- `NetworkFailed` - 网络错误

---

### 3. save_cookies

手动保存Cookies。

**说明**: 此命令通常由 `poll_login_status` 自动调用,无需手动使用。仅在特殊场景(如导入已有cookies)时使用。

**函数签名**:
```typescript
invoke<SaveCookiesResponse>('save_cookies', {
  uid: string;
  cookies: Record<string, string>;
  screenName?: string;
})
```

**参数**:
```typescript
{
  uid: string;                        // 微博用户ID
  cookies: Record<string, string>;    // Cookies键值对 (必须包含SUB)
  screenName?: string;                // 用户昵称 (可选,验证时会自动获取)
}
```

**返回值**:
```typescript
interface SaveCookiesResponse {
  success: boolean;
  redis_key: string;                // Redis存储键
  validation_duration_ms: number;   // 验证耗时(毫秒)
  is_overwrite: boolean;            // 是否覆盖已存在的cookies
}
```

**示例**:
```typescript
const response = await invoke<SaveCookiesResponse>('save_cookies', {
  uid: '1234567890',
  cookies: {
    SUB: '_2A25xxx...',
    SUBP: '0033xxx...'
  },
  screenName: '用户昵称'  // 可选
});

console.log('验证耗时:', response.validation_duration_ms, 'ms');
console.log('是否覆盖:', response.is_overwrite);
```

**验证流程**:
1. Playwright调用微博资料API验证cookies有效性
2. 提取并验证UID匹配
3. 保存到Redis (30天TTL)

**错误**:
- `ProfileApiFailed` - Cookies无效或已过期
- `MissingCookie` - 缺少必需的cookie字段
- `UidMismatch` - UID不匹配
- `RedisConnectionFailed` - Redis连接失败

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
  Confirmed = 'confirmed',
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

**Confirmed**:
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

Tauri命令抛出的错误对象结构:

```typescript
// Tauri错误对象格式
interface TauriError {
  error: string;      // 错误类型标识
  message?: string;   // 错误详细信息
  [key: string]: any; // 其他字段 (如status, uid等)
}
```

**推荐做法**:

```typescript
try {
  const response = await invoke<QrCodeResponse>('generate_qrcode');
  // 成功处理
} catch (err) {
  // err 可能是字符串或对象,需要安全解析
  const error = typeof err === 'string'
    ? { error: err }
    : err as TauriError;

  // 根据error字段判断错误类型
  switch (error.error) {
    case 'NetworkFailed':
      console.error('网络错误,请检查连接');
      break;
    case 'RateLimitExceeded':
      console.error('请求过于频繁,请60秒后重试');
      break;
    case 'QrCodeExpired':
      console.error('二维码已过期,请重新生成');
      break;
    case 'ProfileApiFailed':
      console.error('Cookies验证失败:', error.message);
      break;
    case 'UidMismatch':
      console.error(`UID不匹配: ${error.expected} vs ${error.actual}`);
      break;
    default:
      console.error('未知错误:', error);
  }
}
```

**常见错误类型**:

| 错误类型 | 触发命令 | 说明 |
|---------|---------|------|
| `NetworkFailed` | generate_qrcode, poll_login_status | 网络连接失败 |
| `InvalidResponse` | generate_qrcode | 微博API响应异常 |
| `RateLimitExceeded` | generate_qrcode | 请求频率超限 |
| `QrCodeExpired` | poll_login_status | 二维码已过期 |
| `QrCodeNotFound` | poll_login_status | 二维码不存在 |
| `ProfileApiFailed` | save_cookies | Cookies无效 |
| `MissingCookie` | save_cookies | 缺少必需cookie |
| `UidMismatch` | save_cookies | UID不匹配 |
| `RedisConnectionFailed` | save_cookies, query_cookies | Redis连接失败 |
| `NotFound` | query_cookies | Cookies不存在 |

---

## 性能指标

- `generate_qrcode`: < 500ms
- `poll_login_status`: < 1s (单次轮询)
- `save_cookies`: < 3s (含验证)
- `query_cookies`: < 100ms
- `delete_cookies`: < 100ms
