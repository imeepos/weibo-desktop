use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::models::errors::ValidationError;

/// Cookies数据
///
/// 存储从微博获取的登录凭证,支持验证、持久化和安全日志。
/// 每个字段都不可替代,服务于凭证管理、过期判断和用户体验。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CookiesData {
    /// 微博用户ID
    pub uid: String,

    /// Cookie键值对 (如: SUB, SUBP, _T_WM等)
    pub cookies: HashMap<String, String>,

    /// 获取时间
    pub fetched_at: DateTime<Utc>,

    /// 验证时间 (通过微博资料API验证的时间)
    pub validated_at: DateTime<Utc>,

    /// Redis存储key (格式: weibo:cookies:{uid})
    pub redis_key: String,

    /// 用户昵称 (可选,验证时从API获取)
    pub screen_name: Option<String>,
}

impl CookiesData {
    /// 创建新的Cookies数据
    ///
    /// # 参数
    /// - `uid`: 微博用户ID
    /// - `cookies`: Cookie键值对
    ///
    /// # 示例
    /// ```
    /// use std::collections::HashMap;
    /// let mut cookies = HashMap::new();
    /// cookies.insert("SUB".to_string(), "xxx".to_string());
    /// cookies.insert("SUBP".to_string(), "yyy".to_string());
    ///
    /// let data = CookiesData::new("1234567890".to_string(), cookies);
    /// assert_eq!(data.redis_key, "weibo:cookies:1234567890");
    /// ```
    pub fn new(uid: String, cookies: HashMap<String, String>) -> Self {
        let now = Utc::now();
        Self {
            redis_key: format!("weibo:cookies:{}", uid),
            uid,
            cookies,
            fetched_at: now,
            validated_at: now,
            screen_name: None,
        }
    }

    /// 验证必需的cookie字段
    ///
    /// 检查规则:
    /// 1. cookies不为空
    /// 2. 必须包含 `SUB` 字段
    /// 3. 必须包含 `SUBP` 字段
    ///
    /// # 错误
    /// 返回 `ValidationError::MissingCookie` 如果缺少必需字段
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.cookies.is_empty() {
            return Err(ValidationError::InvalidFormat(
                "Cookies不能为空".to_string(),
            ));
        }

        const REQUIRED_COOKIES: &[&str] = &["SUB", "SUBP"];

        for &cookie_name in REQUIRED_COOKIES {
            if !self.cookies.contains_key(cookie_name) {
                return Err(ValidationError::MissingCookie(cookie_name.to_string()));
            }
        }

        Ok(())
    }

    /// 获取cookies的样本 (用于日志,不记录值)
    ///
    /// 遵循宪章原则五: 日志不记录敏感数据。
    /// 仅返回cookie的键名,不包含实际值。
    ///
    /// # 示例
    /// ```
    /// // 输出: "SUB, SUBP, _T_WM"
    /// let sample = data.sample_for_logging();
    /// ```
    pub fn sample_for_logging(&self) -> String {
        let mut keys: Vec<&String> = self.cookies.keys().collect();
        keys.sort();
        keys.iter()
            .map(|k| k.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// 转换为cookie header格式
    ///
    /// 用于构造HTTP请求的 `Cookie` header。
    ///
    /// # 示例
    /// ```
    /// // 输出: "SUB=xxx; SUBP=yyy; _T_WM=zzz"
    /// let header = data.to_cookie_header();
    /// ```
    pub fn to_cookie_header(&self) -> String {
        self.cookies
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("; ")
    }

    /// 设置用户昵称 (构建器模式)
    ///
    /// # 示例
    /// ```
    /// let data = CookiesData::new(uid, cookies)
    ///     .with_screen_name("张三".to_string());
    /// ```
    pub fn with_screen_name(mut self, screen_name: String) -> Self {
        self.screen_name = Some(screen_name);
        self
    }

    /// 获取cookie数量
    ///
    /// 用于日志统计和验证。
    pub fn cookie_count(&self) -> usize {
        self.cookies.len()
    }

    /// 检查是否包含指定cookie
    pub fn contains_cookie(&self, name: &str) -> bool {
        self.cookies.contains_key(name)
    }

    /// 获取指定cookie的值
    ///
    /// 注意: 仅在内部使用,不应记录到日志。
    pub fn get_cookie(&self, name: &str) -> Option<&String> {
        self.cookies.get(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_cookies() -> HashMap<String, String> {
        let mut cookies = HashMap::new();
        cookies.insert("SUB".to_string(), "xxx_value".to_string());
        cookies.insert("SUBP".to_string(), "yyy_value".to_string());
        cookies.insert("_T_WM".to_string(), "zzz_value".to_string());
        cookies
    }

    #[test]
    fn test_new_cookies_data() {
        let cookies = create_test_cookies();
        let data = CookiesData::new("1234567890".to_string(), cookies);

        assert_eq!(data.uid, "1234567890");
        assert_eq!(data.redis_key, "weibo:cookies:1234567890");
        assert_eq!(data.cookie_count(), 3);
        assert!(data.screen_name.is_none());
    }

    #[test]
    fn test_validate_success() {
        let cookies = create_test_cookies();
        let data = CookiesData::new("1234567890".to_string(), cookies);
        assert!(data.validate().is_ok());
    }

    #[test]
    fn test_validate_missing_required_cookie() {
        let mut cookies = HashMap::new();
        cookies.insert("SUB".to_string(), "xxx".to_string());
        // 缺少 SUBP

        let data = CookiesData::new("1234567890".to_string(), cookies);
        let result = data.validate();
        assert!(result.is_err());
        if let Err(ValidationError::MissingCookie(name)) = result {
            assert_eq!(name, "SUBP");
        } else {
            panic!("Expected MissingCookie error");
        }
    }

    #[test]
    fn test_validate_empty_cookies() {
        let cookies = HashMap::new();
        let data = CookiesData::new("1234567890".to_string(), cookies);
        let result = data.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_sample_for_logging() {
        let cookies = create_test_cookies();
        let data = CookiesData::new("1234567890".to_string(), cookies);
        let sample = data.sample_for_logging();

        // 验证不包含实际值
        assert!(!sample.contains("xxx_value"));
        assert!(!sample.contains("yyy_value"));

        // 验证包含键名
        assert!(sample.contains("SUB"));
        assert!(sample.contains("SUBP"));
        assert!(sample.contains("_T_WM"));
    }

    #[test]
    fn test_to_cookie_header() {
        let mut cookies = HashMap::new();
        cookies.insert("SUB".to_string(), "xxx".to_string());
        cookies.insert("SUBP".to_string(), "yyy".to_string());

        let data = CookiesData::new("1234567890".to_string(), cookies);
        let header = data.to_cookie_header();

        assert!(header.contains("SUB=xxx"));
        assert!(header.contains("SUBP=yyy"));
        assert!(header.contains("; "));
    }

    #[test]
    fn test_with_screen_name() {
        let cookies = create_test_cookies();
        let data = CookiesData::new("1234567890".to_string(), cookies)
            .with_screen_name("测试用户".to_string());

        assert_eq!(data.screen_name, Some("测试用户".to_string()));
    }

    #[test]
    fn test_contains_cookie() {
        let cookies = create_test_cookies();
        let data = CookiesData::new("1234567890".to_string(), cookies);

        assert!(data.contains_cookie("SUB"));
        assert!(data.contains_cookie("SUBP"));
        assert!(!data.contains_cookie("NONEXISTENT"));
    }

    #[test]
    fn test_get_cookie() {
        let cookies = create_test_cookies();
        let data = CookiesData::new("1234567890".to_string(), cookies);

        assert_eq!(data.get_cookie("SUB"), Some(&"xxx_value".to_string()));
        assert_eq!(data.get_cookie("NONEXISTENT"), None);
    }
}
