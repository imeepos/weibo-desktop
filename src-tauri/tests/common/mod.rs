//! 测试公共模块
//!
//! 提供Mock服务和测试工具,遵循优雅即简约的原则。
//! 每个Mock都服务于契约测试,避免外部依赖。

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

// 导入依赖模型
use weibo_login::models::dependency::CheckStatus;

/// Mock Redis服务
///
/// 内存实现的Redis,支持基本操作:
/// - SET/GET: 字符串操作
/// - HSET/HGETALL: Hash操作
/// - EXISTS: 键存在检查
/// - DEL: 删除键
#[allow(dead_code)]
pub struct MockRedisService {
    /// 内存存储 (键 -> JSON字符串)
    storage: Arc<Mutex<HashMap<String, String>>>,
    /// Hash存储 (键 -> HashMap)
    hash_storage: Arc<Mutex<HashMap<String, HashMap<String, String>>>>,
    /// 连接失败模拟开关
    should_fail: Arc<Mutex<bool>>,
}

#[allow(dead_code)]
impl MockRedisService {
    /// 创建新的Mock Redis服务
    pub fn new() -> Self {
        Self {
            storage: Arc::new(Mutex::new(HashMap::new())),
            hash_storage: Arc::new(Mutex::new(HashMap::new())),
            should_fail: Arc::new(Mutex::new(false)),
        }
    }

    /// 设置字符串值
    pub async fn set(&self, key: String, value: String) -> Result<(), String> {
        if *self.should_fail.lock().await {
            return Err("Redis连接失败".to_string());
        }
        self.storage.lock().await.insert(key, value);
        Ok(())
    }

    /// 获取字符串值
    pub async fn get(&self, key: &str) -> Result<Option<String>, String> {
        if *self.should_fail.lock().await {
            return Err("Redis连接失败".to_string());
        }
        Ok(self.storage.lock().await.get(key).cloned())
    }

    /// HSET操作 (设置Hash字段)
    pub async fn hset(&self, key: &str, field: &str, value: String) -> Result<(), String> {
        if *self.should_fail.lock().await {
            return Err("Redis连接失败".to_string());
        }
        let mut hash_storage = self.hash_storage.lock().await;
        hash_storage
            .entry(key.to_string())
            .or_insert_with(HashMap::new)
            .insert(field.to_string(), value);
        Ok(())
    }

    /// HGET操作 (获取Hash单个字段)
    pub async fn hget(&self, key: &str, field: &str) -> Result<Option<String>, String> {
        if *self.should_fail.lock().await {
            return Err("Redis连接失败".to_string());
        }
        Ok(self
            .hash_storage
            .lock()
            .await
            .get(key)
            .and_then(|hash| hash.get(field).cloned()))
    }

    /// HGETALL操作 (获取Hash所有字段)
    pub async fn hgetall(&self, key: &str) -> Result<HashMap<String, String>, String> {
        if *self.should_fail.lock().await {
            return Err("Redis连接失败".to_string());
        }
        Ok(self
            .hash_storage
            .lock()
            .await
            .get(key)
            .cloned()
            .unwrap_or_default())
    }

    /// EXISTS操作 (检查键是否存在)
    pub async fn exists(&self, key: &str) -> Result<bool, String> {
        if *self.should_fail.lock().await {
            return Err("Redis连接失败".to_string());
        }
        let storage = self.storage.lock().await;
        let hash_storage = self.hash_storage.lock().await;
        Ok(storage.contains_key(key) || hash_storage.contains_key(key))
    }

    /// DEL操作 (删除键)
    pub async fn delete(&self, key: &str) -> Result<(), String> {
        if *self.should_fail.lock().await {
            return Err("Redis连接失败".to_string());
        }
        self.storage.lock().await.remove(key);
        self.hash_storage.lock().await.remove(key);
        Ok(())
    }

    /// 设置失败模式 (模拟连接失败)
    pub async fn set_fail_mode(&self, should_fail: bool) {
        *self.should_fail.lock().await = should_fail;
    }

    /// 清空所有数据
    pub async fn clear(&self) {
        self.storage.lock().await.clear();
        self.hash_storage.lock().await.clear();
    }

    /// KEYS操作 (获取匹配的键列表)
    pub async fn keys(&self, pattern: &str) -> Result<Vec<String>, String> {
        if *self.should_fail.lock().await {
            return Err("Redis连接失败".to_string());
        }

        let hash_storage = self.hash_storage.lock().await;

        // 简单的模式匹配: "crawl:task:*"
        let prefix = pattern.trim_end_matches('*');
        let keys: Vec<String> = hash_storage
            .keys()
            .filter(|k| k.starts_with(prefix))
            .cloned()
            .collect();

        Ok(keys)
    }

    /// 插入损坏数据 (用于测试反序列化错误)
    pub async fn insert_corrupted_data(&self, key: &str) -> Result<(), String> {
        if *self.should_fail.lock().await {
            return Err("Redis连接失败".to_string());
        }
        let mut hash_storage = self.hash_storage.lock().await;
        let mut corrupted = HashMap::new();
        corrupted.insert("cookies".to_string(), "invalid json {{{".to_string());
        corrupted.insert("fetched_at".to_string(), "not a timestamp".to_string());
        hash_storage.insert(key.to_string(), corrupted);
        Ok(())
    }
}

impl Default for MockRedisService {
    fn default() -> Self {
        Self::new()
    }
}

/// Mock Playwright验证服务
///
/// 模拟浏览器自动化验证cookies有效性。
/// 可配置成功/失败模式,用于测试不同验证场景。
#[allow(dead_code)]
pub struct MockValidationService {
    /// 是否应该验证成功
    should_succeed: bool,
    /// 模拟返回的UID
    mock_uid: String,
    /// 模拟返回的用户昵称
    mock_screen_name: String,
}

#[allow(dead_code)]
impl MockValidationService {
    /// 创建验证成功的Mock服务
    pub fn new_success() -> Self {
        Self {
            should_succeed: true,
            mock_uid: "1234567890".to_string(),
            mock_screen_name: "测试用户".to_string(),
        }
    }

    /// 创建验证失败的Mock服务
    pub fn new_failure() -> Self {
        Self {
            should_succeed: false,
            mock_uid: String::new(),
            mock_screen_name: String::new(),
        }
    }

    /// 创建自定义Mock服务
    pub fn new(should_succeed: bool, uid: String, screen_name: String) -> Self {
        Self {
            should_succeed,
            mock_uid: uid,
            mock_screen_name: screen_name,
        }
    }

    /// 验证cookies (模拟Playwright调用微博资料API)
    ///
    /// # 返回
    /// - 成功: Ok((uid, screen_name))
    /// - 失败: Err(错误消息)
    pub async fn validate(&self) -> Result<(String, String), String> {
        // 模拟网络延迟
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        if self.should_succeed {
            Ok((self.mock_uid.clone(), self.mock_screen_name.clone()))
        } else {
            Err("Profile API call failed with status 401".to_string())
        }
    }

    /// 设置验证结果
    pub fn set_result(&mut self, should_succeed: bool) {
        self.should_succeed = should_succeed;
    }

    /// 设置模拟数据
    pub fn set_mock_data(&mut self, uid: String, screen_name: String) {
        self.mock_uid = uid;
        self.mock_screen_name = screen_name;
    }
}

/// 创建测试用的cookies
///
/// 包含微博登录所需的必需字段: SUB, SUBP
/// 以及常见的可选字段: _T_WM, XSRF-TOKEN
pub fn create_test_cookies() -> HashMap<String, String> {
    let mut cookies = HashMap::new();
    cookies.insert("SUB".to_string(), "test_sub_value_123".to_string());
    cookies.insert("SUBP".to_string(), "test_subp_value_456".to_string());
    cookies.insert("_T_WM".to_string(), "test_twm_value_789".to_string());
    cookies.insert(
        "XSRF-TOKEN".to_string(),
        "test_xsrf_token_abc".to_string(),
    );
    cookies
}

/// 创建只有必需字段的最小cookies
pub fn create_minimal_cookies() -> HashMap<String, String> {
    let mut cookies = HashMap::new();
    cookies.insert("SUB".to_string(), "minimal_sub".to_string());
    cookies.insert("SUBP".to_string(), "minimal_subp".to_string());
    cookies
}

/// 创建缺少必需字段的无效cookies
pub fn create_invalid_cookies() -> HashMap<String, String> {
    let mut cookies = HashMap::new();
    cookies.insert("SUB".to_string(), "only_sub".to_string());
    // 缺少 SUBP
    cookies
}

/// Mock依赖检测服务
///
/// 模拟系统依赖检测过程,可配置依赖满足状态
/// 用于测试不同依赖场景的集成测试
#[allow(dead_code)]
pub struct MockDependencyChecker {
    /// Redis服务状态
    pub redis_satisfied: bool,
    /// Redis版本
    pub redis_version: String,
    /// Playwright状态
    pub playwright_satisfied: bool,
    /// Playwright版本
    pub playwright_version: String,
    /// 检测延迟(毫秒)
    pub check_delay_ms: u64,
}

#[allow(dead_code)]
impl MockDependencyChecker {
    /// 创建所有依赖都满足的Mock服务
    pub fn new_all_satisfied() -> Self {
        Self {
            redis_satisfied: true,
            redis_version: "7.2.4".to_string(),
            playwright_satisfied: true,
            playwright_version: "1.48.0".to_string(),
            check_delay_ms: 50, // 模拟快速检测
        }
    }

    /// 创建Redis缺失的Mock服务
    pub fn new_redis_missing() -> Self {
        Self {
            redis_satisfied: false,
            redis_version: String::new(),
            playwright_satisfied: true,
            playwright_version: "1.48.0".to_string(),
            check_delay_ms: 100,
        }
    }

    /// 创建Playwright缺失的Mock服务
    pub fn new_playwright_missing() -> Self {
        Self {
            redis_satisfied: true,
            redis_version: "7.2.4".to_string(),
            playwright_satisfied: false,
            playwright_version: String::new(),
            check_delay_ms: 100,
        }
    }

    /// 创建自定义配置的Mock服务
    pub fn new(
        redis_satisfied: bool,
        redis_version: String,
        playwright_satisfied: bool,
        playwright_version: String,
        check_delay_ms: u64,
    ) -> Self {
        Self {
            redis_satisfied,
            redis_version,
            playwright_satisfied,
            playwright_version,
            check_delay_ms,
        }
    }

    /// 检测所有依赖项
    pub async fn check_all(&self) -> Vec<MockDependencyResult> {
        // 模拟检测延迟
        tokio::time::sleep(tokio::time::Duration::from_millis(self.check_delay_ms)).await;

        let mut results = Vec::new();

        // 检测Redis
        let redis_result = MockDependencyResult {
            dependency_id: "redis".to_string(),
            dependency_name: "Redis Server".to_string(),
            status: if self.redis_satisfied {
                CheckStatus::Satisfied
            } else {
                CheckStatus::Missing
            },
            detected_version: if self.redis_satisfied {
                Some(self.redis_version.clone())
            } else {
                None
            },
            error_details: if !self.redis_satisfied {
                Some("Redis service not reachable at localhost:6379".to_string())
            } else {
                None
            },
            duration_ms: self.check_delay_ms / 2,
            checked_at: chrono::Utc::now(),
        };
        results.push(redis_result);

        // 检测Playwright
        let playwright_result = MockDependencyResult {
            dependency_id: "playwright".to_string(),
            dependency_name: "Playwright".to_string(),
            status: if self.playwright_satisfied {
                CheckStatus::Satisfied
            } else {
                CheckStatus::Missing
            },
            detected_version: if self.playwright_satisfied {
                Some(self.playwright_version.clone())
            } else {
                None
            },
            error_details: if !self.playwright_satisfied {
                Some("Playwright not found in node_modules".to_string())
            } else {
                None
            },
            duration_ms: self.check_delay_ms / 2,
            checked_at: chrono::Utc::now(),
        };
        results.push(playwright_result);

        results
    }

    /// 获取总检测耗时
    pub fn get_total_duration(&self) -> u64 {
        self.check_delay_ms
    }
}

/// Mock依赖检测结果
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct MockDependencyResult {
    pub dependency_id: String,
    pub dependency_name: String,
    pub status: CheckStatus,
    pub detected_version: Option<String>,
    pub error_details: Option<String>,
    pub duration_ms: u64,
    pub checked_at: chrono::DateTime<chrono::Utc>,
}

impl MockDependencyResult {
    /// 转换为契约定义的DependencyCheckResult
    pub fn to_contract_result(&self) -> weibo_login::models::dependency::DependencyCheckResult {
        weibo_login::models::dependency::DependencyCheckResult {
            dependency_id: self.dependency_id.clone(),
            checked_at: self.checked_at,
            status: self.status.clone(),
            detected_version: self.detected_version.clone(),
            error_details: self.error_details.clone(),
            duration_ms: self.duration_ms,
        }
    }
}

impl Default for MockDependencyChecker {
    fn default() -> Self {
        Self::new_all_satisfied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_redis_set_get() {
        let redis = MockRedisService::new();
        redis
            .set("test_key".to_string(), "test_value".to_string())
            .await
            .unwrap();
        let value = redis.get("test_key").await.unwrap();
        assert_eq!(value, Some("test_value".to_string()));
    }

    #[tokio::test]
    async fn test_mock_redis_hset_hget() {
        let redis = MockRedisService::new();
        redis
            .hset("hash_key", "field1", "value1".to_string())
            .await
            .unwrap();

        let value = redis.hget("hash_key", "field1").await.unwrap();
        assert_eq!(value, Some("value1".to_string()));

        let nonexistent = redis.hget("hash_key", "nonexistent").await.unwrap();
        assert_eq!(nonexistent, None);
    }

    #[tokio::test]
    async fn test_mock_redis_hset_hgetall() {
        let redis = MockRedisService::new();
        redis
            .hset("hash_key", "field1", "value1".to_string())
            .await
            .unwrap();
        redis
            .hset("hash_key", "field2", "value2".to_string())
            .await
            .unwrap();

        let hash = redis.hgetall("hash_key").await.unwrap();
        assert_eq!(hash.get("field1"), Some(&"value1".to_string()));
        assert_eq!(hash.get("field2"), Some(&"value2".to_string()));
    }

    #[tokio::test]
    async fn test_mock_redis_exists() {
        let redis = MockRedisService::new();
        assert!(!redis.exists("nonexistent").await.unwrap());

        redis
            .hset("test_key", "field", "value".to_string())
            .await
            .unwrap();
        assert!(redis.exists("test_key").await.unwrap());
    }

    #[tokio::test]
    async fn test_mock_redis_fail_mode() {
        let redis = MockRedisService::new();
        redis.set_fail_mode(true).await;

        let result = redis.set("key".to_string(), "value".to_string()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mock_validation_success() {
        let validator = MockValidationService::new_success();
        let result = validator.validate().await;
        assert!(result.is_ok());
        let (uid, screen_name) = result.unwrap();
        assert_eq!(uid, "1234567890");
        assert_eq!(screen_name, "测试用户");
    }

    #[tokio::test]
    async fn test_mock_validation_failure() {
        let validator = MockValidationService::new_failure();
        let result = validator.validate().await;
        assert!(result.is_err());
    }

    #[test]
    fn test_create_test_cookies() {
        let cookies = create_test_cookies();
        assert!(cookies.contains_key("SUB"));
        assert!(cookies.contains_key("SUBP"));
        assert_eq!(cookies.len(), 4);
    }

    #[test]
    fn test_create_minimal_cookies() {
        let cookies = create_minimal_cookies();
        assert_eq!(cookies.len(), 2);
        assert!(cookies.contains_key("SUB"));
        assert!(cookies.contains_key("SUBP"));
    }

    #[test]
    fn test_create_invalid_cookies() {
        let cookies = create_invalid_cookies();
        assert!(cookies.contains_key("SUB"));
        assert!(!cookies.contains_key("SUBP"));
    }
}
