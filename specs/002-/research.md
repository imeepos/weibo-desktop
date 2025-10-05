# Phase 0: Technical Research - 启动时依赖检测与自动安装

**生成日期**: 2025-10-05
**研究范围**: 技术选型验证和最佳实践研究

---

## 1. Tauri启动生命周期集成

### 研究问题
如何在Tauri应用启动时无缝插入依赖检测流程,确保在主窗口展示前完成检测?

### 研究结论

**Decision**: 使用Tauri的`setup`钩子在应用启动early阶段触发依赖检测

**Rationale**:
- `tauri::Builder::setup()` 在窗口创建前执行,适合前置检查
- 可通过`app.handle()`访问应用实例,发送事件到前端
- 支持async/await,配合Tokio运行时执行异步检测任务

**实现方案**:
```rust
// src-tauri/src/main.rs
fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let app_handle = app.handle();
            tauri::async_runtime::spawn(async move {
                dependency_checker::run_startup_check(app_handle).await;
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

**Alternatives Considered**:
1. ❌ 在`main()`函数开头同步检测 - 会阻塞窗口显示
2. ❌ 使用Tauri Plugin系统 - 过度工程化,增加复杂性
3. ✅ 选择setup hook - 平衡启动时机和异步执行

---

## 2. Rust依赖检测模式

### 研究问题
如何在Rust中可靠地检测外部依赖(Node.js/Playwright/Redis/pnpm)的存在和版本?

### 研究结论

**Decision**: 组合使用`which` crate检测可执行文件 + `Command::output()`获取版本 + `semver` crate比较版本

**核心库选型**:

| Crate | 版本 | 用途 | 理由 |
|-------|------|------|------|
| `which` | 5.0+ | 查找PATH中的可执行文件 | 跨平台,处理Windows `.exe`后缀 |
| `semver` | 1.0+ | 版本号解析和比较 | 标准语义化版本支持 |
| `tokio::process::Command` | (Tokio内置) | 执行外部命令获取版本 | 异步执行,不阻塞主线程 |

**检测模式示例**:
```rust
use which::which;
use semver::Version;
use tokio::process::Command;

async fn check_node_version(required: &str) -> Result<CheckStatus, DependencyError> {
    // 1. 检测可执行文件存在性
    let node_path = which("node").map_err(|_| DependencyError::NotFound)?;

    // 2. 异步获取版本号
    let output = Command::new(node_path)
        .arg("--version")
        .output()
        .await?;

    let version_str = String::from_utf8(output.stdout)?
        .trim()
        .trim_start_matches('v');

    // 3. semver版本比较
    let detected = Version::parse(version_str)?;
    let required = Version::parse(required)?;

    if detected >= required {
        Ok(CheckStatus::Satisfied)
    } else {
        Ok(CheckStatus::VersionMismatch)
    }
}
```

**Alternatives Considered**:
1. ❌ 直接使用`std::process::Command` - 同步阻塞,不适合启动流程
2. ❌ 手动解析PATH环境变量 - 平台差异大,易出错
3. ✅ which + semver组合 - 成熟库,错误处理完善

---

## 3. Tokio并发安装模式

### 研究问题
如何使用Tokio实现"必需依赖串行安装 + 可选依赖并行安装"的混合策略?

### 研究结论

**Decision**: 使用`JoinSet`管理可选依赖并行任务,使用async顺序执行必需依赖

**架构设计**:
```rust
use tokio::task::JoinSet;

async fn install_dependencies(
    required: Vec<Dependency>,
    optional: Vec<Dependency>,
) -> Result<Vec<InstallationTask>, DependencyError> {
    let mut results = Vec::new();

    // 1. 串行安装必需依赖(按优先级排序)
    for dep in required {
        let task = install_single_dependency(dep).await?;
        results.push(task);
        // 失败立即中断,不继续安装后续必需依赖
        if task.status == InstallStatus::Failed {
            return Err(DependencyError::CriticalInstallFailed);
        }
    }

    // 2. 并行安装可选依赖
    let mut join_set = JoinSet::new();
    for dep in optional {
        join_set.spawn(install_single_dependency(dep));
    }

    // 3. 收集并行结果(忽略失败,仅记录日志)
    while let Some(res) = join_set.join_next().await {
        match res {
            Ok(Ok(task)) => results.push(task),
            Ok(Err(e)) => tracing::warn!("Optional dependency install failed: {}", e),
            Err(e) => tracing::error!("Task panic: {}", e),
        }
    }

    Ok(results)
}
```

**Rationale**:
- 串行执行保证必需依赖按优先级顺序安装,前一个失败时不浪费资源继续安装
- JoinSet自动管理任务生命周期,支持错误传播
- 可选依赖并行执行,充分利用多核CPU和I/O并发

**Alternatives Considered**:
1. ❌ 全部串行 - 安装时间过长
2. ❌ 全部并行 - 无法保证必需依赖顺序
3. ✅ 混合策略 - 平衡速度和可靠性

---

## 4. Tauri事件流进度更新

### 研究问题
如何从Rust后台任务实时向React前端推送检测和安装进度?

### 研究结论

**Decision**: 使用Tauri Event系统,通过`AppHandle::emit_all()`广播进度事件

**事件Payload Schema**:
```typescript
// 前端TypeScript类型定义
interface DependencyCheckProgressEvent {
    dependency_id: string;
    dependency_name: string;
    status: 'checking' | 'satisfied' | 'missing' | 'version_mismatch';
    detected_version?: string;
    current_index: number;
    total_count: number;
}

interface InstallationProgressEvent {
    task_id: string;
    dependency_id: string;
    progress_percent: number;
    status: 'pending' | 'downloading' | 'installing' | 'success' | 'failed';
    error_message?: string;
}
```

**后端发送示例**:
```rust
use tauri::Manager;

async fn check_dependency_with_progress(
    app: AppHandle,
    dep: Dependency,
    index: usize,
    total: usize,
) -> DependencyCheckResult {
    // 发送检测开始事件
    app.emit_all("dependency-check-progress", ProgressPayload {
        dependency_id: dep.name.clone(),
        status: "checking",
        current_index: index,
        total_count: total,
    }).ok();

    let result = perform_check(&dep).await;

    // 发送检测完成事件
    app.emit_all("dependency-check-progress", ProgressPayload {
        dependency_id: dep.name.clone(),
        status: result.status.to_string(),
        detected_version: result.detected_version.clone(),
        current_index: index,
        total_count: total,
    }).ok();

    result
}
```

**前端订阅示例**:
```typescript
import { listen } from '@tauri-apps/api/event';

useEffect(() => {
    const unlisten = listen<DependencyCheckProgressEvent>(
        'dependency-check-progress',
        (event) => {
            setProgress(event.payload.current_index / event.payload.total_count * 100);
            setCurrentDep(event.payload.dependency_name);
        }
    );
    return () => { unlisten.then(f => f()); };
}, []);
```

**Alternatives Considered**:
1. ❌ 轮询Tauri Command查询进度 - 低效,增加CPU开销
2. ❌ WebSocket - 过度工程化,Tauri已提供事件系统
3. ✅ Tauri Event - 原生支持,零额外依赖

---

## 5. 日志持久化策略

### 研究问题
如何集成tracing框架实现结构化日志永久保留到文件?

### 研究结论

**Decision**: 使用`tracing-appender` + `tracing-subscriber`组合,JSON格式写入应用数据目录

**依赖配置**:
```toml
[dependencies]
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["json", "env-filter"] }
tracing-appender = "0.2"
```

**日志初始化**:
```rust
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use tracing_appender::rolling;

fn init_logging() -> Result<(), Box<dyn std::error::Error>> {
    let app_data_dir = tauri::api::path::app_data_dir(&config)?;
    let log_dir = app_data_dir.join("logs");
    std::fs::create_dir_all(&log_dir)?;

    // 按日期滚动文件,文件名: dependency_check_2025-10-05.log
    let file_appender = rolling::daily(log_dir, "dependency_check");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .json() // JSON格式便于后续解析
                .with_writer(non_blocking)
        )
        .with(tracing_subscriber::EnvFilter::new("info")) // 默认INFO级别
        .init();

    Ok(())
}
```

**日志文件位置**:
- Windows: `C:\Users\<user>\AppData\Roaming\<app-name>\logs\`
- macOS: `~/Library/Application Support/<app-name>/logs/`
- Linux: `~/.local/share/<app-name>/logs/`

**日志格式示例**:
```json
{
    "timestamp": "2025-10-05T10:30:15.123Z",
    "level": "INFO",
    "target": "dependency_checker",
    "fields": {
        "message": "Dependency check completed",
        "dependency": "node",
        "status": "satisfied",
        "version": "20.10.0"
    }
}
```

**Rationale**:
- JSON格式便于后续分析和搜索
- non_blocking避免日志I/O阻塞检测流程
- 按日期滚动,自然组织日志文件(用户提示不自动清理,符合需求)

**Alternatives Considered**:
1. ❌ 纯文本日志 - 难以结构化查询
2. ❌ 数据库日志 - 过度工程化,增加依赖
3. ✅ JSON文件 - 平衡可读性和可解析性

---

## 6. 现有依赖项识别

### 研究任务
分析当前项目依赖Playwright/Node.js/Redis/pnpm的版本要求

### 研究结论

**初始依赖清单** (基于项目文件分析):

| 依赖名称 | 最低版本 | 级别 | 检测方法 | 可自动安装 | 优先级 |
|---------|---------|------|---------|----------|--------|
| Node.js | 20.0.0 | Required | 可执行文件(`node --version`) | ❌ | 1 |
| pnpm | 8.0.0 | Required | 可执行文件(`pnpm --version`) | ✅ (npm i -g pnpm) | 2 |
| Rust | 1.75.0 | Required | 可执行文件(`rustc --version`) | ❌ | 1 |
| Redis Server | 7.0.0 | Required | 服务端口(6379)或可执行文件 | ❌ | 3 |
| Playwright | 1.40.0 | Optional | Node模块(`playwright/package.json`) | ✅ (pnpm install) | 5 |
| Tauri CLI | 1.5.0 | Required | 可执行文件(`tauri --version`) | ✅ (cargo install tauri-cli) | 4 |

**数据来源**:
- `package.json`: Node.js引擎要求 `"engines": { "node": ">=20.0.0" }`
- `Cargo.toml`: Rust版本 `rust-version = "1.75"`
- `.playwright-docker.yml`: Redis镜像 `redis:7-alpine`
- `playwright/package.json`: Playwright版本 `"playwright": "^1.40.0"`

**安装指引准备**:
- Node.js: 引导用户访问 https://nodejs.org/zh-cn/download/
- Redis: 提供Docker命令或平台安装链接
- Rust: 引导用户访问 https://www.rust-lang.org/zh-CN/tools/install

---

## 研究总结

### 技术栈验证

✅ **所有选型与现有技术栈吻合**:
- 复用Tauri 1.x架构,无需新增框架
- 复用Tokio异步运行时(Tauri内置)
- 复用tracing日志库(项目已使用)
- 前端继续使用React + TailwindCSS

### 无新增核心依赖

仅添加工具类crate,均为Rust生态标准库:
- `which`: 5.0+ (查找可执行文件)
- `semver`: 1.0+ (版本比较)
- `tracing-appender`: 0.2 (日志文件输出)

### 风险评估

**低风险**:
- Tauri Event系统成熟稳定
- which/semver crate广泛使用(下载量百万级)
- 并发模型简单,无复杂锁竞争

**已缓解**:
- 依赖检测失败不影响应用后续启动(仅阻塞主界面进入)
- 安装失败回退到手动指引,无死锁风险

### 下一步

进入Phase 1设计阶段:
1. 编写`data-model.md` - 详细定义Rust结构体
2. 生成`contracts/` - 定义Tauri Commands签名
3. 编写`quickstart.md` - 集成测试脚本
4. 更新`CLAUDE.md` - 记录新功能上下文

---

**研究完成日期**: 2025-10-05
**研究者**: Claude Code Agent
**审查状态**: ✅ 通过Constitution Check,无复杂性违规
