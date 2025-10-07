# Data Model: 微博关键字增量爬取

**Feature**: 003- | **Date**: 2025-10-07
**Purpose**: 定义核心数据实体、字段验证规则和状态机转换

---

## 1. CrawlTask (爬取任务)

### 职责
表示一次关键字爬取任务的完整生命周期,从创建到完成的所有状态和统计信息。

### Rust结构定义

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 爬取任务
///
/// 每个字段都不可替代:
/// - id: 唯一标识任务,支持并发管理多个任务
/// - keyword: 搜索关键字,决定爬取内容
/// - event_start_time: 历史回溯的起点,定义时间范围
/// - status: 状态机的当前状态,决定可执行的操作
/// - 时间统计: 支持断点续爬和增量更新
/// - 计数器: 实时进度展示
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlTask {
    /// 任务ID (UUID v4)
    pub id: String,

    /// 搜索关键字
    pub keyword: String,

    /// 事件开始时间 (历史回溯的终点)
    pub event_start_time: DateTime<Utc>,

    /// 任务状态
    pub status: CrawlStatus,

    /// 已爬取的最小帖子时间 (向下取整到小时)
    /// None表示尚未爬取任何帖子
    pub min_post_time: Option<DateTime<Utc>>,

    /// 已爬取的最大帖子时间 (向上取整到小时)
    /// None表示尚未爬取任何帖子
    pub max_post_time: Option<DateTime<Utc>>,

    /// 已爬取帖子总数
    pub crawled_count: u64,

    /// 任务创建时间
    pub created_at: DateTime<Utc>,

    /// 最后更新时间 (每次状态变化或爬取进度更新时刷新)
    pub updated_at: DateTime<Utc>,

    /// 失败原因 (仅当status=Failed时有值)
    pub failure_reason: Option<String>,
}

impl CrawlTask {
    /// 创建新任务
    pub fn new(keyword: String, event_start_time: DateTime<Utc>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            keyword,
            event_start_time,
            status: CrawlStatus::Created,
            min_post_time: None,
            max_post_time: None,
            crawled_count: 0,
            created_at: now,
            updated_at: now,
            failure_reason: None,
        }
    }

    /// 更新爬取进度 (调用时自动刷新updated_at)
    pub fn update_progress(&mut self, post_time: DateTime<Utc>, post_count: u64) {
        self.min_post_time = Some(
            self.min_post_time
                .map(|t| t.min(post_time))
                .unwrap_or(post_time)
        );
        self.max_post_time = Some(
            self.max_post_time
                .map(|t| t.max(post_time))
                .unwrap_or(post_time)
        );
        self.crawled_count += post_count;
        self.updated_at = Utc::now();
    }

    /// 状态转换 (带验证)
    pub fn transition_to(&mut self, new_status: CrawlStatus) -> Result<(), String> {
        if !self.status.can_transition_to(&new_status) {
            return Err(format!(
                "无效的状态转换: {} -> {}",
                self.status.as_str(),
                new_status.as_str()
            ));
        }
        self.status = new_status;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// 标记失败
    pub fn mark_failed(&mut self, reason: String) {
        self.status = CrawlStatus::Failed;
        self.failure_reason = Some(reason);
        self.updated_at = Utc::now();
    }

    /// Redis存储键
    pub fn redis_key(&self) -> String {
        format!("crawl:task:{}", self.id)
    }
}
```

### 状态机: CrawlStatus

```rust
/// 爬取任务状态
///
/// 状态转换规则:
/// Created → HistoryCrawling → HistoryCompleted → IncrementalCrawling
///         ↘ Paused ↔ (恢复到上一个活跃状态)
///         ↘ Failed (终态,可手动重试)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrawlStatus {
    /// 已创建,未开始
    Created,

    /// 历史回溯中 (从现在到event_start_time)
    HistoryCrawling,

    /// 历史回溯完成
    HistoryCompleted,

    /// 增量更新中 (从max_post_time到现在)
    IncrementalCrawling,

    /// 已暂停 (用户主动暂停或检测到验证码)
    Paused,

    /// 失败 (网络错误/Redis错误等)
    Failed,
}

impl CrawlStatus {
    /// 检查是否可以转换到目标状态
    pub fn can_transition_to(&self, target: &CrawlStatus) -> bool {
        use CrawlStatus::*;
        matches!(
            (self, target),
            (Created, HistoryCrawling)
                | (HistoryCrawling, HistoryCompleted)
                | (HistoryCrawling, Paused)
                | (HistoryCrawling, Failed)
                | (HistoryCompleted, IncrementalCrawling)
                | (IncrementalCrawling, Paused)
                | (IncrementalCrawling, Failed)
                | (Paused, HistoryCrawling)
                | (Paused, IncrementalCrawling)
                | (Failed, HistoryCrawling) // 允许手动重试
        )
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Created => "Created",
            Self::HistoryCrawling => "HistoryCrawling",
            Self::HistoryCompleted => "HistoryCompleted",
            Self::IncrementalCrawling => "IncrementalCrawling",
            Self::Paused => "Paused",
            Self::Failed => "Failed",
        }
    }
}
```

### Redis存储结构

**Key**: `crawl:task:{task_id}`
**Type**: Hash
**TTL**: 90天

**Fields**:
```
id: "550e8400-e29b-41d4-a716-446655440000"
keyword: "国庆"
event_start_time: "1696118400"  # Unix timestamp
status: "HistoryCrawling"
min_post_time: "1696118400"
max_post_time: "1696204800"
crawled_count: "12345"
created_at: "1696118400"
updated_at: "1696204800"
failure_reason: ""  # 空字符串表示None
```

### 验证规则

```rust
impl CrawlTask {
    /// 验证任务数据完整性
    pub fn validate(&self) -> Result<(), String> {
        // 1. 关键字不能为空
        if self.keyword.trim().is_empty() {
            return Err("关键字不能为空".to_string());
        }

        // 2. 事件开始时间不能晚于当前时间
        if self.event_start_time > Utc::now() {
            return Err("事件开始时间不能是未来时间".to_string());
        }

        // 3. min_post_time必须 <= max_post_time
        if let (Some(min), Some(max)) = (self.min_post_time, self.max_post_time) {
            if min > max {
                return Err("min_post_time不能大于max_post_time".to_string());
            }
        }

        // 4. 状态为Failed时必须有失败原因
        if self.status == CrawlStatus::Failed && self.failure_reason.is_none() {
            return Err("失败状态必须包含失败原因".to_string());
        }

        Ok(())
    }
}
```

---

## 2. WeiboPost (微博帖子)

### 职责
存储单条微博帖子的完整信息,支持时间范围查询和数据导出。

### Rust结构定义

```rust
/// 微博帖子
///
/// 每个字段都不可替代:
/// - id: 微博唯一ID,用于去重
/// - task_id: 关联到爬取任务,支持按任务查询
/// - text: 帖子内容,核心数据
/// - created_at: 发布时间,用于排序和时间范围查询
/// - 作者信息: 支持数据分析
/// - 互动数据: 反映帖子热度
/// - crawled_at: 爬取时间,用于数据溯源
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeiboPost {
    /// 微博帖子ID (微博官方ID)
    pub id: String,

    /// 所属任务ID
    pub task_id: String,

    /// 帖子内容
    pub text: String,

    /// 发布时间 (微博服务器时间)
    pub created_at: DateTime<Utc>,

    /// 作者UID
    pub author_uid: String,

    /// 作者昵称
    pub author_screen_name: String,

    /// 转发数
    pub reposts_count: u64,

    /// 评论数
    pub comments_count: u64,

    /// 点赞数
    pub attitudes_count: u64,

    /// 爬取时间
    pub crawled_at: DateTime<Utc>,
}

impl WeiboPost {
    /// 创建新帖子
    pub fn new(
        id: String,
        task_id: String,
        text: String,
        created_at: DateTime<Utc>,
        author_uid: String,
        author_screen_name: String,
        reposts_count: u64,
        comments_count: u64,
        attitudes_count: u64,
    ) -> Self {
        Self {
            id,
            task_id,
            text,
            created_at,
            author_uid,
            author_screen_name,
            reposts_count,
            comments_count,
            attitudes_count,
            crawled_at: Utc::now(),
        }
    }

    /// 序列化为JSON (用于Redis存储)
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// 从JSON反序列化
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}
```

### Redis存储结构

**1. 帖子内容** (Sorted Set):
- **Key**: `crawl:posts:{task_id}`
- **Type**: Sorted Set
- **Score**: 帖子发布时间戳 (秒)
- **Member**: 序列化的WeiboPost JSON
- **TTL**: 90天

**命令示例**:
```redis
# 添加帖子
ZADD crawl:posts:task123 1696204800 '{"id":"5008471234567890",...}'

# 按时间范围查询 (2025-10-01 00:00 到 2025-10-07 23:59)
ZRANGEBYSCORE crawl:posts:task123 1696118400 1696723140

# 获取总数
ZCARD crawl:posts:task123
```

**2. 去重索引** (Set):
- **Key**: `crawl:post_ids:{task_id}`
- **Type**: Set
- **Members**: 所有帖子ID
- **用途**: O(1)检查帖子是否已存在

**命令示例**:
```redis
# 检查并添加 (SADD自动去重)
SADD crawl:post_ids:task123 "5008471234567890"

# 检查是否存在
SISMEMBER crawl:post_ids:task123 "5008471234567890"
```

### 验证规则

```rust
impl WeiboPost {
    /// 验证帖子数据完整性
    pub fn validate(&self) -> Result<(), String> {
        // 1. 帖子ID不能为空
        if self.id.trim().is_empty() {
            return Err("帖子ID不能为空".to_string());
        }

        // 2. 作者UID不能为空
        if self.author_uid.trim().is_empty() {
            return Err("作者UID不能为空".to_string());
        }

        // 3. 发布时间不能晚于爬取时间
        if self.created_at > self.crawled_at {
            return Err("帖子发布时间不能晚于爬取时间".to_string());
        }

        Ok(())
    }
}
```

---

## 3. CrawlCheckpoint (检查点)

### 职责
支持断点续爬,记录任务的精确执行位置。

### Rust结构定义

```rust
/// 爬取检查点
///
/// 三级定位: 时间分片 + 页码 + 方向
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlCheckpoint {
    /// 任务ID
    pub task_id: String,

    /// 当前时间分片的起始时间
    pub shard_start_time: DateTime<Utc>,

    /// 当前时间分片的结束时间
    pub shard_end_time: DateTime<Utc>,

    /// 当前分片内的页码 (1-50)
    pub current_page: u32,

    /// 爬取方向
    pub direction: CrawlDirection,

    /// 已完成的时间分片列表 (避免重复爬取)
    pub completed_shards: Vec<(DateTime<Utc>, DateTime<Utc>)>,

    /// 检查点保存时间
    pub saved_at: DateTime<Utc>,
}

/// 爬取方向
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrawlDirection {
    /// 向后回溯 (从现在到event_start_time)
    Backward,

    /// 向前更新 (从max_post_time到现在)
    Forward,
}

impl CrawlCheckpoint {
    /// 创建新检查点 (历史回溯模式)
    pub fn new_backward(
        task_id: String,
        shard_start_time: DateTime<Utc>,
        shard_end_time: DateTime<Utc>,
    ) -> Self {
        Self {
            task_id,
            shard_start_time,
            shard_end_time,
            current_page: 1,
            direction: CrawlDirection::Backward,
            completed_shards: Vec::new(),
            saved_at: Utc::now(),
        }
    }

    /// 创建增量更新检查点
    pub fn new_forward(
        task_id: String,
        shard_start_time: DateTime<Utc>,
    ) -> Self {
        Self {
            task_id,
            shard_start_time,
            shard_end_time: Utc::now(),
            current_page: 1,
            direction: CrawlDirection::Forward,
            completed_shards: Vec::new(),
            saved_at: Utc::now(),
        }
    }

    /// 推进到下一页
    pub fn advance_page(&mut self) {
        self.current_page += 1;
        self.saved_at = Utc::now();
    }

    /// 标记当前分片完成,进入下一个分片
    pub fn complete_current_shard(&mut self, next_start: DateTime<Utc>, next_end: DateTime<Utc>) {
        self.completed_shards.push((self.shard_start_time, self.shard_end_time));
        self.shard_start_time = next_start;
        self.shard_end_time = next_end;
        self.current_page = 1;
        self.saved_at = Utc::now();
    }

    /// Redis存储键
    pub fn redis_key(&self) -> String {
        format!("crawl:checkpoint:{}", self.task_id)
    }
}
```

### Redis存储结构

**Key**: `crawl:checkpoint:{task_id}`
**Type**: Hash
**TTL**: 与任务同生命周期

**Fields**:
```
task_id: "550e8400-e29b-41d4-a716-446655440000"
shard_start_time: "1696118400"
shard_end_time: "1696122000"
current_page: "15"
direction: "Backward"
completed_shards: "[{\"start\":1696114800,\"end\":1696118400}]"  # JSON数组
saved_at: "1696204800"
```

---

## 4. 数据关系图

```
┌──────────────┐
│  CrawlTask   │ (1)
│  - id        │────┐
│  - keyword   │    │
│  - status    │    │
└──────────────┘    │
                    │
                    │ (1:N)
                    │
    ┌───────────────┼───────────────┐
    │               │               │
    ▼               ▼               ▼
┌────────────┐ ┌──────────────┐ ┌──────────────┐
│ WeiboPost  │ │  Checkpoint  │ │ Post IDs Set │
│ (Sorted    │ │  (Hash)      │ │ (Set)        │
│  Set)      │ │              │ │              │
└────────────┘ └──────────────┘ └──────────────┘
```

---

## 5. 状态转换示例

### 场景1: 正常历史回溯完成

```
Created
  ↓ (用户点击"开始爬取")
HistoryCrawling
  ↓ (爬取到event_start_time)
HistoryCompleted
  ↓ (用户启动增量更新)
IncrementalCrawling
```

### 场景2: 中途暂停后恢复

```
HistoryCrawling
  ↓ (用户点击"暂停")
Paused
  ↓ (用户点击"恢复")
HistoryCrawling
```

### 场景3: 检测到验证码

```
HistoryCrawling
  ↓ (检测到验证码)
Paused (failure_reason: "检测到验证码,需要人工处理")
  ↓ (用户手动处理后恢复)
HistoryCrawling
```

### 场景4: 网络错误后重试

```
HistoryCrawling
  ↓ (重试3次失败)
Failed (failure_reason: "网络请求失败: ...")
  ↓ (用户点击"重试")
HistoryCrawling
```

---

## 设计原则符合性

### 存在即合理
- 每个模型都服务于不可替代的职责
- CrawlTask: 任务生命周期管理
- WeiboPost: 帖子内容存储和查询
- CrawlCheckpoint: 断点续爬精确定位

### 优雅即简约
- 状态机清晰可见 (枚举 + 转换验证)
- 命名讲述故事 (`advance_page`, `complete_current_shard`)
- Redis存储结构自然对应数据访问模式

### 性能即艺术
- Sorted Set: O(log N + M)范围查询
- Set去重: O(1)存在性检查
- Hash存储: O(1)字段更新

### 错误处理
- 状态转换带验证,防止非法操作
- 验证规则捕获数据不一致
- 失败状态记录原因,支持诊断

### 日志表达
- 所有时间字段使用`DateTime<Utc>`,便于日志记录
- 序列化为JSON,结构化日志友好
