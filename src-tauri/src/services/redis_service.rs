use deadpool_redis::{Config, Pool, Runtime};
use redis::AsyncCommands;
use std::collections::HashMap;

use crate::models::{CookiesData, CrawlCheckpoint, CrawlTask, StorageError, WeiboPost};

/// Redis服务
///
/// 管理连接池,提供Cookies存储/查询/删除操作。
/// 职责单一:仅处理数据持久化,不涉及业务逻辑。
pub struct RedisService {
    pool: Pool,
}

impl RedisService {
    /// 初始化Redis连接池
    ///
    /// # 参数
    /// - `redis_url`: Redis连接URL,格式: `redis://host:port` 或 `redis://host:port/db`
    ///
    /// # 错误
    /// 返回 `StorageError::RedisConnectionFailed` 如果连接池创建失败
    ///
    /// # 示例
    /// ```no_run
    /// use weibo_login::services::RedisService;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let service = RedisService::new("redis://localhost:6379")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(redis_url: &str) -> Result<Self, StorageError> {
        let config = Config::from_url(redis_url);
        let pool = config.create_pool(Some(Runtime::Tokio1)).map_err(|e| {
            tracing::error!(
                Redis连接URL = %redis_url,
                错误 = %e,
                "创建Redis连接池失败"
            );
            StorageError::RedisConnectionFailed(e.to_string())
        })?;

        tracing::info!(Redis连接URL = %redis_url, "Redis连接池创建成功");
        Ok(Self { pool })
    }

    /// 准备Redis字段数据
    fn prepare_redis_fields(
        cookies_data: &CookiesData,
    ) -> Result<(String, String, String), StorageError> {
        let cookies_json = serde_json::to_string(&cookies_data.cookies)
            .map_err(|e| StorageError::SerializationError(e.to_string()))?;

        let fetched_at_str = cookies_data.fetched_at.timestamp().to_string();
        let validated_at_str = cookies_data.validated_at.timestamp().to_string();

        Ok((cookies_json, fetched_at_str, validated_at_str))
    }

    /// 保存Cookies到Redis
    ///
    /// Redis数据结构:
    /// - 类型: Hash
    /// - Key: `weibo:cookies:{uid}`
    /// - Fields: `cookies`, `fetched_at`, `validated_at`, `screen_name`
    /// - TTL: 30天
    ///
    /// # 参数
    /// - `cookies_data`: 待保存的cookies数据
    ///
    /// # 返回值
    /// - `Ok(true)`: 覆盖了已存在的数据
    /// - `Ok(false)`: 新建数据
    ///
    /// # 错误
    /// 返回 `StorageError` 如果Redis操作失败
    pub async fn save_cookies(&self, cookies_data: &CookiesData) -> Result<bool, StorageError> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| StorageError::RedisConnectionFailed(e.to_string()))?;

        // 检查是否已存在
        let exists: bool = conn
            .exists(&cookies_data.redis_key)
            .await
            .map_err(|e| StorageError::CommandFailed(e.to_string()))?;

        // 准备字段数据
        let (cookies_json, fetched_at_str, validated_at_str) =
            Self::prepare_redis_fields(cookies_data)?;

        // 保存基础字段
        let fields = vec![
            ("cookies", cookies_json.as_str()),
            ("fetched_at", fetched_at_str.as_str()),
            ("validated_at", validated_at_str.as_str()),
        ];

        redis::pipe()
            .atomic()
            .hset_multiple(&cookies_data.redis_key, &fields)
            .ignore()
            .query_async::<()>(&mut *conn)
            .await
            .map_err(|e| StorageError::CommandFailed(e.to_string()))?;

        // 保存昵称(可选)
        if let Some(ref screen_name) = cookies_data.screen_name {
            conn.hset::<_, _, _, ()>(&cookies_data.redis_key, "screen_name", screen_name)
                .await
                .map_err(|e| StorageError::CommandFailed(e.to_string()))?;
        }

        // 设置30天过期
        const EXPIRE_SECONDS: i64 = 30 * 24 * 3600;
        conn.expire::<_, ()>(&cookies_data.redis_key, EXPIRE_SECONDS)
            .await
            .map_err(|e| StorageError::CommandFailed(e.to_string()))?;

        tracing::info!(
            用户ID = %cookies_data.uid,
            Redis键 = %cookies_data.redis_key,
            是否覆盖 = %exists,
            Cookies样本 = %cookies_data.sample_for_logging(),
            "Cookies已保存到Redis"
        );

        Ok(exists)
    }

    /// 查询Cookies
    ///
    /// # 参数
    /// - `uid`: 微博用户ID
    ///
    /// # 返回值
    /// 完整的 `CookiesData` 结构
    ///
    /// # 错误
    /// - `StorageError::NotFound`: UID不存在
    /// - `StorageError::SerializationError`: 数据格式错误
    /// - `StorageError::RedisConnectionFailed`: 连接失败
    pub async fn query_cookies(&self, uid: &str) -> Result<CookiesData, StorageError> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| StorageError::RedisConnectionFailed(e.to_string()))?;

        let redis_key = format!("weibo:cookies:{}", uid);

        // 检查是否存在
        let exists: bool = conn
            .exists(&redis_key)
            .await
            .map_err(|e| StorageError::CommandFailed(e.to_string()))?;

        if !exists {
            tracing::warn!(用户ID = %uid, "Redis中未找到Cookies");
            return Err(StorageError::NotFound(uid.to_string()));
        }

        // 获取所有字段
        let data: HashMap<String, String> = conn
            .hgetall(&redis_key)
            .await
            .map_err(|e| StorageError::CommandFailed(e.to_string()))?;

        // 反序列化cookies
        let cookies: HashMap<String, String> = serde_json::from_str(
            data.get("cookies")
                .ok_or_else(|| StorageError::SerializationError("Missing cookies field".into()))?,
        )
        .map_err(|e| StorageError::SerializationError(e.to_string()))?;

        // 解析时间戳
        let fetched_at = data
            .get("fetched_at")
            .and_then(|s| s.parse::<i64>().ok())
            .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0))
            .ok_or_else(|| StorageError::SerializationError("Invalid fetched_at".into()))?;

        let validated_at = data
            .get("validated_at")
            .and_then(|s| s.parse::<i64>().ok())
            .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0))
            .ok_or_else(|| StorageError::SerializationError("Invalid validated_at".into()))?;

        let cookies_data = CookiesData {
            uid: uid.to_string(),
            cookies,
            fetched_at,
            validated_at,
            redis_key: redis_key.clone(),
            screen_name: data.get("screen_name").cloned(),
        };

        tracing::debug!(
            用户ID = %uid,
            Redis键 = %redis_key,
            Cookies样本 = %cookies_data.sample_for_logging(),
            "从Redis检索到Cookies"
        );

        Ok(cookies_data)
    }

    /// 删除Cookies
    ///
    /// # 参数
    /// - `uid`: 微博用户ID
    ///
    /// # 错误
    /// 返回 `StorageError` 如果Redis操作失败
    ///
    /// # 注意
    /// 即使UID不存在,也返回成功 (幂等操作)
    pub async fn delete_cookies(&self, uid: &str) -> Result<(), StorageError> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| StorageError::RedisConnectionFailed(e.to_string()))?;

        let redis_key = format!("weibo:cookies:{}", uid);
        conn.del::<_, ()>(&redis_key)
            .await
            .map_err(|e| StorageError::CommandFailed(e.to_string()))?;

        tracing::info!(用户ID = %uid, Redis键 = %redis_key, "已从Redis删除Cookies");
        Ok(())
    }

    /// 列出所有已保存的UID
    ///
    /// 扫描所有 `weibo:cookies:*` key,提取UID列表。
    /// 用于账户管理界面展示。
    ///
    /// # 注意
    /// 使用 `KEYS` 命令。对于微博 cookies 场景（数量通常 <1000），性能影响可忽略。
    /// 若需处理大量数据，可改用 SCAN 迭代器实现。
    pub async fn list_all_uids(&self) -> Result<Vec<String>, StorageError> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| StorageError::RedisConnectionFailed(e.to_string()))?;

        let pattern = "weibo:cookies:*";
        let keys: Vec<String> = redis::cmd("KEYS")
            .arg(pattern)
            .query_async(&mut *conn)
            .await
            .map_err(|e| StorageError::CommandFailed(e.to_string()))?;

        let uids: Vec<String> = keys
            .iter()
            .filter_map(|key| key.strip_prefix("weibo:cookies:").map(String::from))
            .collect();

        tracing::debug!(
            key数量 = %keys.len(),
            uid数量 = %uids.len(),
            "从Redis列出所有UID"
        );
        Ok(uids)
    }

    // ==================== 爬取任务存储方法 (Phase 3.3 - T023) ====================

    /// 保存爬取任务到Redis
    ///
    /// Redis数据结构:
    /// - Key: `crawl:task:{task_id}`
    /// - Type: Hash
    /// - TTL: 90天
    pub async fn save_crawl_task(&self, task: &CrawlTask) -> Result<(), StorageError> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| StorageError::RedisConnectionFailed(e.to_string()))?;

        let fields = vec![
            ("id", task.id.clone()),
            ("keyword", task.keyword.clone()),
            (
                "event_start_time",
                task.event_start_time.timestamp().to_string(),
            ),
            ("status", task.status.as_str().to_string()),
            (
                "min_post_time",
                task.min_post_time
                    .map(|t| t.timestamp().to_string())
                    .unwrap_or_default(),
            ),
            (
                "max_post_time",
                task.max_post_time
                    .map(|t| t.timestamp().to_string())
                    .unwrap_or_default(),
            ),
            ("crawled_count", task.crawled_count.to_string()),
            ("created_at", task.created_at.timestamp().to_string()),
            ("updated_at", task.updated_at.timestamp().to_string()),
            (
                "failure_reason",
                task.failure_reason.clone().unwrap_or_default(),
            ),
        ];

        let fields_refs: Vec<(&str, &str)> = fields.iter().map(|(k, v)| (*k, v.as_str())).collect();

        redis::pipe()
            .atomic()
            .hset_multiple(task.redis_key(), &fields_refs)
            .ignore()
            .query_async::<()>(&mut *conn)
            .await
            .map_err(|e| StorageError::CommandFailed(e.to_string()))?;

        const EXPIRE_SECONDS: i64 = 90 * 24 * 3600;
        conn.expire::<_, ()>(&task.redis_key(), EXPIRE_SECONDS)
            .await
            .map_err(|e| StorageError::CommandFailed(e.to_string()))?;

        tracing::info!(
            任务ID = %task.id,
            关键字 = %task.keyword,
            状态 = %task.status.as_str(),
            "爬取任务已保存到Redis"
        );

        Ok(())
    }

    /// 保存任务关联的cookies
    pub async fn save_task_cookies(
        &self,
        task_id: &str,
        cookies: &HashMap<String, String>,
    ) -> Result<(), StorageError> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| StorageError::RedisConnectionFailed(e.to_string()))?;

        let redis_key = format!("crawl:task:{}:cookies", task_id);
        let cookies_json = serde_json::to_string(cookies)
            .map_err(|e| StorageError::SerializationError(e.to_string()))?;

        conn.set::<_, _, ()>(&redis_key, cookies_json)
            .await
            .map_err(|e| StorageError::CommandFailed(e.to_string()))?;

        // 设置90天TTL
        const EXPIRE_SECONDS: i64 = 90 * 24 * 3600;
        conn.expire::<_, ()>(&redis_key, EXPIRE_SECONDS)
            .await
            .map_err(|e| StorageError::CommandFailed(e.to_string()))?;

        tracing::debug!(
            任务ID = %task_id,
            cookies数量 = %cookies.len(),
            "任务cookies已保存"
        );

        Ok(())
    }

    /// 加载任务关联的cookies
    pub async fn load_task_cookies(
        &self,
        task_id: &str,
    ) -> Result<HashMap<String, String>, StorageError> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| StorageError::RedisConnectionFailed(e.to_string()))?;

        let redis_key = format!("crawl:task:{}:cookies", task_id);

        let cookies_json: String = conn.get(&redis_key).await.map_err(|e| {
            StorageError::NotFound(format!("任务{}的cookies不存在: {}", task_id, e))
        })?;

        let cookies: HashMap<String, String> = serde_json::from_str(&cookies_json)
            .map_err(|e| StorageError::SerializationError(e.to_string()))?;

        tracing::debug!(
            任务ID = %task_id,
            cookies数量 = %cookies.len(),
            "任务cookies已加载"
        );

        Ok(cookies)
    }

    /// 加载爬取任务
    pub async fn load_task(&self, task_id: &str) -> Result<CrawlTask, StorageError> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| StorageError::RedisConnectionFailed(e.to_string()))?;

        let redis_key = format!("crawl:task:{}", task_id);

        let exists: bool = conn
            .exists(&redis_key)
            .await
            .map_err(|e| StorageError::CommandFailed(e.to_string()))?;

        if !exists {
            tracing::warn!(任务ID = %task_id, "Redis中未找到爬取任务");
            return Err(StorageError::NotFound(task_id.to_string()));
        }

        let data: HashMap<String, String> = conn
            .hgetall(&redis_key)
            .await
            .map_err(|e| StorageError::CommandFailed(e.to_string()))?;

        let status_str = data
            .get("status")
            .ok_or_else(|| StorageError::SerializationError("Missing status field".into()))?;

        let status = serde_json::from_str(&format!("\"{}\"", status_str))
            .map_err(|e| StorageError::SerializationError(e.to_string()))?;

        let event_start_time = data
            .get("event_start_time")
            .and_then(|s| s.parse::<i64>().ok())
            .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0))
            .ok_or_else(|| StorageError::SerializationError("Invalid event_start_time".into()))?;

        let min_post_time = data
            .get("min_post_time")
            .filter(|s| !s.is_empty())
            .and_then(|s| s.parse::<i64>().ok())
            .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0));

        let max_post_time = data
            .get("max_post_time")
            .filter(|s| !s.is_empty())
            .and_then(|s| s.parse::<i64>().ok())
            .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0));

        let created_at = data
            .get("created_at")
            .and_then(|s| s.parse::<i64>().ok())
            .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0))
            .ok_or_else(|| StorageError::SerializationError("Invalid created_at".into()))?;

        let updated_at = data
            .get("updated_at")
            .and_then(|s| s.parse::<i64>().ok())
            .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0))
            .ok_or_else(|| StorageError::SerializationError("Invalid updated_at".into()))?;

        let failure_reason = data
            .get("failure_reason")
            .filter(|s| !s.is_empty())
            .cloned();

        let task = CrawlTask {
            id: data
                .get("id")
                .ok_or_else(|| StorageError::SerializationError("Missing id field".into()))?
                .clone(),
            keyword: data
                .get("keyword")
                .ok_or_else(|| StorageError::SerializationError("Missing keyword field".into()))?
                .clone(),
            event_start_time,
            status,
            min_post_time,
            max_post_time,
            crawled_count: data
                .get("crawled_count")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0),
            created_at,
            updated_at,
            failure_reason,
        };

        tracing::debug!(任务ID = %task_id, "从Redis加载爬取任务");
        Ok(task)
    }

    /// 列出所有任务
    pub async fn list_all_tasks(&self) -> Result<Vec<CrawlTask>, StorageError> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| StorageError::RedisConnectionFailed(e.to_string()))?;

        let pattern = "crawl:task:*";
        let keys: Vec<String> = redis::cmd("KEYS")
            .arg(pattern)
            .query_async(&mut *conn)
            .await
            .map_err(|e| StorageError::CommandFailed(e.to_string()))?;

        let mut tasks = Vec::new();
        for key in keys {
            if let Some(task_id) = key.strip_prefix("crawl:task:") {
                if let Ok(task) = self.load_task(task_id).await {
                    tasks.push(task);
                }
            }
        }

        tracing::debug!(任务数量 = %tasks.len(), "从Redis列出所有爬取任务");
        Ok(tasks)
    }

    /// 保存检查点
    ///
    /// Redis数据结构:
    /// - Key: `crawl:checkpoint:{task_id}`
    /// - Type: Hash
    /// - TTL: 与任务同生命周期(90天)
    pub async fn save_checkpoint(&self, checkpoint: &CrawlCheckpoint) -> Result<(), StorageError> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| StorageError::RedisConnectionFailed(e.to_string()))?;

        let completed_shards_json = serde_json::to_string(&checkpoint.completed_shards)
            .map_err(|e| StorageError::SerializationError(e.to_string()))?;

        let direction_str = serde_json::to_string(&checkpoint.direction)
            .map_err(|e| StorageError::SerializationError(e.to_string()))?;

        let fields = vec![
            ("task_id", checkpoint.task_id.clone()),
            (
                "shard_start_time",
                checkpoint.shard_start_time.timestamp().to_string(),
            ),
            (
                "shard_end_time",
                checkpoint.shard_end_time.timestamp().to_string(),
            ),
            ("current_page", checkpoint.current_page.to_string()),
            ("direction", direction_str),
            ("completed_shards", completed_shards_json),
            ("saved_at", checkpoint.saved_at.timestamp().to_string()),
        ];

        let fields_refs: Vec<(&str, &str)> = fields.iter().map(|(k, v)| (*k, v.as_str())).collect();

        redis::pipe()
            .atomic()
            .hset_multiple(checkpoint.redis_key(), &fields_refs)
            .ignore()
            .query_async::<()>(&mut *conn)
            .await
            .map_err(|e| StorageError::CommandFailed(e.to_string()))?;

        const EXPIRE_SECONDS: i64 = 90 * 24 * 3600;
        conn.expire::<_, ()>(&checkpoint.redis_key(), EXPIRE_SECONDS)
            .await
            .map_err(|e| StorageError::CommandFailed(e.to_string()))?;

        tracing::debug!(
            任务ID = %checkpoint.task_id,
            当前页码 = %checkpoint.current_page,
            "检查点已保存到Redis"
        );

        Ok(())
    }

    /// 加载检查点
    pub async fn load_checkpoint(
        &self,
        task_id: &str,
    ) -> Result<Option<CrawlCheckpoint>, StorageError> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| StorageError::RedisConnectionFailed(e.to_string()))?;

        let redis_key = format!("crawl:checkpoint:{}", task_id);

        let exists: bool = conn
            .exists(&redis_key)
            .await
            .map_err(|e| StorageError::CommandFailed(e.to_string()))?;

        if !exists {
            tracing::debug!(任务ID = %task_id, "Redis中未找到检查点");
            return Ok(None);
        }

        let data: HashMap<String, String> = conn
            .hgetall(&redis_key)
            .await
            .map_err(|e| StorageError::CommandFailed(e.to_string()))?;

        let shard_start_time = data
            .get("shard_start_time")
            .and_then(|s| s.parse::<i64>().ok())
            .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0))
            .ok_or_else(|| StorageError::SerializationError("Invalid shard_start_time".into()))?;

        let shard_end_time = data
            .get("shard_end_time")
            .and_then(|s| s.parse::<i64>().ok())
            .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0))
            .ok_or_else(|| StorageError::SerializationError("Invalid shard_end_time".into()))?;

        let saved_at = data
            .get("saved_at")
            .and_then(|s| s.parse::<i64>().ok())
            .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0))
            .ok_or_else(|| StorageError::SerializationError("Invalid saved_at".into()))?;

        let direction = data
            .get("direction")
            .ok_or_else(|| StorageError::SerializationError("Missing direction field".into()))
            .and_then(|s| {
                serde_json::from_str(s).map_err(|e| StorageError::SerializationError(e.to_string()))
            })?;

        let completed_shards = data
            .get("completed_shards")
            .ok_or_else(|| {
                StorageError::SerializationError("Missing completed_shards field".into())
            })
            .and_then(|s| {
                serde_json::from_str(s).map_err(|e| StorageError::SerializationError(e.to_string()))
            })?;

        let checkpoint = CrawlCheckpoint {
            task_id: data
                .get("task_id")
                .ok_or_else(|| StorageError::SerializationError("Missing task_id field".into()))?
                .clone(),
            shard_start_time,
            shard_end_time,
            current_page: data
                .get("current_page")
                .and_then(|s| s.parse().ok())
                .unwrap_or(1),
            direction,
            completed_shards,
            saved_at,
        };

        tracing::debug!(任务ID = %task_id, "从Redis加载检查点");
        Ok(Some(checkpoint))
    }

    /// 批量保存帖子
    ///
    /// Redis数据结构:
    /// 1. 帖子内容 - Sorted Set: `crawl:posts:{task_id}`
    ///    - Score: 帖子发布时间戳
    ///    - Member: 序列化的WeiboPost JSON
    /// 2. 去重索引 - Set: `crawl:post_ids:{task_id}`
    ///    - Members: 所有帖子ID
    pub async fn save_posts(&self, task_id: &str, posts: &[WeiboPost]) -> Result<(), StorageError> {
        if posts.is_empty() {
            return Ok(());
        }

        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| StorageError::RedisConnectionFailed(e.to_string()))?;

        let posts_key = format!("crawl:posts:{}", task_id);
        let ids_key = format!("crawl:post_ids:{}", task_id);

        let mut pipe = redis::pipe();
        pipe.atomic();

        for post in posts {
            let json = post
                .to_json()
                .map_err(|e| StorageError::SerializationError(e.to_string()))?;

            let score = post.created_at.timestamp();

            pipe.zadd(&posts_key, json, score).ignore();
            pipe.sadd(&ids_key, &post.id).ignore();
        }

        pipe.query_async::<()>(&mut *conn)
            .await
            .map_err(|e| StorageError::CommandFailed(e.to_string()))?;

        const EXPIRE_SECONDS: i64 = 90 * 24 * 3600;
        conn.expire::<_, ()>(&posts_key, EXPIRE_SECONDS)
            .await
            .map_err(|e| StorageError::CommandFailed(e.to_string()))?;
        conn.expire::<_, ()>(&ids_key, EXPIRE_SECONDS)
            .await
            .map_err(|e| StorageError::CommandFailed(e.to_string()))?;

        tracing::info!(
            任务ID = %task_id,
            帖子数量 = %posts.len(),
            "批量保存帖子到Redis"
        );

        Ok(())
    }

    /// 按时间范围查询帖子
    ///
    /// 使用ZRANGEBYSCORE命令按时间戳范围查询
    pub async fn get_posts_by_time_range(
        &self,
        task_id: &str,
        start: chrono::DateTime<chrono::Utc>,
        end: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<WeiboPost>, StorageError> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| StorageError::RedisConnectionFailed(e.to_string()))?;

        let posts_key = format!("crawl:posts:{}", task_id);

        let start_score = start.timestamp();
        let end_score = end.timestamp();

        let json_strings: Vec<String> = redis::cmd("ZRANGEBYSCORE")
            .arg(&posts_key)
            .arg(start_score)
            .arg(end_score)
            .query_async(&mut *conn)
            .await
            .map_err(|e| StorageError::CommandFailed(e.to_string()))?;

        let mut posts = Vec::new();
        for json in json_strings {
            match WeiboPost::from_json(&json) {
                Ok(post) => posts.push(post),
                Err(e) => {
                    tracing::warn!(错误 = %e, "反序列化帖子JSON失败");
                }
            }
        }

        tracing::debug!(
            任务ID = %task_id,
            查询到帖子数量 = %posts.len(),
            "按时间范围查询帖子"
        );

        Ok(posts)
    }

    /// 检查帖子是否已存在
    ///
    /// 使用SISMEMBER命令检查去重索引
    pub async fn check_post_exists(
        &self,
        task_id: &str,
        post_id: &str,
    ) -> Result<bool, StorageError> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| StorageError::RedisConnectionFailed(e.to_string()))?;

        let ids_key = format!("crawl:post_ids:{}", task_id);

        let exists: bool = conn
            .sismember(&ids_key, post_id)
            .await
            .map_err(|e| StorageError::CommandFailed(e.to_string()))?;

        Ok(exists)
    }

    /// 获取Redis连接 (测试辅助方法)
    pub async fn get_connection(&self) -> Result<deadpool_redis::Connection, StorageError> {
        self.pool
            .get()
            .await
            .map_err(|e| StorageError::RedisConnectionFailed(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // 需要Redis实例
    async fn test_save_and_query_cookies() {
        let service = RedisService::new("redis://localhost:6379").unwrap();

        let mut cookies = HashMap::new();
        cookies.insert("SUB".to_string(), "test_sub".to_string());
        cookies.insert("SUBP".to_string(), "test_subp".to_string());

        let cookies_data = CookiesData::new("test_uid_123".to_string(), cookies)
            .with_screen_name("测试用户".to_string());

        // 保存
        let is_overwrite = service.save_cookies(&cookies_data).await.unwrap();
        assert!(!is_overwrite);

        // 查询
        let retrieved = service.query_cookies("test_uid_123").await.unwrap();
        assert_eq!(retrieved.uid, "test_uid_123");
        assert_eq!(retrieved.screen_name, Some("测试用户".to_string()));
        assert_eq!(retrieved.cookies.get("SUB"), Some(&"test_sub".to_string()));

        // 清理
        service.delete_cookies("test_uid_123").await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_delete_nonexistent() {
        let service = RedisService::new("redis://localhost:6379").unwrap();
        let result = service.delete_cookies("nonexistent_uid").await;
        assert!(result.is_ok()); // 幂等操作
    }
}
