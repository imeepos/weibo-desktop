use crate::services::{
    CrawlService, RedisService, SessionManager, TimeShardService, ValidationService,
    WeiboApiClient,
};
use std::sync::Arc;

/// 应用全局状态
///
/// 存在即合理: 每个字段代表应用核心能力的单一来源
/// - redis: 数据持久化
/// - weibo_api: 微博平台交互 (Playwright自动化)
/// - validator: Cookies可信度保障
/// - session_manager: 二维码会话生命周期管理
/// - crawl_service: 微博爬取任务协调器 (003功能)
pub struct AppState {
    /// Redis服务: 唯一的数据存储入口
    pub redis: Arc<RedisService>,

    /// 微博API客户端: 唯一的微博平台通信渠道 (Playwright实现)
    pub weibo_api: Arc<WeiboApiClient>,

    /// Cookies验证服务: 唯一的可信度检验机制
    pub validator: Arc<ValidationService>,

    /// 会话管理器: 防止资源泄露的看守者
    pub session_manager: Arc<SessionManager>,

    /// 爬取服务: 微博爬取任务协调器 (003功能)
    pub crawl_service: Arc<CrawlService>,
}

impl AppState {
    /// 初始化应用状态
    ///
    /// 三个参数,三个核心能力,缺一不可:
    /// - redis_url: 数据根基
    /// - playwright_server_url: Playwright WebSocket server地址
    /// - playwright_validation_script: 验证工具
    ///
    /// # 错误处理
    /// 任何服务初始化失败都将导致整个应用无法启动 - 这是必然,因为不完整的状态等同于无用
    pub fn new(
        redis_url: &str,
        playwright_server_url: &str,
        playwright_validation_script: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let redis = Arc::new(RedisService::new(redis_url)?);
        let weibo_api = Arc::new(WeiboApiClient::new(playwright_server_url.to_string()));
        let validator = Arc::new(ValidationService::new(
            playwright_validation_script.to_string(),
        ));
        let session_manager = Arc::new(SessionManager::new());

        // 初始化003功能的服务
        let time_shard_service = Arc::new(TimeShardService::new());
        let crawl_service = Arc::new(CrawlService::new(
            Arc::clone(&redis),
            time_shard_service,
        ));

        tracing::info!(
            redis_url = %redis_url,
            playwright_server = %playwright_server_url,
            playwright_validation = %playwright_validation_script,
            "AppState initialized with session manager and crawl service"
        );

        Ok(Self {
            redis,
            weibo_api,
            validator,
            session_manager,
            crawl_service,
        })
    }
}
