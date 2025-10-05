use deadpool_redis::{Config, Pool, Runtime};
use redis::AsyncCommands;
use std::collections::HashMap;

use crate::models::{CookiesData, StorageError};

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
                redis_url = %redis_url,
                error = %e,
                "Failed to create Redis connection pool"
            );
            StorageError::RedisConnectionFailed(e.to_string())
        })?;

        tracing::info!(redis_url = %redis_url, "Redis connection pool created");
        Ok(Self { pool })
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

        // 序列化cookies为JSON
        let cookies_json = serde_json::to_string(&cookies_data.cookies)
            .map_err(|e| StorageError::SerializationError(e.to_string()))?;

        // 准备时间戳字符串 (需要保持在作用域内)
        let fetched_at_str = cookies_data.fetched_at.timestamp().to_string();
        let validated_at_str = cookies_data.validated_at.timestamp().to_string();

        // 使用HSET保存所有字段
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

        // 如果有昵称,也保存
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
            uid = %cookies_data.uid,
            redis_key = %cookies_data.redis_key,
            is_overwrite = %exists,
            cookies_sample = %cookies_data.sample_for_logging(),
            "Cookies saved to Redis"
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
            tracing::warn!(uid = %uid, "Cookies not found in Redis");
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
            uid = %uid,
            redis_key = %redis_key,
            cookies_sample = %cookies_data.sample_for_logging(),
            "Cookies retrieved from Redis"
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

        tracing::info!(uid = %uid, redis_key = %redis_key, "Cookies deleted from Redis");
        Ok(())
    }

    /// 列出所有已保存的UID
    ///
    /// 扫描所有 `weibo:cookies:*` key,提取UID列表。
    /// 用于账户管理界面展示。
    ///
    /// # 注意
    /// 使用 `SCAN` 而非 `KEYS`,避免阻塞Redis
    pub async fn list_all_uids(&self) -> Result<Vec<String>, StorageError> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| StorageError::RedisConnectionFailed(e.to_string()))?;

        let pattern = "weibo:cookies:*";
        let keys: Vec<String> = redis::cmd("SCAN")
            .arg("0")
            .arg("MATCH")
            .arg(pattern)
            .arg("COUNT")
            .arg(100)
            .query_async(&mut *conn)
            .await
            .map_err(|e| StorageError::CommandFailed(e.to_string()))?;

        let uids = keys
            .iter()
            .filter_map(|key| key.strip_prefix("weibo:cookies:").map(String::from))
            .collect();

        tracing::debug!(count = keys.len(), "Listed all UIDs from Redis");
        Ok(uids)
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
