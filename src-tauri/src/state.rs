use crate::services::{RedisService, ValidationService, WeiboApiClient};
use std::sync::Arc;

/// 应用全局状态
///
/// 存在即合理: 每个字段代表应用核心能力的单一来源
/// - redis: 数据持久化
/// - weibo_api: 微博平台交互 (Playwright自动化)
/// - validator: Cookies可信度保障
///
/// Arc保证多线程安全访问,无需Mutex因为服务自身已处理并发
pub struct AppState {
    /// Redis服务: 唯一的数据存储入口
    pub redis: Arc<RedisService>,

    /// 微博API客户端: 唯一的微博平台通信渠道 (Playwright实现)
    pub weibo_api: Arc<WeiboApiClient>,

    /// Cookies验证服务: 唯一的可信度检验机制
    pub validator: Arc<ValidationService>,
}

impl AppState {
    /// 初始化应用状态
    ///
    /// 三个参数,三个核心能力,缺一不可:
    /// - redis_url: 数据根基
    /// - playwright_login_script: 登录自动化工具
    /// - playwright_validation_script: 验证工具
    ///
    /// # 错误处理
    /// 任何服务初始化失败都将导致整个应用无法启动 - 这是必然,因为不完整的状态等同于无用
    pub fn new(
        redis_url: &str,
        playwright_login_script: &str,
        playwright_validation_script: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let redis = Arc::new(RedisService::new(redis_url)?);
        let weibo_api = Arc::new(WeiboApiClient::new(
            playwright_login_script.to_string(),
        ));
        let validator = Arc::new(ValidationService::new(
            playwright_validation_script.to_string(),
        ));

        tracing::info!(
            redis_url = %redis_url,
            playwright_login = %playwright_login_script,
            playwright_validation = %playwright_validation_script,
            "AppState initialized (Playwright mode)"
        );

        Ok(Self {
            redis,
            weibo_api,
            validator,
        })
    }
}
