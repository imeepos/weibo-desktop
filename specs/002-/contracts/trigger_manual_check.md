# Contract: trigger_manual_check

**Tauri Command**: `trigger_manual_check`
**Purpose**: 手动触发依赖检测,强制重新检测所有依赖项
**Feature**: 002-dependency-check

---

## 契约概述

允许用户在应用运行期间手动触发完整的依赖检测流程。与启动时的自动检测(`check_dependencies`)功能相同,但强制忽略缓存,始终执行实时检测。

**关键区别**:
- `check_dependencies`: 启动时自动调用,可能使用缓存结果(24小时TTL)
- `trigger_manual_check`: 用户手动触发,强制重新检测,更新缓存

---

## Request

### 参数

**无输入参数** - 检测所有已配置的依赖项

### Rust函数签名

```rust
#[tauri::command]
async fn trigger_manual_check(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<DependencyCheckResult>, DependencyError>
```

### TypeScript调用

```typescript
import { invoke } from '@tauri-apps/api/tauri';

// 无需传参
const results = await invoke<DependencyCheckResult[]>('trigger_manual_check');
console.log(`检测完成,共${results.length}个依赖`);
```

---

## Response

### 成功响应

返回所有依赖项的最新检测结果数组。

```rust
Vec<DependencyCheckResult>
```

参考 `data-model.md` → DependencyCheckResult定义:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyCheckResult {
    pub dependency_id: String,
    pub checked_at: DateTime<Utc>,
    pub status: CheckStatus,
    pub detected_version: Option<String>,
    pub error_details: Option<String>,
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

### 示例响应

```json
[
  {
    "dependency_id": "nodejs",
    "checked_at": "2025-10-05T14:30:25.123Z",
    "status": "satisfied",
    "detected_version": "20.10.0",
    "duration_ms": 45
  },
  {
    "dependency_id": "redis",
    "checked_at": "2025-10-05T14:30:25.200Z",
    "status": "satisfied",
    "detected_version": "7.2.4",
    "duration_ms": 12
  },
  {
    "dependency_id": "playwright",
    "checked_at": "2025-10-05T14:30:25.350Z",
    "status": "missing",
    "error_details": "Executable 'playwright' not found in PATH",
    "duration_ms": 28
  }
]
```

---

## Events

### dependency-check-progress

手动检测过程中,每完成一个依赖项的检测就emit一次进度事件。

**Event Payload**:
```typescript
interface DependencyCheckProgress {
  current_index: number;      // 当前检测项索引(1-based)
  total_count: number;         // 总依赖项数量
  dependency_id: string;       // 当前检测的依赖ID
  dependency_name: string;     // 当前检测的依赖名称
  status: CheckStatus;         // 检测结果状态
}
```

**前端监听**:
```typescript
import { listen } from '@tauri-apps/api/event';

const unlisten = await listen<DependencyCheckProgress>('dependency-check-progress', (event) => {
  const { current_index, total_count, dependency_name, status } = event.payload;

  console.log(`[${current_index}/${total_count}] ${dependency_name}: ${status}`);

  // 更新UI进度条
  const progress = (current_index / total_count) * 100;
  updateProgressBar(progress);
});
```

**事件时序示例** (3个依赖):
```
1. current_index=1, total_count=3, dependency_id="nodejs", status="satisfied"
2. current_index=2, total_count=3, dependency_id="redis", status="satisfied"
3. current_index=3, total_count=3, dependency_id="playwright", status="missing"
→ 返回完整结果数组
```

---

## Error Handling

### DependencyError::CheckFailed

**触发条件**: 检测过程中发生系统级错误(如配置文件损坏、权限问题)

**错误响应**:
```json
{
  "error": "DependencyError::CheckFailed",
  "message": "Failed to load dependency configuration: permission denied"
}
```

**前端处理**: 显示错误提示,建议检查应用权限或重新安装应用

---

## Behavior Specification

### 强制重新检测

- **忽略缓存**: 即使Redis中存在有效缓存(24小时TTL),也必须重新执行实际检测
- **更新缓存**: 检测完成后,将新结果写入Redis,覆盖旧缓存
- **时间戳**: 所有结果的`checked_at`必须是本次检测的实际时间,不能复用旧时间戳

### 并发执行

- 与`check_dependencies`共享相同的检测逻辑
- 多个依赖项的检测可以并行执行(通过Tokio spawn)
- 每完成一个依赖,立即emit进度事件,不等待全部完成

### 非阻塞UI

- 手动检测在异步任务中执行,不阻塞主线程
- 前端通过事件监听实时更新UI,不需要轮询
- 检测期间用户仍可操作其他界面元素

---

## Test Cases

### 测试文件
`src-tauri/tests/contract_manual_check.rs`

### 用例1: 所有依赖满足

```rust
#[tokio::test]
async fn test_manual_check_all_satisfied() {
    // Mock: nodejs=20.10.0, redis=7.2.4, playwright已安装
    mock_all_dependencies_satisfied();

    let results = trigger_manual_check(
        mock_app_handle(),
        mock_state()
    ).await.unwrap();

    assert_eq!(results.len(), 3);
    assert!(results.iter().all(|r| r.status == CheckStatus::Satisfied));

    // 验证时间戳是新的(最近1秒内)
    let now = Utc::now();
    for result in &results {
        let duration = now.signed_duration_since(result.checked_at);
        assert!(duration.num_seconds() < 1);
    }
}
```

### 用例2: 部分依赖缺失

```rust
#[tokio::test]
async fn test_manual_check_partial_missing() {
    // Mock: nodejs和redis已安装, playwright缺失
    mock_nodejs_redis_satisfied();
    mock_playwright_missing();

    let results = trigger_manual_check(
        mock_app_handle(),
        mock_state()
    ).await.unwrap();

    assert_eq!(results.len(), 3);

    let nodejs_result = results.iter().find(|r| r.dependency_id == "nodejs").unwrap();
    assert_eq!(nodejs_result.status, CheckStatus::Satisfied);

    let playwright_result = results.iter().find(|r| r.dependency_id == "playwright").unwrap();
    assert_eq!(playwright_result.status, CheckStatus::Missing);
    assert!(playwright_result.error_details.is_some());
}
```

### 用例3: 验证进度事件

```rust
#[tokio::test]
async fn test_manual_check_emits_progress_events() {
    let (tx, mut rx) = tokio::sync::mpsc::channel(10);

    // Mock事件监听器
    mock_event_listener("dependency-check-progress", tx);

    let results = trigger_manual_check(
        mock_app_handle(),
        mock_state()
    ).await.unwrap();

    // 验证收到3个进度事件
    let mut events = vec![];
    while let Ok(event) = rx.try_recv() {
        events.push(event);
    }

    assert_eq!(events.len(), 3);

    // 验证事件顺序
    assert_eq!(events[0].current_index, 1);
    assert_eq!(events[0].total_count, 3);
    assert_eq!(events[1].current_index, 2);
    assert_eq!(events[2].current_index, 3);
}
```

### 用例4: 强制忽略缓存

```rust
#[tokio::test]
async fn test_manual_check_ignores_cache() {
    // 第一次检测: nodejs版本20.10.0
    mock_nodejs_version("20.10.0");
    let first_results = trigger_manual_check(
        mock_app_handle(),
        mock_state()
    ).await.unwrap();

    let first_checked_at = first_results[0].checked_at;

    // 模拟版本升级
    tokio::time::sleep(Duration::from_millis(100)).await;
    mock_nodejs_version("20.11.0");

    // 第二次检测: 应该检测到新版本
    let second_results = trigger_manual_check(
        mock_app_handle(),
        mock_state()
    ).await.unwrap();

    let nodejs_result = second_results.iter().find(|r| r.dependency_id == "nodejs").unwrap();

    // 验证检测到新版本
    assert_eq!(nodejs_result.detected_version, Some("20.11.0".to_string()));

    // 验证时间戳更新
    assert!(nodejs_result.checked_at > first_checked_at);
}
```

### 用例5: 版本不匹配

```rust
#[tokio::test]
async fn test_manual_check_version_mismatch() {
    // Mock: nodejs安装了19.0.0,但要求>=20.0.0
    mock_nodejs_version("19.0.0");
    mock_version_requirement("nodejs", ">=20.0.0");

    let results = trigger_manual_check(
        mock_app_handle(),
        mock_state()
    ).await.unwrap();

    let nodejs_result = results.iter().find(|r| r.dependency_id == "nodejs").unwrap();

    assert_eq!(nodejs_result.status, CheckStatus::VersionMismatch);
    assert_eq!(nodejs_result.detected_version, Some("19.0.0".to_string()));
    assert!(nodejs_result.error_details.as_ref().unwrap().contains("requires >=20.0.0"));
}
```

---

## 日志记录

### 成功路径

```rust
tracing::info!(
    "Manual dependency check triggered by user"
);

tracing::info!(
    dependency_id = %dep_id,
    status = ?result.status,
    duration_ms = %result.duration_ms,
    "Dependency checked"
);

tracing::info!(
    total_count = %results.len(),
    satisfied_count = %satisfied,
    missing_count = %missing,
    "Manual check completed"
);
```

### 失败路径

```rust
tracing::error!(
    error = %err,
    "Manual dependency check failed"
);
```

---

## 性能要求

- **总体耗时**: 所有依赖检测完成 < 2秒 (P95)
- **单个依赖**: 检测耗时 < 500ms (P95)
- **事件延迟**: 完成检测后emit事件延迟 < 10ms
- **缓存更新**: 结果写入Redis < 50ms

---

## 与check_dependencies的对比

| 特性 | check_dependencies | trigger_manual_check |
|------|-------------------|---------------------|
| 调用时机 | 应用启动时自动 | 用户手动触发 |
| 缓存策略 | 可使用24小时缓存 | 强制重新检测 |
| 结果更新 | 写入缓存(如无缓存) | 强制更新缓存 |
| UI阻塞 | 阻塞启动流程 | 不阻塞主界面 |
| 事件发送 | 是 | 是 |
| 返回值 | Vec<DependencyCheckResult> | Vec<DependencyCheckResult> |

---

**契约版本**: 1.0.0
**创建日期**: 2025-10-05
**状态**: ✅ Ready for testing
