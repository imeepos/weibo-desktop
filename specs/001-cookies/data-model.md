# Data Model: 微博扫码登录

**Feature**: 001-cookies
**Date**: 2025-10-05
**Purpose**: 定义核心数据结构、验证规则和状态转换

遵循章程原则:
- **存在即合理**: 每个字段都不可替代,无冗余数据
- **优雅即简约**: 类型名和字段名自文档化,无需注释
- **性能即艺术**: 使用高效的数据结构 (HashMap, enum)

---

## LoginSession (登录会话)

**Purpose**: 追踪一次完整的扫码登录流程,从二维码生成到确认完成。

### Rust定义
```rust
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginSession {
    /// 微博API返回的二维码唯一标识
    pub qr_id: String,

    /// 当前登录状态
    pub status: QrCodeStatus,

    /// 二维码生成时间
    pub created_at: DateTime<Utc>,

    /// 用户扫码时间 (可选,仅在Scanned后有值)
    pub scanned_at: Option<DateTime<Utc>>,

    /// 用户确认登录时间 (可选,仅在Confirmed后有值)
    pub confirmed_at: Option<DateTime<Utc>>,

    /// 二维码过期时间 (由微博API返回,通常为180秒)
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QrCodeStatus {
    /// 等待扫码
    Pending,

    /// 已扫码待确认
    Scanned,

    /// 已确认,登录成功
    Confirmed,

    /// 二维码已过期
    Expired,
}
```

### 字段说明

| 字段 | 类型 | 必需 | 说明 | 不可替代性 |
|------|------|------|------|-----------|
| `qr_id` | String | ✓ | 二维码ID | 轮询API必需,唯一标识会话 |
| `status` | QrCodeStatus | ✓ | 当前状态 | UI展示和流程控制必需 |
| `created_at` | DateTime | ✓ | 创建时间 | 计算过期、日志追溯必需 |
| `scanned_at` | Option<DateTime> | ✗ | 扫码时间 | 用户体验反馈、性能分析 |
| `confirmed_at` | Option<DateTime> | ✗ | 确认时间 | 完整流程追踪、SLA统计 |
| `expires_at` | DateTime | ✓ | 过期时间 | 前端倒计时、自动刷新判断 |

### 验证规则
```rust
impl LoginSession {
    /// 验证会话有效性
    pub fn validate(&self) -> Result<(), ValidationError> {
        // 规则1: qr_id非空
        if self.qr_id.is_empty() {
            return Err(ValidationError::EmptyQrId);
        }

        // 规则2: 时间戳递增性
        if let Some(scanned) = self.scanned_at {
            if scanned < self.created_at {
                return Err(ValidationError::InvalidTimestamp {
                    field: "scanned_at",
                    reason: "cannot be before created_at",
                });
            }
        }

        if let Some(confirmed) = self.confirmed_at {
            if let Some(scanned) = self.scanned_at {
                if confirmed < scanned {
                    return Err(ValidationError::InvalidTimestamp {
                        field: "confirmed_at",
                        reason: "cannot be before scanned_at",
                    });
                }
            }
        }

        // 规则3: 过期时间在创建时间之后
        if self.expires_at <= self.created_at {
            return Err(ValidationError::InvalidExpiry);
        }

        Ok(())
    }

    /// 检查是否已过期
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at || self.status == QrCodeStatus::Expired
    }
}
```

### 状态转换图
```
    ┌─────────┐
    │ Pending │ ◄─── 初始状态 (create时)
    └────┬────┘
         │ 用户扫码
         ▼
    ┌─────────┐
    │ Scanned │ ◄─── scanned_at记录
    └────┬────┘
         │ 用户确认
         ▼
    ┌───────────┐
    │ Confirmed │ ◄─── confirmed_at记录 (终态)
    └───────────┘

    任何状态 ──超时或API返回过期──► Expired (终态)
```

### TypeScript类型定义
```typescript
export interface LoginSession {
  qr_id: string;
  status: 'pending' | 'scanned' | 'confirmed' | 'expired';
  created_at: string; // ISO 8601
  scanned_at: string | null;
  confirmed_at: string | null;
  expires_at: string;
}
```

---

## CookiesData (Cookies数据)

**Purpose**: 存储和管理从微博获取的登录凭证,支持验证和Redis持久化。

### Rust定义
```rust
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CookiesData {
    /// 微博用户唯一标识 (从cookies中提取或API返回)
    pub uid: String,

    /// Cookie键值对 (如: SUB, SUBP, _T_WM等)
    pub cookies: HashMap<String, String>,

    /// Cookies获取时间
    pub fetched_at: DateTime<Utc>,

    /// 通过微博资料API验证的时间
    pub validated_at: DateTime<Utc>,

    /// Redis存储key (格式: weibo:cookies:{uid})
    pub redis_key: String,

    /// 用户昵称 (可选,验证时从API获取)
    pub screen_name: Option<String>,
}
```

### 字段说明

| 字段 | 类型 | 必需 | 说明 | 不可替代性 |
|------|------|------|------|-----------|
| `uid` | String | ✓ | 用户ID | Redis key生成、账户区分必需 |
| `cookies` | HashMap | ✓ | Cookie数据 | 核心凭证数据 |
| `fetched_at` | DateTime | ✓ | 获取时间 | 过期判断、审计追踪 |
| `validated_at` | DateTime | ✓ | 验证时间 | 确保cookies有效性 |
| `redis_key` | String | ✓ | 存储key | Redis操作必需 |
| `screen_name` | Option<String> | ✗ | 用户昵称 | UI展示、用户体验 |

### 验证规则
```rust
impl CookiesData {
    /// 创建新的Cookies数据
    pub fn new(uid: String, cookies: HashMap<String, String>) -> Self {
        let now = Utc::now();
        Self {
            redis_key: format!("weibo:cookies:{}", uid),
            uid,
            cookies,
            fetched_at: now,
            validated_at: now,
            screen_name: None,
        }
    }

    /// 验证数据完整性
    pub fn validate(&self) -> Result<(), ValidationError> {
        // 规则1: uid非空
        if self.uid.is_empty() {
            return Err(ValidationError::EmptyUid);
        }

        // 规则2: cookies非空
        if self.cookies.is_empty() {
            return Err(ValidationError::EmptyCookies);
        }

        // 规则3: 必需的cookie字段存在
        const REQUIRED_COOKIES: &[&str] = &["SUB", "SUBP"];
        for key in REQUIRED_COOKIES {
            if !self.cookies.contains_key(*key) {
                return Err(ValidationError::MissingCookie {
                    cookie_name: key.to_string(),
                });
            }
        }

        // 规则4: redis_key格式正确
        if !self.redis_key.starts_with("weibo:cookies:") {
            return Err(ValidationError::InvalidRedisKey);
        }

        Ok(())
    }

    /// 生成用于验证的cookies采样 (脱敏,仅用于日志)
    pub fn sample_for_logging(&self) -> String {
        self.cookies
            .keys()
            .map(|k| k.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// 转换为cookie header格式
    pub fn to_cookie_header(&self) -> String {
        self.cookies
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("; ")
    }
}
```

### Redis存储格式
```
Key: weibo:cookies:{uid}
Type: Hash
Fields:
  - cookies: JSON字符串 (HashMap<String, String>)
  - fetched_at: Unix timestamp
  - validated_at: Unix timestamp
  - screen_name: String (可选)

TTL: 30天 (2592000秒)
```

### TypeScript类型定义
```typescript
export interface CookiesData {
  uid: string;
  cookies: Record<string, string>;
  fetched_at: string;
  validated_at: string;
  redis_key: string;
  screen_name?: string;
}
```

---

## LoginEvent (登录事件)

**Purpose**: 记录登录流程中的所有关键事件,用于日志追踪和问题诊断。

### Rust定义
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginEvent {
    /// 事件类型
    pub event_type: LoginEventType,

    /// 事件发生时间
    pub timestamp: DateTime<Utc>,

    /// 关联的会话ID
    pub session_id: String,

    /// 关联的用户ID (可选,仅在获取到cookies后有值)
    pub uid: Option<String>,

    /// 事件详细信息 (JSON格式,灵活扩展)
    pub details: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoginEventType {
    /// 二维码生成成功
    Generated,

    /// 用户已扫码
    Scanned,

    /// 用户确认登录成功
    Confirmed,

    /// Cookies验证成功
    ValidationSuccess,

    /// Cookies验证失败
    ValidationFailed,

    /// 二维码过期
    QrCodeExpired,

    /// API调用失败
    ApiError,

    /// Redis存储失败
    StorageError,
}
```

### 字段说明

| 字段 | 类型 | 必需 | 说明 | 不可替代性 |
|------|------|------|------|-----------|
| `event_type` | Enum | ✓ | 事件类型 | 日志分类、问题定位必需 |
| `timestamp` | DateTime | ✓ | 发生时间 | 时序分析、性能监控 |
| `session_id` | String | ✓ | 会话ID | 关联同一登录流程的所有事件 |
| `uid` | Option<String> | ✗ | 用户ID | 用户级问题追踪 |
| `details` | JSON | ✓ | 详细信息 | 灵活扩展,包含错误信息、API响应等 |

### 事件创建辅助函数
```rust
impl LoginEvent {
    /// 创建生成二维码事件
    pub fn qrcode_generated(session_id: String, qr_id: String, expires_in: u64) -> Self {
        Self {
            event_type: LoginEventType::Generated,
            timestamp: Utc::now(),
            session_id,
            uid: None,
            details: serde_json::json!({
                "qr_id": qr_id,
                "expires_in_seconds": expires_in,
            }),
        }
    }

    /// 创建验证失败事件
    pub fn validation_failed(
        session_id: String,
        uid: String,
        error: &ValidationError,
    ) -> Self {
        Self {
            event_type: LoginEventType::ValidationFailed,
            timestamp: Utc::now(),
            session_id,
            uid: Some(uid),
            details: serde_json::json!({
                "error_type": error.error_type(),
                "error_message": error.to_string(),
            }),
        }
    }

    /// 创建API错误事件
    pub fn api_error(session_id: String, error: &ApiError) -> Self {
        Self {
            event_type: LoginEventType::ApiError,
            timestamp: Utc::now(),
            session_id,
            uid: None,
            details: serde_json::json!({
                "error_type": error.error_type(),
                "error_message": error.to_string(),
                "status_code": error.status_code(),
            }),
        }
    }
}
```

### 日志输出格式
使用 `tracing` 输出为结构化JSON:
```json
{
  "timestamp": "2025-10-05T10:30:45.123Z",
  "level": "INFO",
  "target": "weibo_login::events",
  "fields": {
    "event_type": "ValidationSuccess",
    "session_id": "qr_abc123",
    "uid": "1234567890",
    "details": {
      "screen_name": "用户昵称",
      "validation_duration_ms": 350
    }
  }
}
```

### TypeScript类型定义
```typescript
export type LoginEventType =
  | 'generated'
  | 'scanned'
  | 'confirmed'
  | 'validation_success'
  | 'validation_failed'
  | 'qrcode_expired'
  | 'api_error'
  | 'storage_error';

export interface LoginEvent {
  event_type: LoginEventType;
  timestamp: string;
  session_id: string;
  uid?: string;
  details: Record<string, unknown>;
}
```

---

## 错误类型定义

**Purpose**: 统一的错误处理,提供有意义的错误信息和上下文。

### Rust定义
```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("QR code ID cannot be empty")]
    EmptyQrId,

    #[error("User ID cannot be empty")]
    EmptyUid,

    #[error("Cookies cannot be empty")]
    EmptyCookies,

    #[error("Missing required cookie: {cookie_name}")]
    MissingCookie { cookie_name: String },

    #[error("Invalid Redis key format")]
    InvalidRedisKey,

    #[error("Invalid timestamp for field '{field}': {reason}")]
    InvalidTimestamp { field: &'static str, reason: &'static str },

    #[error("Expiry time must be after creation time")]
    InvalidExpiry,
}

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("Network request failed: {0}")]
    NetworkFailed(#[from] reqwest::Error),

    #[error("Invalid API response format")]
    InvalidResponse,

    #[error("QR code expired (generated at {generated_at}, expired at {expired_at})")]
    QrCodeExpired {
        generated_at: DateTime<Utc>,
        expired_at: DateTime<Utc>,
    },

    #[error("QR code not found: {qr_id}")]
    QrCodeNotFound { qr_id: String },

    #[error("API rate limit exceeded (retry after {retry_after}s)")]
    RateLimitExceeded { retry_after: u64 },
}

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("Failed to get Redis connection: {0}")]
    PoolError(#[from] deadpool_redis::PoolError),

    #[error("Redis operation failed: {0}")]
    RedisError(#[from] redis::RedisError),

    #[error("Cookies not found for UID: {0}")]
    NotFound(String),

    #[error("Serialization failed: {0}")]
    SerializationError(#[from] serde_json::Error),
}
```

---

## 总结

本数据模型设计严格遵循项目章程:

✅ **存在即合理**: 每个实体、字段、枚举值都有明确的存在理由,无冗余
✅ **优雅即简约**: 类型名自文档化,字段命名清晰,无需额外注释
✅ **性能即艺术**: 使用高效数据结构 (HashMap, enum),验证规则编译期优化
✅ **错误处理如为人处世的哲学**: 错误类型提供上下文,帮助诊断和恢复
✅ **日志是思想的表达**: LoginEvent记录关键路径,结构化输出便于分析

**下一步**: 创建 Tauri 命令契约 (contracts/*.md)。
