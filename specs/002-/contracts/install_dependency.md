# Contract: install_dependency

**Tauri Command**: `install_dependency`
**Purpose**: 在线自动安装单个可自动安装的依赖项
**Feature**: 002-dependency-check

---

## 契约概述

执行依赖项的自动在线安装,支持进度追踪和实时日志输出。仅对 `auto_installable=true` 的依赖有效。安装过程中通过事件流推送进度更新。

---

## Request

### 参数

```typescript
interface InstallDependencyRequest {
  /// 依赖项ID (如: "pnpm", "playwright-browsers")
  dependency_id: string;

  /// 是否强制重新安装 (即使当前已满足版本要求)
  force: boolean;
}
```

### Rust函数签名

```rust
#[tauri::command]
async fn install_dependency(
    dependency_id: String,
    force: bool,
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<InstallationTask, DependencyError>
```

### TypeScript调用

```typescript
import { invoke } from '@tauri-apps/api/tauri';

const task = await invoke<InstallationTask>('install_dependency', {
  dependency_id: 'pnpm',
  force: false
});
```

---

## Response

### 成功响应

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallationTask {
    /// 任务唯一ID
    pub task_id: String,  // UUID v4

    /// 关联的依赖项ID
    pub dependency_id: String,

    /// 任务创建时间(UTC)
    pub created_at: String,  // ISO 8601

    /// 开始安装时间
    pub started_at: Option<String>,

    /// 完成时间
    pub completed_at: Option<String>,

    /// 安装状态
    pub status: InstallStatus,

    /// 安装进度百分比(0-100)
    pub progress_percent: u8,

    /// 错误消息(失败时提供)
    pub error_message: Option<String>,

    /// 安装日志条目
    pub install_log: Vec<String>,

    /// 错误类型(失败时分类)
    pub error_type: Option<InstallErrorType>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallStatus {
    Pending,
    Downloading,
    Installing,
    Success,
    Failed,
}
```

### 示例响应

```json
{
  "task_id": "550e8400-e29b-41d4-a716-446655440000",
  "dependency_id": "pnpm",
  "created_at": "2025-10-05T10:35:00.000Z",
  "started_at": "2025-10-05T10:35:00.100Z",
  "completed_at": null,
  "status": "downloading",
  "progress_percent": 15,
  "error_message": null,
  "install_log": [
    "Starting installation of pnpm@8.10.0",
    "Downloading from registry.npmjs.org..."
  ],
  "error_type": null
}
```

---

## Events

### installation-progress

安装过程中定期发送进度更新事件 (每500ms或状态变化时触发)。

**Event Payload**:
```typescript
interface InstallationProgress {
  task_id: string;
  dependency_id: string;
  status: InstallStatus;
  progress_percent: number;
  log_entry?: string;  // 新增的日志条目
}
```

**前端监听**:
```typescript
import { listen } from '@tauri-apps/api/event';

const unlisten = await listen<InstallationProgress>('installation-progress', (event) => {
  const { task_id, status, progress_percent, log_entry } = event.payload;

  console.log(`Task ${task_id}: ${status} - ${progress_percent}%`);
  if (log_entry) {
    console.log(`  > ${log_entry}`);
  }
});
```

**事件时序**:
```
1. status=Downloading, progress=10  → "正在下载 pnpm@8.10.0"
2. status=Downloading, progress=50  → "下载完成: 5.2MB"
3. status=Installing,  progress=75  → "正在安装到全局目录"
4. status=Success,     progress=100 → "安装完成,版本验证通过"
```

---

## Error Handling

### DependencyError::NotAutoInstallable

**触发条件**: 尝试安装 `auto_installable=false` 的依赖

**错误响应**:
```json
{
  "error": "DependencyError::NotAutoInstallable",
  "message": "Dependency 'nodejs' cannot be auto-installed. Please install manually.",
  "dependency_id": "nodejs"
}
```

**前端处理**: 展示手动安装指引 (`Dependency.install_guide`)

---

### DependencyError::InstallFailed(InstallErrorType::NetworkError)

**触发条件**: 下载依赖包时网络超时、连接失败、DNS解析错误

**错误响应**:
```json
{
  "error": "DependencyError::InstallFailed",
  "error_type": "network_error",
  "message": "Download failed: connection timeout after 30s",
  "dependency_id": "pnpm",
  "task_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

**前端处理**: 提示检查网络连接,提供重试按钮

---

### DependencyError::InstallFailed(InstallErrorType::PermissionError)

**触发条件**: 安装目录需要管理员权限 (如写入 `/usr/local/bin`)

**错误响应**:
```json
{
  "error": "DependencyError::InstallFailed",
  "error_type": "permission_error",
  "message": "Permission denied: cannot write to /usr/local/bin",
  "dependency_id": "playwright-browsers",
  "task_id": "660e9500-f39c-52e5-b827-557766551111"
}
```

**前端处理**: 提示以管理员身份重新运行应用,或选择用户目录安装

---

### DependencyError::InstallFailed(InstallErrorType::DiskSpaceError)

**触发条件**: 磁盘剩余空间不足

**错误响应**:
```json
{
  "error": "DependencyError::InstallFailed",
  "error_type": "disk_space_error",
  "message": "Insufficient disk space: required 500MB, available 120MB",
  "dependency_id": "playwright-browsers",
  "task_id": "770f0600-g40d-63f6-c938-668877662222"
}
```

**前端处理**: 提示清理磁盘空间,并显示所需空间大小

---

### DependencyError::InstallFailed(InstallErrorType::VersionConflictError)

**触发条件**: 系统已存在不兼容的版本且无法覆盖

**错误响应**:
```json
{
  "error": "DependencyError::InstallFailed",
  "error_type": "version_conflict_error",
  "message": "Conflicting version detected: pnpm@7.0.0 is installed, required >=8.0.0",
  "dependency_id": "pnpm",
  "task_id": "880g1700-h51e-74g7-d049-779988773333"
}
```

**前端处理**: 提示手动卸载旧版本,提供卸载指引链接

---

### DependencyError::InstallFailed(InstallErrorType::UnknownError)

**触发条件**: 未分类的安装失败 (脚本执行异常、依赖关系解析失败)

**错误响应**:
```json
{
  "error": "DependencyError::InstallFailed",
  "error_type": "unknown_error",
  "message": "Installation failed: npm ERR! code ETARGET",
  "dependency_id": "pnpm",
  "task_id": "990h2800-i62f-85h8-e150-880099884444"
}
```

**前端处理**: 展示通用错误提示,提供完整日志查看入口

---

### DependencyError::AlreadySatisfied

**触发条件**: 依赖已满足且 `force=false`

**错误响应**:
```json
{
  "error": "DependencyError::AlreadySatisfied",
  "message": "Dependency 'pnpm' is already satisfied (version 8.10.0)",
  "dependency_id": "pnpm",
  "current_version": "8.10.0"
}
```

**前端处理**: 提示已安装,询问是否强制重新安装 (`force=true`)

---

## Test Cases

### 测试文件
`src-tauri/tests/contract_install_dependency.rs`

### 用例1: 成功安装pnpm

```rust
#[tokio::test]
async fn test_install_pnpm_success() {
    let task = install_dependency(
        "pnpm".to_string(),
        false,
        mock_app_handle(),
        mock_state()
    ).await.unwrap();

    assert_eq!(task.dependency_id, "pnpm");
    assert_eq!(task.status, InstallStatus::Pending);
    assert_eq!(task.progress_percent, 0);
    assert!(!task.install_log.is_empty());

    // 等待安装完成
    tokio::time::sleep(Duration::from_secs(10)).await;

    // 验证最终状态
    let final_task = get_task_status(&task.task_id).await.unwrap();
    assert_eq!(final_task.status, InstallStatus::Success);
    assert_eq!(final_task.progress_percent, 100);
}
```

### 用例2: 网络失败重试

```rust
#[tokio::test]
async fn test_install_network_failure() {
    // Mock网络超时
    mock_network_timeout();

    let result = install_dependency(
        "pnpm".to_string(),
        false,
        mock_app_handle(),
        mock_state()
    ).await;

    assert!(matches!(
        result,
        Err(DependencyError::InstallFailed(InstallErrorType::NetworkError))
    ));
}
```

### 用例3: 权限拒绝

```rust
#[tokio::test]
async fn test_install_permission_denied() {
    // Mock无权限写入目录
    mock_permission_error();

    let result = install_dependency(
        "playwright-browsers".to_string(),
        false,
        mock_app_handle(),
        mock_state()
    ).await;

    let err = result.unwrap_err();
    assert!(matches!(
        err,
        DependencyError::InstallFailed(InstallErrorType::PermissionError)
    ));
    assert!(err.to_string().contains("Permission denied"));
}
```

### 用例4: 版本冲突

```rust
#[tokio::test]
async fn test_install_version_conflict() {
    // Mock已存在pnpm@7.0.0
    mock_existing_version("pnpm", "7.0.0");

    let result = install_dependency(
        "pnpm".to_string(),
        false,
        mock_app_handle(),
        mock_state()
    ).await;

    let err = result.unwrap_err();
    assert!(matches!(
        err,
        DependencyError::InstallFailed(InstallErrorType::VersionConflictError)
    ));
}
```

### 用例5: 强制重新安装

```rust
#[tokio::test]
async fn test_force_reinstall() {
    // 依赖已满足
    mock_satisfied_dependency("pnpm", "8.10.0");

    // force=false → 返回AlreadySatisfied
    let result = install_dependency(
        "pnpm".to_string(),
        false,
        mock_app_handle(),
        mock_state()
    ).await;
    assert!(matches!(result, Err(DependencyError::AlreadySatisfied)));

    // force=true → 执行重新安装
    let task = install_dependency(
        "pnpm".to_string(),
        true,
        mock_app_handle(),
        mock_state()
    ).await.unwrap();
    assert_eq!(task.status, InstallStatus::Pending);
}
```

---

## 日志记录

### 成功路径

```rust
tracing::info!(
    task_id = %task.task_id,
    dependency_id = %dependency_id,
    "Installation started"
);

tracing::info!(
    task_id = %task.task_id,
    status = ?InstallStatus::Downloading,
    progress = %progress_percent,
    "Installation progress update"
);

tracing::info!(
    task_id = %task.task_id,
    duration_ms = %duration.as_millis(),
    "Installation completed successfully"
);
```

### 失败路径

```rust
tracing::error!(
    task_id = %task.task_id,
    dependency_id = %dependency_id,
    error_type = ?InstallErrorType::NetworkError,
    error = %error_message,
    "Installation failed"
);
```

---

## 性能要求

- **轻量级依赖** (如pnpm): < 30s (P95)
- **大型依赖** (如playwright-browsers): < 120s (P95)
- **进度更新频率**: 每500ms或状态变化时emit事件
- **超时设置**: 单个下载操作30s超时,总体安装120s超时
- **重试策略**: 网络失败时最多重试3次 (1s, 2s, 4s exponential backoff)

---

## 安全考虑

- 安装命令执行前验证 `dependency_id` 在白名单内,防止命令注入
- 下载源必须使用HTTPS,验证SSL证书
- 日志中不记录敏感环境变量 (如API tokens)
- 安装脚本执行时使用沙箱隔离 (Tauri Shell API安全模式)
