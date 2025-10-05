# Tasks: 微博扫码登录获取Cookies

**Feature**: 001-cookies | **Date**: 2025-10-05
**Purpose**: 可执行的任务清单,按TDD和依赖顺序组织

遵循章程原则:
- **存在即合理**: 每个任务都不可替代,无冗余
- **优雅即简约**: 任务描述自文档化,路径精确可执行
- **TDD优先**: 测试先于实现,契约定义接口

---

## 路径约定

所有路径均为**绝对路径**,相对于项目根目录 `/workspace/desktop`:

```
/workspace/desktop/
├── src-tauri/               # Rust后端
│   ├── src/
│   │   ├── main.rs
│   │   ├── commands/        # Tauri命令
│   │   ├── services/        # 业务逻辑
│   │   ├── models/          # 数据模型
│   │   ├── errors.rs
│   │   └── logging.rs
│   └── Cargo.toml
├── src/                     # React前端
│   ├── components/
│   ├── hooks/
│   └── services/
├── playwright/              # Node.js自动化
│   ├── validate-cookies.js
│   └── package.json
└── tests/                   # 测试
    ├── contract/
    └── integration/
```

**标记说明**:
- `[P]`: 可并行执行的任务 (无依赖或同级依赖)
- 无标记: 必须串行执行 (有依赖)

---

## Phase 1: 基础设施搭建

### T001 [P] 初始化Tauri项目结构
**描述**: 创建Tauri桌面应用骨架

**文件**:
- `/workspace/desktop/src-tauri/Cargo.toml`
- `/workspace/desktop/src-tauri/tauri.conf.json`
- `/workspace/desktop/src-tauri/src/main.rs`
- `/workspace/desktop/package.json`
- `/workspace/desktop/pnpm-workspace.yaml`

**实现要点**:
- Tauri版本: 1.5+
- 配置app名称: "Weibo Login Manager"
- 窗口尺寸: 800x600, 可调整大小
- 权限: 允许访问文件系统、网络

**验证**:
```bash
cd /workspace/desktop
pnpm install
cd src-tauri && cargo build
pnpm tauri dev  # 应成功启动空白窗口
```

---

### T002 [P] 配置Rust依赖
**描述**: 添加所有Rust crates到Cargo.toml

**文件**: `/workspace/desktop/src-tauri/Cargo.toml`

**依赖列表**:
```toml
[dependencies]
tauri = { version = "1.5", features = ["api-all"] }
tokio = { version = "1.35", features = ["full"] }
reqwest = { version = "0.11", features = ["json"] }
redis = { version = "0.24", features = ["tokio-comp", "connection-manager"] }
deadpool-redis = { version = "0.14", features = ["rt_tokio_1"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
chrono = { version = "0.4", features = ["serde"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["json", "env-filter"] }
tracing-appender = "0.2"
thiserror = "1.0"
base64 = "0.21"
```

**验证**:
```bash
cd /workspace/desktop/src-tauri
cargo check  # 应无错误
```

---

### T003 [P] 配置前端依赖
**描述**: 添加React、TailwindCSS和Tauri API依赖

**文件**:
- `/workspace/desktop/package.json`
- `/workspace/desktop/playwright/package.json`

**依赖列表**:
```json
{
  "dependencies": {
    "react": "^18.2.0",
    "react-dom": "^18.2.0",
    "@tauri-apps/api": "^1.5.0",
    "zustand": "^4.4.0"
  },
  "devDependencies": {
    "@types/react": "^18.2.0",
    "typescript": "^5.0.0",
    "vite": "^5.0.0",
    "tailwindcss": "^3.4.0",
    "autoprefixer": "^10.4.0",
    "postcss": "^8.4.0"
  }
}
```

**Playwright**:
```json
{
  "name": "weibo-cookies-validator",
  "dependencies": {
    "playwright": "^1.40.0"
  }
}
```

**验证**:
```bash
pnpm install
cd playwright && pnpm install
```

---

### T004 [P] 实现日志系统
**描述**: 配置tracing-subscriber,支持控制台和文件输出

**文件**: `/workspace/desktop/src-tauri/src/logging.rs`

**实现要点**:
- 双重输出: stdout (开发) + 文件 (生产)
- 文件路径: `./logs/weibo-login.log`
- JSON格式,结构化字段
- 环境变量控制级别: `RUST_LOG` (默认info)
- 每日文件滚动,保留最近10个文件

**核心代码**:
```rust
use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use tracing_appender::rolling;

pub fn init_logging() {
    let file_appender = rolling::daily("./logs", "weibo-login.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt::layer().with_writer(std::io::stdout))
        .with(fmt::layer().json().with_writer(non_blocking))
        .init();

    tracing::info!("Logging initialized");
}
```

**验证**:
- 启动应用,检查 `/workspace/desktop/logs/` 目录生成
- 日志文件包含JSON格式的启动日志

---

### T005 [P] 定义错误类型
**描述**: 创建ApiError、ValidationError、StorageError

**文件**: `/workspace/desktop/src-tauri/src/errors.rs`

**实现要点**:
- 使用 `thiserror` 派生宏
- 每个错误包含有意义的上下文
- 错误可序列化为JSON (Serde)
- 参考 `/workspace/desktop/specs/001-cookies/data-model.md` 错误定义

**错误类型**:
```rust
#[derive(Debug, Error, Serialize)]
pub enum ApiError {
    #[error("Network request failed: {0}")]
    NetworkFailed(String),

    #[error("Invalid API response format")]
    InvalidResponse,

    #[error("QR code expired")]
    QrCodeExpired {
        generated_at: DateTime<Utc>,
        expired_at: DateTime<Utc>,
    },

    #[error("QR code not found: {qr_id}")]
    QrCodeNotFound { qr_id: String },

    #[error("API rate limit exceeded (retry after {retry_after}s)")]
    RateLimitExceeded { retry_after: u64 },
}

#[derive(Debug, Error, Serialize)]
pub enum ValidationError {
    // ... (参考data-model.md)
}

#[derive(Debug, Error, Serialize)]
pub enum StorageError {
    // ... (参考data-model.md)
}
```

**验证**:
```bash
cargo build  # 应无错误
```

---

## Phase 2: 数据模型 (TDD)

### T006 [P] 实现LoginSession模型
**描述**: 登录会话模型 + 验证规则 + 单元测试

**文件**: `/workspace/desktop/src-tauri/src/models/login_session.rs`

**实现要点**:
- 参考 `/workspace/desktop/specs/001-cookies/data-model.md`
- 包含 `QrCodeStatus` 枚举
- 实现 `validate()`, `is_expired()`, `is_final_status()` 方法
- 时间戳递增验证

**单元测试**:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_success() {
        let session = LoginSession {
            qr_id: "qr_123".into(),
            status: QrCodeStatus::Pending,
            created_at: Utc::now(),
            scanned_at: None,
            confirmed_at: None,
            expires_at: Utc::now() + Duration::seconds(180),
        };
        assert!(session.validate().is_ok());
    }

    #[test]
    fn test_validate_empty_qr_id() {
        let mut session = valid_session();
        session.qr_id = "".into();
        assert!(session.validate().is_err());
    }

    #[test]
    fn test_is_expired() {
        let mut session = valid_session();
        session.expires_at = Utc::now() - Duration::seconds(1);
        assert!(session.is_expired());
    }
}
```

**验证**:
```bash
cargo test models::login_session
```

---

### T007 [P] 实现CookiesData模型
**描述**: Cookies数据模型 + 验证规则 + 单元测试

**文件**: `/workspace/desktop/src-tauri/src/models/cookies_data.rs`

**实现要点**:
- HashMap存储cookies
- 必需字段验证 (SUB, SUBP)
- `to_cookie_header()`: 生成HTTP Cookie header
- `sample_for_logging()`: 脱敏日志采样

**单元测试**:
```rust
#[test]
fn test_validate_success() {
    let mut cookies = HashMap::new();
    cookies.insert("SUB".into(), "token".into());
    cookies.insert("SUBP".into(), "subp".into());

    let data = CookiesData::new("123".into(), cookies);
    assert!(data.validate().is_ok());
}

#[test]
fn test_validate_missing_required_cookie() {
    let mut cookies = HashMap::new();
    cookies.insert("SUB".into(), "token".into());
    // 缺少SUBP

    let data = CookiesData::new("123".into(), cookies);
    assert!(matches!(
        data.validate(),
        Err(ValidationError::MissingCookie { .. })
    ));
}

#[test]
fn test_to_cookie_header() {
    let data = create_test_cookies();
    let header = data.to_cookie_header();
    assert!(header.contains("SUB=token"));
    assert!(header.contains("SUBP=subp"));
}
```

**验证**:
```bash
cargo test models::cookies_data
```

---

### T008 [P] 实现LoginEvent模型
**描述**: 登录事件模型 + 辅助构造函数 + 单元测试

**文件**: `/workspace/desktop/src-tauri/src/models/login_event.rs`

**实现要点**:
- `LoginEventType` 枚举 (8种事件)
- 辅助函数: `qrcode_generated()`, `validation_failed()`, `api_error()`
- details字段使用 `serde_json::Value` 灵活扩展

**单元测试**:
```rust
#[test]
fn test_qrcode_generated_event() {
    let event = LoginEvent::qrcode_generated(
        "session_1".into(),
        "qr_123".into(),
        180
    );

    assert_eq!(event.event_type, LoginEventType::Generated);
    assert_eq!(event.session_id, "session_1");
    assert!(event.details.get("qr_id").is_some());
}

#[test]
fn test_validation_failed_event() {
    let error = ValidationError::EmptyCookies;
    let event = LoginEvent::validation_failed(
        "session_1".into(),
        "123".into(),
        &error
    );

    assert_eq!(event.event_type, LoginEventType::ValidationFailed);
    assert_eq!(event.uid, Some("123".into()));
}
```

**验证**:
```bash
cargo test models::login_event
```

---

### T009 创建models模块导出
**描述**: 统一导出所有模型

**文件**: `/workspace/desktop/src-tauri/src/models/mod.rs`

**内容**:
```rust
mod login_session;
mod cookies_data;
mod login_event;

pub use login_session::{LoginSession, QrCodeStatus};
pub use cookies_data::CookiesData;
pub use login_event::{LoginEvent, LoginEventType};
```

**依赖**: T006, T007, T008 完成

---

## Phase 3: 契约测试 (TDD)

### T010 [P] 契约测试: generate_qrcode
**描述**: 验证生成二维码命令的请求/响应schema

**文件**: `/workspace/desktop/tests/contract/test_generate_qrcode.rs`

**实现要点**:
- 参考 `/workspace/desktop/specs/001-cookies/contracts/generate_qrcode.md`
- 测试成功响应schema
- 测试网络失败错误
- 使用mock避免真实API调用

**测试用例**:
```rust
#[tokio::test]
async fn test_generate_qrcode_response_schema() {
    let response = mock_generate_qrcode().await.unwrap();

    assert!(!response.qr_id.is_empty());
    assert!(!response.qr_image.is_empty());
    assert!(response.expires_in > 0);
    assert!(response.expires_at > Utc::now());
}

#[tokio::test]
async fn test_generate_qrcode_network_failure() {
    // Mock网络失败
    let result = mock_network_failed_qrcode().await;
    assert!(matches!(result, Err(ApiError::NetworkFailed(_))));
}
```

**初始状态**: 测试应失败 (无实现)

**验证**:
```bash
cargo test contract::test_generate_qrcode --no-fail-fast
# 预期: 部分测试失败
```

---

### T011 [P] 契约测试: poll_login_status
**描述**: 验证轮询状态命令的所有状态响应

**文件**: `/workspace/desktop/tests/contract/test_poll_login_status.rs`

**测试用例**:
```rust
#[tokio::test]
async fn test_poll_pending_status() {
    let response = mock_poll_pending().await.unwrap();
    assert_eq!(response.status, QrCodeStatus::Pending);
    assert!(response.cookies.is_none());
}

#[tokio::test]
async fn test_poll_scanned_status() {
    let response = mock_poll_scanned().await.unwrap();
    assert_eq!(response.status, QrCodeStatus::Scanned);
    assert!(response.cookies.is_none());
}

#[tokio::test]
async fn test_poll_confirmed_status() {
    let response = mock_poll_confirmed().await.unwrap();
    assert_eq!(response.status, QrCodeStatus::Confirmed);
    assert!(response.cookies.is_some());
}

#[tokio::test]
async fn test_poll_expired_error() {
    let result = mock_poll_expired().await;
    assert!(matches!(result, Err(ApiError::QrCodeExpired { .. })));
}
```

**验证**:
```bash
cargo test contract::test_poll_login_status --no-fail-fast
```

---

### T012 [P] 契约测试: save_cookies
**描述**: 验证保存cookies的成功和失败场景

**文件**: `/workspace/desktop/tests/contract/test_save_cookies.rs`

**测试用例**:
```rust
#[tokio::test]
async fn test_save_valid_cookies() {
    let response = mock_save_valid().await.unwrap();
    assert!(response.success);
    assert_eq!(response.redis_key, "weibo:cookies:123");
    assert!(!response.is_overwrite);
}

#[tokio::test]
async fn test_save_invalid_cookies() {
    let result = mock_save_invalid().await;
    assert!(matches!(result, Err(ValidationError::ProfileApiFailed { .. })));
}

#[tokio::test]
async fn test_save_overwrite_existing() {
    mock_save_valid().await.unwrap(); // 首次
    let response = mock_save_valid().await.unwrap(); // 再次
    assert!(response.is_overwrite);
}
```

**验证**:
```bash
cargo test contract::test_save_cookies --no-fail-fast
```

---

### T013 [P] 契约测试: query_cookies
**描述**: 验证查询cookies的响应和错误

**文件**: `/workspace/desktop/tests/contract/test_query_cookies.rs`

**测试用例**:
```rust
#[tokio::test]
async fn test_query_existing_cookies() {
    mock_insert_cookies("123").await;
    let data = mock_query_cookies("123").await.unwrap();

    assert_eq!(data.uid, "123");
    assert!(!data.cookies.is_empty());
}

#[tokio::test]
async fn test_query_nonexistent_cookies() {
    let result = mock_query_cookies("999").await;
    assert!(matches!(result, Err(StorageError::NotFound(_))));
}
```

**验证**:
```bash
cargo test contract::test_query_cookies --no-fail-fast
```

---

## Phase 4: 服务层实现

### T014 [P] 实现WeiboApiService
**描述**: 封装微博API调用 (生成二维码、轮询状态)

**文件**: `/workspace/desktop/src-tauri/src/services/weibo_api.rs`

**实现要点**:
- 使用 `reqwest::Client` 异步HTTP请求
- API端点配置化 (从环境变量或配置文件)
- 错误码映射: 21327→Expired, 21330→Confirmed
- Exponential backoff重试 (最多3次)
- 日志记录请求/响应 (DEBUG级别)

**核心方法**:
```rust
pub struct WeiboApiService {
    client: reqwest::Client,
    client_id: String,
}

impl WeiboApiService {
    pub async fn generate_qrcode(&self) -> Result<QrCodeResponse, ApiError> {
        let response = self.client
            .get("https://api.weibo.com/oauth2/qrcode/generate")
            .query(&[("client_id", &self.client_id)])
            .send()
            .await?;

        // 解析并验证响应
        Ok(response.json().await?)
    }

    pub async fn poll_status(&self, qr_id: &str) -> Result<LoginStatusResponse, ApiError> {
        // 实现轮询逻辑
    }
}
```

**依赖**: T002 (Rust依赖), T005 (错误类型), T009 (数据模型)

**验证**:
- 单元测试 (使用mock HTTP server)
- 集成测试 (真实API调用,可选)

---

### T015 [P] 实现RedisStorage服务
**描述**: Redis连接池 + CRUD操作

**文件**: `/workspace/desktop/src-tauri/src/services/redis_storage.rs`

**实现要点**:
- 使用 `deadpool-redis` 连接池
- 配置: `max_size: 10`, `min_idle: 2`
- 超时: 5秒
- 健康检查: `check_health()` 方法
- TTL管理: 30天自动过期

**核心方法**:
```rust
pub struct RedisStorage {
    pool: deadpool_redis::Pool,
}

impl RedisStorage {
    pub fn new(redis_url: &str) -> Self {
        let cfg = deadpool_redis::Config::from_url(redis_url);
        let pool = cfg.create_pool(Some(Runtime::Tokio1)).unwrap();
        Self { pool }
    }

    pub async fn save_cookies(&self, data: &CookiesData) -> Result<bool, StorageError> {
        let mut conn = self.pool.get().await?;
        let key = &data.redis_key;

        let exists: bool = redis::cmd("EXISTS").arg(key).query_async(&mut conn).await?;

        redis::cmd("HSET")
            .arg(key)
            .arg("cookies")
            .arg(serde_json::to_string(&data.cookies)?)
            .query_async(&mut conn)
            .await?;

        redis::cmd("EXPIRE").arg(key).arg(30 * 24 * 3600).query_async(&mut conn).await?;

        Ok(exists)
    }

    pub async fn query_cookies(&self, uid: &str) -> Result<CookiesData, StorageError> {
        // 实现查询逻辑
    }
}
```

**依赖**: T002, T005, T007

**验证**:
- 需要运行Redis: `docker run -p 6379:6379 redis:7-alpine`
- 单元测试验证CRUD操作

---

### T016 实现CookiesValidator服务
**描述**: Playwright集成,调用sidecar验证cookies

**文件**: `/workspace/desktop/src-tauri/src/services/cookies_validator.rs`

**实现要点**:
- 使用 `tauri::api::process::Command::new_sidecar()`
- 传递cookies为JSON参数
- 解析stdout JSON输出
- 超时: 10秒
- 错误处理: Playwright进程崩溃、超时

**核心方法**:
```rust
pub struct CookiesValidator;

impl CookiesValidator {
    pub async fn validate(&self, cookies: &CookiesData) -> Result<bool, ValidationError> {
        let cookies_json = serde_json::to_string(&cookies.cookies)?;

        let (mut rx, _child) = Command::new_sidecar("validate-cookies")?
            .args(&["--cookies", &cookies_json])
            .spawn()?;

        while let Some(event) = rx.recv().await {
            if let CommandEvent::Stdout(line) = event {
                let result: ValidationResult = serde_json::from_str(&line)?;
                return Ok(result.valid);
            }
        }

        Err(ValidationError::PlaywrightNoOutput)
    }
}
```

**依赖**: T002, T005, T007, T021 (Playwright脚本)

**验证**:
- 需要先完成 T021 (Playwright脚本)
- 测试有效和无效cookies

---

### T017 [P] 实现AppState全局状态
**描述**: Tauri应用全局状态管理

**文件**: `/workspace/desktop/src-tauri/src/main.rs` (或单独的state.rs)

**实现要点**:
- 存储 `WeiboApiService`, `RedisStorage`, `CookiesValidator` 实例
- 配置加载 (client_id, redis_url)
- 日志初始化调用

**核心代码**:
```rust
pub struct AppState {
    pub weibo_api: WeiboApiService,
    pub redis_storage: RedisStorage,
    pub cookies_validator: CookiesValidator,
}

fn main() {
    logging::init_logging();

    let config = load_config();
    let state = AppState {
        weibo_api: WeiboApiService::new(config.client_id),
        redis_storage: RedisStorage::new(&config.redis_url),
        cookies_validator: CookiesValidator,
    };

    tauri::Builder::default()
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            // 注册命令
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

**依赖**: T004 (日志), T014-T016 (服务层)

---

## Phase 5: Tauri命令实现

### T018 实现generate_qrcode命令
**描述**: 生成二维码Tauri命令

**文件**: `/workspace/desktop/src-tauri/src/commands/qrcode.rs`

**实现要点**:
- 参考 `/workspace/desktop/specs/001-cookies/contracts/generate_qrcode.md`
- 调用 `WeiboApiService::generate_qrcode()`
- 日志记录成功/失败
- 错误映射为前端可识别的JSON

**代码**:
```rust
#[tauri::command]
pub async fn generate_qrcode(
    state: tauri::State<'_, AppState>
) -> Result<QrCodeResponse, ApiError> {
    tracing::info!("Generating QR code");

    let response = state.weibo_api.generate_qrcode().await?;

    tracing::info!(
        qr_id = %response.qr_id,
        expires_in = %response.expires_in,
        "QR code generated successfully"
    );

    Ok(response)
}
```

**依赖**: T014 (WeiboApiService), T017 (AppState)

**验证**: T010的契约测试应通过

---

### T019 实现poll_login_status命令
**描述**: 轮询登录状态 + 后台任务

**文件**: `/workspace/desktop/src-tauri/src/commands/login_poll.rs`

**实现要点**:
- 调用 `WeiboApiService::poll_status()`
- 可选: 实现后台轮询 (`tokio::spawn` + `Window::emit`)
- 状态变化时记录日志
- 避免噪音日志 (无变化不记录)

**代码**:
```rust
#[tauri::command]
pub async fn poll_login_status(
    qr_id: String,
    state: tauri::State<'_, AppState>
) -> Result<LoginStatusResponse, ApiError> {
    let response = state.weibo_api.poll_status(&qr_id).await?;

    if response.status != QrCodeStatus::Pending {
        tracing::info!(
            qr_id = %qr_id,
            status = ?response.status,
            "Status changed"
        );
    }

    Ok(response)
}

// 可选: 后台轮询命令
#[tauri::command]
pub async fn start_background_polling(
    qr_id: String,
    window: tauri::Window,
    state: tauri::State<'_, AppState>
) -> Result<(), String> {
    tokio::spawn(async move {
        let mut interval = 2;
        loop {
            tokio::time::sleep(Duration::from_secs(interval)).await;

            match poll_login_status(qr_id.clone(), state.clone()).await {
                Ok(status) => {
                    window.emit("login-status", &status).unwrap();
                    if status.status == QrCodeStatus::Confirmed {
                        break;
                    }
                }
                Err(_) => break,
            }

            interval = (interval + 1).min(5);
        }
    });
    Ok(())
}
```

**依赖**: T014, T017

**验证**: T011的契约测试应通过

---

### T020 实现save_cookies命令
**描述**: 验证并保存cookies

**文件**: `/workspace/desktop/src-tauri/src/commands/cookies_save.rs`

**实现要点**:
- 验证输入数据 (`CookiesData::validate()`)
- 调用 `CookiesValidator::validate()`
- 验证成功后调用 `RedisStorage::save_cookies()`
- 记录验证耗时
- 区分新增和覆盖

**代码**:
```rust
#[tauri::command]
#[tracing::instrument(skip_all, fields(uid = %uid))]
pub async fn save_cookies(
    uid: String,
    cookies: HashMap<String, String>,
    screen_name: Option<String>,
    state: tauri::State<'_, AppState>
) -> Result<SaveCookiesResponse, SaveCookiesError> {
    let mut cookies_data = CookiesData::new(uid, cookies);
    cookies_data.screen_name = screen_name;
    cookies_data.validate()?;

    tracing::info!("Starting cookies validation");
    let validation_start = Instant::now();

    let is_valid = state.cookies_validator.validate(&cookies_data).await?;
    if !is_valid {
        return Err(ValidationError::ProfileApiFailed { /* ... */ }.into());
    }

    let validation_duration = validation_start.elapsed();

    let is_overwrite = state.redis_storage.save_cookies(&cookies_data).await?;

    tracing::info!(
        validation_duration_ms = %validation_duration.as_millis(),
        is_overwrite = %is_overwrite,
        "Cookies saved successfully"
    );

    Ok(SaveCookiesResponse {
        success: true,
        redis_key: cookies_data.redis_key,
        validation_duration_ms: validation_duration.as_millis() as u64,
        is_overwrite,
    })
}
```

**依赖**: T015 (Redis), T016 (Validator), T017

**验证**: T012的契约测试应通过

---

### T021 实现query_cookies命令
**描述**: 从Redis查询cookies

**文件**: `/workspace/desktop/src-tauri/src/commands/cookies_query.rs`

**实现要点**:
- 调用 `RedisStorage::query_cookies()`
- DEBUG级别日志 (非INFO)
- 错误处理: NotFound, RedisConnectionFailed

**代码**:
```rust
#[tauri::command]
pub async fn query_cookies(
    uid: String,
    state: tauri::State<'_, AppState>
) -> Result<CookiesData, StorageError> {
    let cookies_data = state.redis_storage.query_cookies(&uid).await?;

    tracing::debug!(
        uid = %uid,
        cookies_count = %cookies_data.cookies.len(),
        "Cookies queried successfully"
    );

    Ok(cookies_data)
}
```

**依赖**: T015, T017

**验证**: T013的契约测试应通过

---

### T022 注册Tauri命令
**描述**: 在main.rs中注册所有命令

**文件**: `/workspace/desktop/src-tauri/src/main.rs`

**实现要点**:
```rust
mod commands;

use commands::{
    qrcode::generate_qrcode,
    login_poll::{poll_login_status, start_background_polling},
    cookies_save::save_cookies,
    cookies_query::query_cookies,
};

fn main() {
    // ...
    tauri::Builder::default()
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            generate_qrcode,
            poll_login_status,
            start_background_polling,
            save_cookies,
            query_cookies,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

**依赖**: T018-T021

**验证**:
```bash
cargo build
pnpm tauri dev  # 应启动无错误
```

---

## Phase 6: Playwright脚本

### T023 [P] 实现Playwright验证脚本
**描述**: Node.js脚本,调用微博资料API验证cookies

**文件**: `/workspace/desktop/playwright/validate-cookies.js`

**实现要点**:
- 使用Playwright Chromium
- 接收命令行参数: `--cookies <json>`
- 访问微博资料API: `https://weibo.com/ajax/profile/info`
- 输出JSON结果到stdout: `{ valid: boolean, status: number }`
- 超时: 10秒

**代码**:
```javascript
#!/usr/bin/env node
const { chromium } = require('playwright');

async function validateCookies(cookiesJson) {
  const browser = await chromium.launch({ headless: true });
  const context = await browser.newContext();

  const cookies = JSON.parse(cookiesJson);
  await context.addCookies(
    Object.entries(cookies).map(([name, value]) => ({
      name,
      value,
      domain: '.weibo.com',
      path: '/',
    }))
  );

  const page = await context.newPage();
  try {
    const response = await page.goto('https://weibo.com/ajax/profile/info', {
      timeout: 10000,
      waitUntil: 'networkidle',
    });

    const valid = response.ok() && response.status() === 200;
    console.log(JSON.stringify({ valid, status: response.status() }));
  } catch (error) {
    console.log(JSON.stringify({ valid: false, error: error.message }));
  } finally {
    await browser.close();
  }
}

const args = process.argv.slice(2);
const cookiesIndex = args.indexOf('--cookies');
if (cookiesIndex === -1 || !args[cookiesIndex + 1]) {
  console.error('Usage: validate-cookies --cookies <json>');
  process.exit(1);
}

validateCookies(args[cookiesIndex + 1]).catch(console.error);
```

**验证**:
```bash
cd /workspace/desktop/playwright
node validate-cookies.js --cookies '{"SUB":"test","SUBP":"test"}'
# 输出: {"valid":false,"status":401}
```

---

### T024 打包Playwright为可执行文件
**描述**: 使用pkg打包为跨平台二进制

**文件**:
- `/workspace/desktop/playwright/package.json` (添加pkg配置)
- `/workspace/desktop/src-tauri/tauri.conf.json` (sidecar配置)

**实现要点**:
- 安装pkg: `npm install -g pkg`
- 打包命令: `pkg validate-cookies.js --targets node18-linux-x64,node18-macos-x64,node18-win-x64`
- 输出到: `/workspace/desktop/src-tauri/binaries/`
- Tauri配置: `"externalBin": ["binaries/validate-cookies"]`

**pkg配置** (package.json):
```json
{
  "bin": "validate-cookies.js",
  "pkg": {
    "assets": ["node_modules/playwright/**/*"],
    "targets": ["node18-linux-x64", "node18-macos-x64", "node18-win-x64"]
  }
}
```

**Tauri配置** (tauri.conf.json):
```json
{
  "tauri": {
    "bundle": {
      "externalBin": ["binaries/validate-cookies"]
    }
  }
}
```

**依赖**: T023

**验证**:
```bash
cd /workspace/desktop/playwright
pkg validate-cookies.js
ls ../src-tauri/binaries/  # 应看到可执行文件
```

---

## Phase 7: 前端实现

### T025 [P] 实现QrCodeDisplay组件
**描述**: 显示二维码和倒计时

**文件**: `/workspace/desktop/src/components/QrCodeDisplay.tsx`

**实现要点**:
- 接收props: `qrImage` (base64), `expiresAt` (ISO 8601)
- 使用 `<img>` 显示二维码
- 倒计时显示: `useEffect` + `setInterval`
- 过期时显示"已过期"
- TailwindCSS样式

**代码**:
```typescript
interface QrCodeDisplayProps {
  qrImage: string;
  expiresAt: string;
  onExpired?: () => void;
}

export function QrCodeDisplay({ qrImage, expiresAt, onExpired }: QrCodeDisplayProps) {
  const [timeLeft, setTimeLeft] = useState<number>(0);

  useEffect(() => {
    const updateTimer = () => {
      const now = new Date().getTime();
      const expiry = new Date(expiresAt).getTime();
      const left = Math.max(0, Math.floor((expiry - now) / 1000));

      setTimeLeft(left);
      if (left === 0) {
        onExpired?.();
      }
    };

    updateTimer();
    const timer = setInterval(updateTimer, 1000);
    return () => clearInterval(timer);
  }, [expiresAt, onExpired]);

  return (
    <div className="flex flex-col items-center space-y-4">
      <img
        src={`data:image/png;base64,${qrImage}`}
        alt="QR Code"
        className={`w-64 h-64 ${timeLeft === 0 ? 'opacity-30' : ''}`}
      />
      <div className="text-lg font-semibold">
        {timeLeft > 0 ? `${timeLeft}秒后过期` : '二维码已过期'}
      </div>
    </div>
  );
}
```

**验证**: 运行应用,生成二维码时检查组件渲染

---

### T026 [P] 实现LoginStatus组件
**描述**: 显示登录状态和反馈

**文件**: `/workspace/desktop/src/components/LoginStatus.tsx`

**实现要点**:
- 接收props: `status` (pending|scanned|confirmed|expired)
- 不同状态显示不同文案和图标
- 动画效果: loading spinner (pending), checkmark (confirmed)

**代码**:
```typescript
interface LoginStatusProps {
  status: 'pending' | 'scanned' | 'confirmed' | 'expired';
  screenName?: string;
}

export function LoginStatus({ status, screenName }: LoginStatusProps) {
  const statusConfig = {
    pending: { text: '请使用微博App扫描二维码', color: 'text-gray-600' },
    scanned: { text: '已扫描,请在手机上确认登录', color: 'text-blue-600' },
    confirmed: { text: `登录成功! 欢迎, ${screenName || '用户'}`, color: 'text-green-600' },
    expired: { text: '二维码已过期,请刷新重试', color: 'text-red-600' },
  };

  const config = statusConfig[status];

  return (
    <div className={`text-center text-xl ${config.color}`}>
      {status === 'pending' && <LoadingSpinner />}
      {status === 'confirmed' && <CheckmarkIcon />}
      <p>{config.text}</p>
    </div>
  );
}
```

**验证**: 测试不同状态的UI显示

---

### T027 [P] 实现CookiesViewer组件
**描述**: 查看和导出cookies

**文件**: `/workspace/desktop/src/components/CookiesViewer.tsx`

**实现要点**:
- 显示cookies表格 (key, value)
- 显示元数据: uid, fetched_at, screen_name
- 提供"复制"按钮 (复制为JSON)
- 可选: 导出为文件

**代码**:
```typescript
interface CookiesViewerProps {
  cookiesData: CookiesData;
}

export function CookiesViewer({ cookiesData }: CookiesViewerProps) {
  const copyToClipboard = () => {
    const json = JSON.stringify(cookiesData.cookies, null, 2);
    navigator.clipboard.writeText(json);
    // 显示toast提示
  };

  return (
    <div className="space-y-4">
      <div className="flex justify-between items-center">
        <h2 className="text-2xl font-bold">{cookiesData.screen_name || cookiesData.uid}</h2>
        <button onClick={copyToClipboard} className="btn btn-primary">
          复制Cookies
        </button>
      </div>

      <div className="grid grid-cols-2 gap-2 text-sm">
        <span className="font-semibold">用户ID:</span>
        <span>{cookiesData.uid}</span>
        <span className="font-semibold">获取时间:</span>
        <span>{new Date(cookiesData.fetched_at).toLocaleString()}</span>
      </div>

      <table className="w-full">
        <thead>
          <tr>
            <th>Cookie名称</th>
            <th>值</th>
          </tr>
        </thead>
        <tbody>
          {Object.entries(cookiesData.cookies).map(([key, value]) => (
            <tr key={key}>
              <td className="font-mono">{key}</td>
              <td className="font-mono text-sm truncate">{value}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
```

**验证**: 查询cookies后检查组件渲染

---

### T028 实现useQrCodeLogin hook
**描述**: 封装登录流程逻辑

**文件**: `/workspace/desktop/src/hooks/useQrCodeLogin.ts`

**实现要点**:
- 状态管理: qrCode, status, cookies
- 调用Tauri命令: `generate_qrcode`, `start_background_polling`
- 监听事件: `listen('login-status')`
- 错误处理

**代码**:
```typescript
import { useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import { listen } from '@tauri-apps/api/event';

export function useQrCodeLogin() {
  const [qrCode, setQrCode] = useState<QrCodeResponse | null>(null);
  const [status, setStatus] = useState<QrCodeStatus>('pending');
  const [cookies, setCookies] = useState<CookiesData | null>(null);
  const [error, setError] = useState<string | null>(null);

  const generateQrCode = useCallback(async () => {
    try {
      setError(null);
      const response = await invoke<QrCodeResponse>('generate_qrcode');
      setQrCode(response);
      setStatus('pending');

      // 开始后台轮询
      await invoke('start_background_polling', { qrId: response.qr_id });

      // 监听状态变化
      const unlisten = await listen<LoginStatusResponse>('login-status', (event) => {
        setStatus(event.payload.status);
        if (event.payload.cookies) {
          setCookies(event.payload.cookies);
        }
      });

      return unlisten;
    } catch (err: any) {
      setError(err.message || '生成二维码失败');
      throw err;
    }
  }, []);

  return {
    qrCode,
    status,
    cookies,
    error,
    generateQrCode,
  };
}
```

**依赖**: T018, T019 (Tauri命令)

**验证**: 在App组件中调用,测试完整登录流程

---

### T029 实现useCookiesQuery hook
**描述**: 查询cookies逻辑

**文件**: `/workspace/desktop/src/hooks/useCookiesQuery.ts`

**实现要点**:
- 调用 `query_cookies` 命令
- 加载状态管理
- 错误处理

**代码**:
```typescript
import { useState } from 'react';
import { invoke } from '@tauri-apps/api/tauri';

export function useCookiesQuery() {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const queryCookies = async (uid: string): Promise<CookiesData | null> => {
    try {
      setLoading(true);
      setError(null);
      const data = await invoke<CookiesData>('query_cookies', { uid });
      return data;
    } catch (err: any) {
      setError(err.message || '查询失败');
      return null;
    } finally {
      setLoading(false);
    }
  };

  return {
    loading,
    error,
    queryCookies,
  };
}
```

**依赖**: T021 (query_cookies命令)

---

### T030 实现主App组件
**描述**: 整合所有组件和hooks

**文件**: `/workspace/desktop/src/App.tsx`

**实现要点**:
- 使用 `useQrCodeLogin` hook
- 渲染 `QrCodeDisplay`, `LoginStatus`, `CookiesViewer` 组件
- 路由或tab切换 (登录页 vs 查看页)

**代码**:
```typescript
import { useQrCodeLogin } from './hooks/useQrCodeLogin';
import { QrCodeDisplay } from './components/QrCodeDisplay';
import { LoginStatus } from './components/LoginStatus';
import { CookiesViewer } from './components/CookiesViewer';

export function App() {
  const { qrCode, status, cookies, error, generateQrCode } = useQrCodeLogin();

  return (
    <div className="container mx-auto p-8">
      <h1 className="text-3xl font-bold mb-8">微博登录管理器</h1>

      {error && (
        <div className="bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded mb-4">
          {error}
        </div>
      )}

      {!qrCode && (
        <button onClick={generateQrCode} className="btn btn-primary">
          生成登录二维码
        </button>
      )}

      {qrCode && (
        <div className="space-y-6">
          <QrCodeDisplay
            qrImage={qrCode.qr_image}
            expiresAt={qrCode.expires_at}
            onExpired={() => setQrCode(null)}
          />
          <LoginStatus status={status} screenName={cookies?.screen_name} />
          {cookies && <CookiesViewer cookiesData={cookies} />}
        </div>
      )}
    </div>
  );
}
```

**依赖**: T025-T029

**验证**: 完整登录流程测试

---

## Phase 8: 集成测试

### T031 集成测试: 完整登录流程
**描述**: 端到端测试从生成二维码到保存cookies

**文件**: `/workspace/desktop/tests/integration/test_full_login_flow.rs`

**实现要点**:
- 使用真实的微博API (或mock)
- 模拟扫码和确认 (如果可能)
- 验证Redis存储
- 验证日志输出

**测试用例**:
```rust
#[tokio::test]
async fn test_complete_login_flow() {
    // 1. 生成二维码
    let qr_response = generate_qrcode(app_state()).await.unwrap();
    assert!(!qr_response.qr_id.is_empty());

    // 2. 模拟扫描 (需要mock或手动介入)
    // ...

    // 3. 轮询直到确认
    let mut status = QrCodeStatus::Pending;
    for _ in 0..60 {
        let response = poll_login_status(qr_response.qr_id.clone(), app_state()).await.unwrap();
        status = response.status;
        if status == QrCodeStatus::Confirmed {
            assert!(response.cookies.is_some());
            break;
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }

    assert_eq!(status, QrCodeStatus::Confirmed);

    // 4. 验证Redis存储
    let uid = "test_uid";
    let cookies_data = query_cookies(uid.into(), app_state()).await.unwrap();
    assert_eq!(cookies_data.uid, uid);
}
```

**依赖**: 所有Tauri命令和服务层完成

**验证**: 需要真实微博账户或完善的mock系统

---

### T032 [P] 集成测试: 错误处理
**描述**: 测试所有错误场景

**文件**: `/workspace/desktop/tests/integration/test_error_handling.rs`

**测试场景**:
- 网络断开
- API限流
- Redis连接失败
- Cookies验证失败
- 二维码过期

**验证**: 所有错误路径都有日志记录,应用不崩溃

---

### T033 [P] 集成测试: Redis操作
**描述**: 测试Redis存储的各种场景

**文件**: `/workspace/desktop/tests/integration/test_redis_operations.rs`

**测试场景**:
- 保存和查询
- 覆盖更新
- TTL验证
- 并发操作
- 连接池耗尽恢复

**验证**: 需要运行Redis服务

---

## Phase 9: 文档和配置

### T034 [P] 编写README.md
**描述**: 项目说明文档

**文件**: `/workspace/desktop/README.md`

**内容**:
- 项目简介
- 功能特性
- 安装步骤
- 配置说明 (client_id, redis_url)
- 使用指南
- 开发指南
- 故障排查

**参考**: `/workspace/desktop/specs/001-cookies/quickstart.md`

---

### T035 [P] 配置Tauri应用
**描述**: 完善tauri.conf.json

**文件**: `/workspace/desktop/src-tauri/tauri.conf.json`

**配置项**:
- 应用名称、版本
- 窗口配置
- sidecar打包
- 权限配置
- 构建选项

---

### T036 [P] 创建配置示例文件
**描述**: 环境配置模板

**文件**: `/workspace/desktop/config.example.json`

**内容**:
```json
{
  "weibo_client_id": "YOUR_APP_ID_HERE",
  "redis_url": "redis://localhost:6379",
  "log_level": "info"
}
```

**使用说明**: 复制为 `config.json` 并填写实际值

---

## 依赖关系图

```
Phase 1 (基础设施)
├── T001-T003 [P] → 项目初始化
├── T004 [P] → 日志系统
└── T005 [P] → 错误类型
        ↓
Phase 2 (数据模型)
├── T006-T008 [P] → 模型实现
└── T009 → 模块导出 (依赖 T006-T008)
        ↓
Phase 3 (契约测试)
└── T010-T013 [P] → 契约测试 (依赖 T009)
        ↓
Phase 4 (服务层)
├── T014-T015 [P] → API + Redis (依赖 T002, T005, T009)
├── T016 → Validator (依赖 T021-Playwright)
└── T017 → AppState (依赖 T014-T016)
        ↓
Phase 5 (Tauri命令)
├── T018-T021 [并行] → 4个命令 (依赖 T017)
└── T022 → 注册命令 (依赖 T018-T021)
        ↓
Phase 6 (Playwright)
├── T023 [P] → 验证脚本
└── T024 → 打包 (依赖 T023)
        ↓
Phase 7 (前端)
├── T025-T027 [P] → 组件 (可并行)
├── T028-T029 → hooks (依赖 T022)
└── T030 → 主App (依赖 T025-T029)
        ↓
Phase 8 (集成测试)
├── T031 → 完整流程 (依赖所有)
└── T032-T033 [P] → 错误和Redis测试
        ↓
Phase 9 (文档)
└── T034-T036 [P] → README + 配置
```

---

## 并行执行建议

**第一批** (可同时开始):
- T001-T005: 基础设施
- T023: Playwright脚本

**第二批**:
- T006-T008: 数据模型
- T010-T013: 契约测试

**第三批**:
- T014-T015: API + Redis服务
- T024: Playwright打包
- T025-T027: React组件

**第四批**:
- T018-T021: Tauri命令
- T028-T029: React hooks

**第五批**:
- T031-T033: 集成测试
- T034-T036: 文档

---

## 验证检查清单

### 代码质量
- [ ] 所有单元测试通过 (`cargo test`)
- [ ] 所有契约测试通过
- [ ] 集成测试通过 (需要Redis)
- [ ] 无编译警告 (`cargo clippy`)
- [ ] 代码格式化 (`cargo fmt --check`)

### 功能验证
- [ ] 能生成二维码并显示
- [ ] 扫码后状态正确更新
- [ ] Cookies成功保存到Redis
- [ ] 能查询和显示cookies
- [ ] 错误场景都有友好提示

### 性能验证
- [ ] 二维码生成 < 500ms
- [ ] Cookies验证 < 2秒
- [ ] Redis操作 < 100ms
- [ ] UI响应流畅 (60fps)

### 日志验证
- [ ] 关键路径都有INFO日志
- [ ] 错误都有ERROR日志
- [ ] 无敏感数据泄漏 (cookies值)
- [ ] 日志格式为JSON
- [ ] 文件rotation正常工作

### 文档验证
- [ ] README清晰易懂
- [ ] quickstart.md场景都能执行
- [ ] 配置示例文件完整

---

**总任务数**: 36个
**预估工作量**: 约40-50小时 (单人)
**关键路径**: Phase 1 → Phase 2 → Phase 4 → Phase 5 → Phase 7

遵循TDD原则,先写测试再实现,确保每个任务完成后都有明确的验收标准。
