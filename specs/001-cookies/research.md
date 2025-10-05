# Research: 微博扫码登录获取Cookies

**Date**: 2025-10-05
**Feature**: 001-cookies
**Purpose**: 解决技术实现路径的关键决策点

---

## 微博扫码登录API调用模式

### Decision
使用微博开放平台的扫码登录接口,包含三个核心API:
1. **生成二维码**: `GET https://api.weibo.com/oauth2/qrcode/generate`
2. **轮询状态**: `GET https://api.weibo.com/oauth2/qrcode/status?qrcode_id={id}`
3. **获取Cookies**: 通过状态API返回的授权码exchange cookies

### Rationale
- 微博官方API提供稳定的扫码登录能力,避免模拟浏览器行为的不稳定性
- 状态轮询机制简单可靠,适合异步处理
- 返回的cookies包含完整的会话凭证,可用于后续API调用

### Alternatives
- **逆向工程微博网页登录流程**: 不稳定,易受页面改版影响,违反服务条款
- **使用第三方登录SDK**: 引入额外依赖,可能有安全风险,文档支持不足
- **Selenium模拟用户登录**: 资源占用高,速度慢,难以处理验证码

### Implementation Notes
**API端点** (需实际测试确认):
```rust
const QRCODE_GENERATE_URL: &str = "https://api.weibo.com/oauth2/qrcode/generate";
const QRCODE_STATUS_URL: &str = "https://api.weibo.com/oauth2/qrcode/status";
```

**请求参数**:
- `client_id`: 应用ID (需在微博开放平台注册)
- `redirect_uri`: 回调地址
- `scope`: 权限范围 (如 `all`)

**轮询策略**:
- 初始间隔: 2秒
- 最大间隔: 5秒
- 使用exponential backoff避免API限流
- 二维码有效期: 180秒 (由API返回)

**错误码处理**:
- `21327`: 二维码已过期 → 提示重新生成
- `21328`: 二维码未扫描 → 继续轮询
- `21329`: 已扫描待确认 → 更新UI状态
- `21330`: 已确认 → 获取cookies

---

## Tauri与Playwright集成方案

### Decision
使用 **Tauri Sidecar** 机制打包Playwright Node.js脚本作为独立可执行文件,通过 `Command::new()` 调用并解析stdout输出。

### Rationale
- **Sidecar模式**: Tauri原生支持打包外部可执行文件,跨平台兼容性好
- **进程隔离**: Playwright运行在独立进程,崩溃不影响主应用
- **标准输出通信**: 简单可靠,无需IPC复杂性
- **打包便捷**: Tauri自动处理平台差异 (Windows .exe, Linux/macOS binary)

### Alternatives
- **Node.js HTTP Server**: Playwright启动HTTP服务,Tauri通过HTTP调用
  - **缺点**: 需要端口管理,增加网络层复杂度,安全风险
- **FFI (Foreign Function Interface)**: 通过neon或napi绑定Node.js
  - **缺点**: 编译复杂,跨平台兼容性差,维护成本高
- **WebSocket IPC**: 双向通信通道
  - **缺点**: 过度设计,简单验证任务不需要持久连接

### Implementation Notes
**Tauri配置** (`tauri.conf.json`):
```json
{
  "tauri": {
    "bundle": {
      "externalBin": ["playwright/validate-cookies"]
    }
  }
}
```

**Rust调用代码**:
```rust
use tauri::api::process::{Command, CommandEvent};

async fn validate_cookies(cookies: &str) -> Result<bool, String> {
    let (mut rx, _child) = Command::new_sidecar("validate-cookies")?
        .args(&["--cookies", cookies])
        .spawn()?;

    while let Some(event) = rx.recv().await {
        if let CommandEvent::Stdout(line) = event {
            // 解析JSON输出
            let result: ValidationResult = serde_json::from_str(&line)?;
            return Ok(result.valid);
        }
    }
    Err("No output from sidecar".to_string())
}
```

**Playwright脚本结构** (`playwright/validate-cookies.js`):
```javascript
#!/usr/bin/env node
const { chromium } = require('playwright');

async function validateCookies(cookiesJson) {
  const browser = await chromium.launch();
  const context = await browser.newContext();
  await context.addCookies(JSON.parse(cookiesJson));

  const page = await context.newPage();
  const response = await page.goto('https://weibo.com/ajax/profile/info');
  const valid = response.ok();

  await browser.close();
  console.log(JSON.stringify({ valid, status: response.status() }));
}

const cookies = process.argv[2];
validateCookies(cookies).catch(console.error);
```

**打包为可执行文件**:
使用 `pkg` 或 `nexe` 将Node.js脚本打包:
```bash
pkg playwright/validate-cookies.js --targets node18-linux-x64,node18-macos-x64,node18-win-x64
```

---

## Rust异步模式与Tauri Commands

### Decision
所有Tauri commands使用 `async fn`,依赖Tokio runtime,避免阻塞主线程。长时间操作 (如轮询) 使用 `tokio::spawn` 独立任务,通过 `tauri::Window::emit` 发送事件到前端。

### Rationale
- **非阻塞**: Tauri默认集成Tokio,async commands自动在async runtime执行
- **响应性**: UI主线程不被阻塞,保持60fps流畅度
- **事件驱动**: `Window::emit` 实现服务端推送,前端监听状态变化
- **取消支持**: `tokio::spawn` 返回 `JoinHandle`,可实现取消轮询功能

### Alternatives
- **同步commands + 线程池**: 使用 `std::thread::spawn`
  - **缺点**: 线程创建开销大,难以取消,资源占用高
- **回调函数传递**: 通过closure回传结果
  - **缺点**: Rust lifetime复杂,跨线程传递困难
- **轮询在前端实现**: 前端定时调用 `poll_login_status`
  - **缺点**: 网络往返开销大,状态管理复杂,无法统一日志

### Implementation Notes
**Async Command示例**:
```rust
#[tauri::command]
async fn generate_qrcode(state: tauri::State<'_, AppState>) -> Result<QrCodeResponse, ApiError> {
    let client = &state.http_client;
    let response = client
        .get(QRCODE_GENERATE_URL)
        .query(&[("client_id", &state.config.client_id)])
        .send()
        .await?
        .json::<QrCodeResponse>()
        .await?;

    tracing::info!(qr_id = %response.qr_id, "QR code generated");
    Ok(response)
}
```

**后台轮询任务**:
```rust
#[tauri::command]
async fn start_polling(
    qr_id: String,
    window: tauri::Window,
    state: tauri::State<'_, AppState>
) -> Result<(), String> {
    tokio::spawn(async move {
        let mut interval = 2;
        loop {
            tokio::time::sleep(Duration::from_secs(interval)).await;

            match poll_status(&qr_id, &state).await {
                Ok(status) => {
                    window.emit("login-status", &status).unwrap();
                    if status.is_final() {
                        break;
                    }
                }
                Err(e) => {
                    tracing::error!(error = ?e, "Poll failed");
                    break;
                }
            }

            interval = (interval + 1).min(5); // exponential backoff
        }
    });
    Ok(())
}
```

**前端事件监听** (TypeScript):
```typescript
import { listen } from '@tauri-apps/api/event';

listen<LoginStatus>('login-status', (event) => {
  setStatus(event.payload.status);
  if (event.payload.status === 'confirmed') {
    setCookies(event.payload.cookies);
  }
});
```

---

## Redis连接池管理

### Decision
使用 `deadpool-redis` crate实现连接池,配置 `max_size: 10`, `min_idle: 2`,超时5秒,支持自动重连和健康检查。

### Rationale
- **成熟稳定**: `deadpool` 是Rust生态中广泛使用的连接池实现
- **异步原生**: 完全基于Tokio,与Tauri async commands无缝集成
- **资源高效**: 连接复用避免频繁TCP握手,减少Redis服务器负载
- **自动恢复**: 连接失效时自动重建,无需手动处理

### Alternatives
- **redis crate原生连接**: 每次操作创建新连接
  - **缺点**: TCP握手开销大,高并发下性能差,无连接复用
- **bb8-redis**: 另一个连接池库
  - **缺点**: 社区支持不如deadpool,配置选项较少
- **r2d2-redis**: 同步连接池
  - **缺点**: 不支持async/await,需要blocking_task包装,性能差

### Implementation Notes
**依赖配置** (`Cargo.toml`):
```toml
[dependencies]
deadpool-redis = { version = "0.14", features = ["rt_tokio_1"] }
redis = { version = "0.24", features = ["tokio-comp", "connection-manager"] }
```

**连接池初始化**:
```rust
use deadpool_redis::{Config, Runtime, Pool};

pub fn create_redis_pool(redis_url: &str) -> Pool {
    let cfg = Config::from_url(redis_url);
    cfg.create_pool(Some(Runtime::Tokio1))
        .expect("Failed to create Redis pool")
}

// 在AppState中存储
pub struct AppState {
    pub redis_pool: Pool,
    // ...
}
```

**连接使用示例**:
```rust
async fn save_to_redis(
    pool: &Pool,
    uid: &str,
    cookies: &HashMap<String, String>
) -> Result<(), StorageError> {
    let mut conn = pool.get().await.map_err(|e| StorageError::PoolError(e))?;

    let key = format!("weibo:cookies:{}", uid);
    let json = serde_json::to_string(cookies)?;

    redis::cmd("SET")
        .arg(&key)
        .arg(&json)
        .query_async(&mut conn)
        .await
        .map_err(|e| StorageError::RedisError(e))?;

    tracing::debug!(uid = %uid, key = %key, "Cookies saved to Redis");
    Ok(())
}
```

**错误处理**:
```rust
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("Failed to get connection from pool: {0}")]
    PoolError(#[from] deadpool_redis::PoolError),

    #[error("Redis operation failed: {0}")]
    RedisError(#[from] redis::RedisError),

    #[error("Cookies not found for uid: {0}")]
    NotFound(String),
}
```

**健康检查**:
```rust
async fn check_redis_health(pool: &Pool) -> bool {
    match pool.get().await {
        Ok(mut conn) => redis::cmd("PING").query_async::<_, String>(&mut conn).await.is_ok(),
        Err(_) => false,
    }
}
```

---

## 结构化日志方案

### Decision
使用 `tracing` + `tracing-subscriber` + `tracing-appender`,配置:
- **输出**: 同时输出到stdout (开发) 和文件 (生产)
- **格式**: JSON格式结构化日志
- **Rotation**: 文件大小100MB自动滚动,保留最近10个文件
- **级别**: 环境变量 `RUST_LOG` 控制 (默认 `info`)

### Rationale
- **结构化**: tracing原生支持结构化字段,便于日志分析和搜索
- **性能**: 零成本抽象,编译期优化,运行时开销极小
- **可观测性**: span + event模型天然支持分布式追踪
- **生态集成**: 与Tokio完美集成,自动跟踪async task上下文

### Alternatives
- **log + env_logger**: Rust传统日志方案
  - **缺点**: 非结构化,难以解析,缺少上下文追踪
- **slog**: 另一个结构化日志库
  - **缺点**: API复杂,与async集成不佳,社区支持不如tracing
- **直接println!**: 最简单方案
  - **缺点**: 无级别控制,无格式化,无文件输出,生产不可用

### Implementation Notes
**依赖配置** (`Cargo.toml`):
```toml
[dependencies]
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["json", "env-filter"] }
tracing-appender = "0.2"
```

**日志初始化** (`logging.rs`):
```rust
use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use tracing_appender::{rolling, non_blocking};

pub fn init_logging() {
    // 文件输出 (每日滚动)
    let file_appender = rolling::daily("./logs", "weibo-login.log");
    let (non_blocking_file, _guard) = non_blocking(file_appender);

    // 环境变量控制级别,默认info
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    // 组合订阅器
    tracing_subscriber::registry()
        .with(env_filter)
        .with(
            fmt::layer()
                .with_writer(std::io::stdout)
                .with_target(true)
                .with_thread_ids(true)
        )
        .with(
            fmt::layer()
                .json()
                .with_writer(non_blocking_file)
        )
        .init();

    tracing::info!("Logging initialized");
}
```

**使用示例**:
```rust
use tracing::{info, warn, error, instrument};

#[instrument(skip(cookies))] // 自动记录函数入参和返回值
async fn validate_cookies(cookies: &HashMap<String, String>) -> Result<bool, ValidationError> {
    info!(cookies_count = cookies.len(), "Starting validation");

    match call_profile_api(cookies).await {
        Ok(response) if response.status().is_success() => {
            info!(uid = %response.uid, "Validation successful");
            Ok(true)
        }
        Ok(response) => {
            warn!(status = %response.status(), "Validation failed");
            Ok(false)
        }
        Err(e) => {
            error!(error = ?e, "API call failed");
            Err(ValidationError::ApiError(e))
        }
    }
}
```

**敏感数据脱敏**:
```rust
#[instrument(skip_all, fields(uid = %uid))] // skip_all避免记录cookies
async fn save_cookies(uid: &str, cookies: &HashMap<String, String>) {
    info!("Saving cookies");
    // cookies不会出现在日志中
}
```

**性能监控**:
```rust
use tracing::Instrument;

async fn expensive_operation() {
    async {
        // 操作内容
    }
    .instrument(tracing::info_span!("expensive_op"))
    .await;
}
// 日志会包含操作耗时
```

---

## 总结

所有技术决策遵循项目章程的五大原则:

1. **存在即合理**: 每个技术选择都有不可替代的理由,避免过度工程
2. **优雅即简约**: 方案简洁明了,代码自文档化
3. **性能即艺术**: 连接池、异步处理、日志优化都考虑性能影响
4. **错误处理如为人处世的哲学**: 完善的错误类型和优雅的降级策略
5. **日志是思想的表达**: 结构化日志记录关键路径,避免噪音

**下一步**: 基于研究成果,创建 Phase 1 设计文档 (data-model.md, contracts/, quickstart.md)。
