# Contract: query_dependency_status

**Tauri Command**: `query_dependency_status`
**Purpose**: 查询依赖检测结果的缓存状态
**Feature**: 002-dependency-check

---

## 契约概述

从缓存(Redis或内存)中查询依赖项的检测结果。支持查询所有依赖或指定单个依赖。此命令不执行实际检测,仅返回已缓存的结果。

---

## Request

### 参数

```typescript
interface QueryDependencyStatusRequest {
  /// 依赖项ID (可选,为null时返回所有结果)
  dependency_id?: string | null;
}
```

### Rust函数签名

```rust
#[tauri::command]
async fn query_dependency_status(
    dependency_id: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<DependencyCheckResult>, String>
```

### TypeScript调用

```typescript
import { invoke } from '@tauri-apps/api/tauri';

// 查询所有依赖状态
const allResults = await invoke<DependencyCheckResult[]>('query_dependency_status', {
  dependency_id: null
});

// 查询单个依赖
const nodeResult = await invoke<DependencyCheckResult[]>('query_dependency_status', {
  dependency_id: 'nodejs'
});
```

---

## Response

### 成功响应

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyCheckResult {
    /// 关联的依赖项ID
    pub dependency_id: String,

    /// 检测开始时间(UTC)
    pub checked_at: String,  // ISO 8601

    /// 检测状态
    pub status: CheckStatus,

    /// 检测到的版本号(如果成功检测到)
    pub detected_version: Option<String>,

    /// 错误详情(status为失败状态时提供)
    pub error_details: Option<String>,

    /// 检测耗时(毫秒)
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckStatus {
    Satisfied,
    Missing,
    VersionMismatch,
    Corrupted,
}
```

### 示例响应 (查询所有)

```json
[
  {
    "dependency_id": "nodejs",
    "checked_at": "2025-10-05T10:30:15.123Z",
    "status": "satisfied",
    "detected_version": "20.10.0",
    "duration_ms": 45
  },
  {
    "dependency_id": "redis",
    "checked_at": "2025-10-05T10:30:15.200Z",
    "status": "missing",
    "detected_version": null,
    "error_details": "Executable 'redis-server' not found in PATH",
    "duration_ms": 30
  }
]
```

### 示例响应 (查询单个)

```json
[
  {
    "dependency_id": "nodejs",
    "checked_at": "2025-10-05T10:30:15.123Z",
    "status": "satisfied",
    "detected_version": "20.10.0",
    "duration_ms": 45
  }
]
```

### 示例响应 (未找到)

```json
[]
```

---

## Behavior

### 查询所有依赖 (dependency_id=None)

- 返回缓存中所有依赖的检测结果
- 结果按`dependency_id`字母序排序
- 如果缓存为空,返回空Vec `[]`
- 不抛出错误

### 查询单个依赖 (dependency_id=Some)

- 返回匹配`dependency_id`的结果(包装在Vec中)
- 如果不存在,返回空Vec `[]`
- 不抛出错误

### 缓存策略

- **Redis Key**: `dep:check:{dependency_id}`
- **TTL**: 24小时
- **Fallback**: 如果Redis不可用,从内存缓存读取

---

## Error Handling

此命令设计为**无错误抛出**,所有失败场景返回空Vec。

### 场景1: Redis连接失败

**行为**: 从内存缓存读取,如果内存也为空则返回 `[]`

**日志**:
```rust
tracing::warn!("Redis connection failed, falling back to memory cache");
```

### 场景2: 数据反序列化失败

**行为**: 跳过损坏的数据,返回可解析的结果

**日志**:
```rust
tracing::error!(
    dependency_id = %key,
    error = %err,
    "Failed to deserialize cached result, skipping"
);
```

---

## Test Cases

### 测试文件
`src-tauri/tests/contract_query_status.rs`

### 用例1: 查询所有依赖(缓存命中)

```rust
#[tokio::test]
async fn test_query_all_dependencies_from_cache() {
    let state = mock_state_with_cached_results(vec![
        ("nodejs", CheckStatus::Satisfied),
        ("redis", CheckStatus::Missing),
    ]);

    let results = query_dependency_status(None, state).await.unwrap();

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].dependency_id, "nodejs");
    assert_eq!(results[0].status, CheckStatus::Satisfied);
    assert_eq!(results[1].dependency_id, "redis");
    assert_eq!(results[1].status, CheckStatus::Missing);
}
```

### 用例2: 查询单个依赖(缓存命中)

```rust
#[tokio::test]
async fn test_query_single_dependency() {
    let state = mock_state_with_cached_results(vec![
        ("nodejs", CheckStatus::Satisfied),
        ("redis", CheckStatus::Missing),
    ]);

    let results = query_dependency_status(
        Some("nodejs".to_string()),
        state
    ).await.unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].dependency_id, "nodejs");
}
```

### 用例3: 查询不存在的依赖

```rust
#[tokio::test]
async fn test_query_nonexistent_dependency() {
    let state = mock_state_with_empty_cache();

    let results = query_dependency_status(
        Some("nonexistent".to_string()),
        state
    ).await.unwrap();

    assert_eq!(results.len(), 0);
}
```

### 用例4: 缓存为空

```rust
#[tokio::test]
async fn test_query_empty_cache() {
    let state = mock_state_with_empty_cache();

    let results = query_dependency_status(None, state).await.unwrap();

    assert_eq!(results.len(), 0);
}
```

---

## 日志记录

### 成功路径

```rust
tracing::debug!(
    dependency_id = ?dependency_id,
    result_count = %results.len(),
    "Query dependency status completed"
);
```

### Fallback路径

```rust
tracing::warn!(
    error = %err,
    "Failed to query Redis cache, using memory fallback"
);
```

---

## 性能要求

- **响应时间**: < 50ms (P95) - 纯缓存读取,无实际检测
- **并发支持**: 支持最多100个并发查询请求
- **缓存命中率**: > 95% (启动检测后24小时内)

---

## 与其他命令的关系

- **check_dependencies**: 生成缓存数据的来源
- **trigger_manual_check**: 更新缓存数据
- **前端轮询**: 可用于实时更新检测状态(不推荐,应使用事件流)

---

## 缓存数据结构

### Redis存储格式

```
Key: dep:check:nodejs
Value: {
  "dependency_id": "nodejs",
  "checked_at": "2025-10-05T10:30:15.123Z",
  "status": "satisfied",
  "detected_version": "20.10.0",
  "duration_ms": 45
}
TTL: 86400 seconds (24 hours)
```

### 内存存储格式

```rust
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct AppState {
    /// 内存缓存 (dependency_id -> CheckResult)
    pub check_cache: Arc<RwLock<HashMap<String, DependencyCheckResult>>>,
}
```

---

**契约版本**: 1.0.0
**最后更新**: 2025-10-05
**状态**: ✅ Ready for testing
