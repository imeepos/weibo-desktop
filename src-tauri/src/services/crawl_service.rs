//! 爬取服务
//!
//! 核心职责:
//! - 协调历史回溯和增量更新的爬取流程
//! - 管理后台任务生命周期(启动/暂停/恢复/取消)
//! - 推送实时进度事件到前端
//! - 调用Playwright脚本执行实际爬取
//! - 保存检查点支持断点续爬

use crate::models::crawl_events::{
    CrawlErrorEvent, CrawlProgressEvent, CrawlStatus, ErrorCode, TimeRange,
};
use crate::models::{CrawlCheckpoint, CrawlDirection, WeiboPost};
use crate::services::RedisService;
use crate::utils::time_utils::{ceil_to_hour, floor_to_hour, format_weibo_time};
use chrono::{DateTime, Utc};
use rand::Rng;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::Emitter; // Trait for emit method
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

/// Playwright爬取请求
///
/// 通过WebSocket发送到Playwright服务器的爬取请求
/// 字段使用camelCase以对齐TypeScript端
#[derive(Debug, Clone, serde::Serialize)]
pub struct PlaywrightCrawlRequest {
    pub keyword: String,
    #[serde(rename = "startTime")]
    pub start_time: Option<String>,
    #[serde(rename = "endTime")]
    pub end_time: Option<String>,
    pub page: u32,
    pub cookies: HashMap<String, String>,
}

/// Playwright爬取结果
///
/// Playwright服务器返回的爬取结果
/// 字段使用camelCase以对齐TypeScript端
#[derive(Debug, Clone, serde::Deserialize)]
pub struct PlaywrightCrawlResult {
    pub posts: Vec<WeiboPostRaw>,
    #[serde(rename = "hasMore")]
    pub has_more: bool,
    #[serde(rename = "totalResults")]
    pub total_results: Option<usize>,
    #[serde(rename = "captchaDetected")]
    pub captcha_detected: Option<bool>,
}

/// 原始微博帖子数据
///
/// Playwright返回的未处理帖子数据
#[derive(Debug, Clone, serde::Deserialize)]
pub struct WeiboPostRaw {
    pub id: String,
    pub text: String,
    pub created_at: String,
    pub author_uid: String,
    pub author_screen_name: String,
    pub reposts_count: u64,
    pub comments_count: u64,
    pub attitudes_count: u64,
}

// TimeShardService已在T024实现，直接使用
use super::time_shard_service::TimeShardService;

/// 爬取服务
///
/// 单例服务,同一时刻只允许一个任务处于活跃状态
pub struct CrawlService {
    redis_service: Arc<RedisService>,
    time_shard_service: Arc<TimeShardService>,

    /// 活跃任务的取消令牌
    active_tasks: Arc<Mutex<HashMap<String, CancellationToken>>>,

    /// Tauri应用句柄(用于推送事件)
    app_handle: Option<tauri::AppHandle>,
}

impl CrawlService {
    /// 创建新的爬取服务
    pub fn new(
        redis_service: Arc<RedisService>,
        time_shard_service: Arc<TimeShardService>,
    ) -> Self {
        Self {
            redis_service,
            time_shard_service,
            active_tasks: Arc::new(Mutex::new(HashMap::new())),
            app_handle: None,
        }
    }

    /// 设置Tauri应用句柄
    pub fn with_app_handle(mut self, app_handle: tauri::AppHandle) -> Self {
        self.app_handle = Some(app_handle);
        self
    }

    /// 启动历史回溯
    ///
    /// 从现在回溯到event_start_time
    /// 使用TimeShardService拆分时间范围,逐分片爬取
    pub async fn start_history_crawl(&self, task_id: &str) -> Result<(), String> {
        // 1. 加载任务
        let mut task = self
            .redis_service
            .load_task(task_id)
            .await
            .map_err(|e| format!("加载任务失败: {}", e))?;

        // 2. 验证任务状态
        if task.status != crate::models::CrawlStatus::Created {
            return Err(format!("任务状态无效: {}", task.status.as_str()));
        }

        // 3. 转换状态
        task.transition_to(crate::models::CrawlStatus::HistoryCrawling)
            .map_err(|e| format!("状态转换失败: {}", e))?;
        self.redis_service
            .save_crawl_task(&task)
            .await
            .map_err(|e| format!("保存任务失败: {}", e))?;

        // 4. 检查是否有其他任务正在运行
        if self.has_active_task().await {
            return Err("已有任务正在运行,请先暂停或等待完成".to_string());
        }

        // 5. 创建取消令牌
        let cancel_token = CancellationToken::new();
        self.register_active_task(task_id, cancel_token.clone())
            .await;

        tracing::info!(
            任务ID = %task_id,
            关键字 = %task.keyword,
            事件开始时间 = %task.event_start_time,
            "开始准备历史回溯,将在后台执行时间分片"
        );

        // 6. 立即返回,将耗时操作移到后台任务
        let service_clone = self.clone_for_background();
        let task_id_owned = task_id.to_string();
        let event_start = task.event_start_time;
        let keyword = task.keyword.clone();

        tokio::spawn(async move {
            tracing::info!(
                任务ID = %task_id_owned,
                "后台任务开始: 准备时间范围和分片"
            );

            // 准备时间范围
            let now = Utc::now();
            let end_time = ceil_to_hour(now);
            let start_time = floor_to_hour(event_start);

            tracing::debug!(
                任务ID = %task_id_owned,
                开始时间 = %start_time,
                结束时间 = %end_time,
                时间跨度小时数 = %(end_time - start_time).num_hours(),
                "调用时间分片服务"
            );

            // 拆分时间范围
            let time_shards = match service_clone
                .time_shard_service
                .split_time_range_if_needed(start_time, end_time, &keyword)
                .await
            {
                Ok(shards) => {
                    tracing::info!(
                        任务ID = %task_id_owned,
                        时间分片数量 = %shards.len(),
                        "时间分片完成"
                    );
                    shards
                }
                Err(e) => {
                    let error_msg = format!("时间分片失败: {}", e);
                    tracing::error!(
                        任务ID = %task_id_owned,
                        错误 = %e,
                        "时间分片失败"
                    );
                    service_clone.handle_crawl_error(&task_id_owned, error_msg).await;
                    return;
                }
            };

            // 执行爬取
            if let Err(e) = service_clone
                .execute_backward_crawl(&task_id_owned, time_shards, cancel_token)
                .await
            {
                tracing::error!(任务ID = %task_id_owned, 错误 = %e, "历史回溯失败");
                service_clone.handle_crawl_error(&task_id_owned, e).await;
            }
        });

        tracing::info!(
            任务ID = %task_id,
            "历史回溯命令返回,后台任务已启动"
        );

        Ok(())
    }

    /// 启动增量更新
    ///
    /// 从max_post_time到现在
    pub async fn start_incremental_crawl(&self, task_id: &str) -> Result<(), String> {
        // 1. 加载任务
        let mut task = self
            .redis_service
            .load_task(task_id)
            .await
            .map_err(|e| format!("加载任务失败: {}", e))?;

        // 2. 验证任务状态
        if task.status != crate::models::CrawlStatus::HistoryCompleted {
            return Err(format!("任务状态无效: {}", task.status.as_str()));
        }

        // 3. 验证max_post_time存在
        let max_post_time = task
            .max_post_time
            .ok_or_else(|| "增量更新需要max_post_time".to_string())?;

        // 4. 转换状态
        task.transition_to(crate::models::CrawlStatus::IncrementalCrawling)
            .map_err(|e| format!("状态转换失败: {}", e))?;
        self.redis_service
            .save_crawl_task(&task)
            .await
            .map_err(|e| format!("保存任务失败: {}", e))?;

        // 5. 检查是否有其他任务正在运行
        if self.has_active_task().await {
            return Err("已有任务正在运行,请先暂停或等待完成".to_string());
        }

        // 6. 创建取消令牌
        let cancel_token = CancellationToken::new();
        self.register_active_task(task_id, cancel_token.clone())
            .await;

        // 7. 准备时间范围
        let start_time = ceil_to_hour(max_post_time);
        let end_time = ceil_to_hour(Utc::now());

        tracing::info!(
            任务ID = %task_id,
            开始时间 = %start_time,
            结束时间 = %end_time,
            "启动增量更新"
        );

        // 8. 在后台任务中执行爬取
        let service_clone = self.clone_for_background();
        let task_id_owned = task_id.to_string();

        tokio::spawn(async move {
            if let Err(e) = service_clone
                .execute_forward_crawl(&task_id_owned, start_time, end_time, cancel_token)
                .await
            {
                tracing::error!(任务ID = %task_id_owned, 错误 = %e, "增量更新失败");
                service_clone.handle_crawl_error(&task_id_owned, e).await;
            }
        });

        Ok(())
    }

    /// 从检查点恢复爬取
    pub async fn resume_crawl(&self, task_id: &str) -> Result<(), String> {
        // 1. 加载任务
        let mut task = self
            .redis_service
            .load_task(task_id)
            .await
            .map_err(|e| format!("加载任务失败: {}", e))?;

        // 2. 验证任务状态
        if task.status != crate::models::CrawlStatus::Paused {
            return Err(format!("任务状态无效: {}", task.status.as_str()));
        }

        // 3. 加载检查点
        let checkpoint = self
            .redis_service
            .load_checkpoint(task_id)
            .await
            .map_err(|e| format!("加载检查点失败: {}", e))?
            .ok_or_else(|| "未找到检查点".to_string())?;

        // 4. 根据方向恢复到相应状态
        let target_status = match checkpoint.direction {
            CrawlDirection::Backward => crate::models::CrawlStatus::HistoryCrawling,
            CrawlDirection::Forward => crate::models::CrawlStatus::IncrementalCrawling,
        };

        task.transition_to(target_status)
            .map_err(|e| format!("状态转换失败: {}", e))?;
        self.redis_service
            .save_crawl_task(&task)
            .await
            .map_err(|e| format!("保存任务失败: {}", e))?;

        // 5. 检查是否有其他任务正在运行
        if self.has_active_task().await {
            return Err("已有任务正在运行,请先暂停或等待完成".to_string());
        }

        // 6. 创建取消令牌
        let cancel_token = CancellationToken::new();
        self.register_active_task(task_id, cancel_token.clone())
            .await;

        tracing::info!(
            任务ID = %task_id,
            方向 = ?checkpoint.direction,
            当前页码 = %checkpoint.current_page,
            "从检查点恢复爬取"
        );

        // 7. 在后台任务中恢复爬取
        let service_clone = self.clone_for_background();
        let task_id_owned = task_id.to_string();

        tokio::spawn(async move {
            if let Err(e) = service_clone
                .resume_from_checkpoint(&task_id_owned, checkpoint, cancel_token)
                .await
            {
                tracing::error!(任务ID = %task_id_owned, 错误 = %e, "恢复爬取失败");
                service_clone.handle_crawl_error(&task_id_owned, e).await;
            }
        });

        Ok(())
    }

    /// 取消爬取
    pub async fn cancel_crawl(&self, task_id: &str) -> Result<(), String> {
        // 1. 加载任务
        let mut task = self
            .redis_service
            .load_task(task_id)
            .await
            .map_err(|e| format!("加载任务失败: {}", e))?;

        // 2. 验证任务状态
        if task.status != crate::models::CrawlStatus::HistoryCrawling
            && task.status != crate::models::CrawlStatus::IncrementalCrawling
        {
            return Err(format!("任务状态无效: {}", task.status.as_str()));
        }

        // 3. 发送取消信号
        let mut active_tasks = self.active_tasks.lock().await;
        if let Some(token) = active_tasks.remove(task_id) {
            token.cancel();
            tracing::info!(任务ID = %task_id, "取消信号已发送");
        }

        // 4. 更新状态为暂停
        task.transition_to(crate::models::CrawlStatus::Paused)
            .map_err(|e| format!("状态转换失败: {}", e))?;
        self.redis_service
            .save_crawl_task(&task)
            .await
            .map_err(|e| format!("保存任务失败: {}", e))?;

        Ok(())
    }

    // ==================== 私有方法 ====================

    /// 执行向后爬取(历史回溯)
    async fn execute_backward_crawl(
        &self,
        task_id: &str,
        time_shards: Vec<(DateTime<Utc>, DateTime<Utc>)>,
        cancel_token: CancellationToken,
    ) -> Result<(), String> {
        let start_time = std::time::Instant::now();

        // 按时间倒序处理分片(从现在到过去)
        for (shard_start, shard_end) in time_shards.iter().rev() {
            if cancel_token.is_cancelled() {
                tracing::info!(任务ID = %task_id, "任务已取消");
                return Ok(());
            }

            // 创建检查点
            let mut checkpoint =
                CrawlCheckpoint::new_backward(task_id.to_string(), *shard_start, *shard_end);

            // 爬取当前分片
            self.crawl_time_shard(task_id, &mut checkpoint, &cancel_token)
                .await?;

            // 保存检查点
            self.redis_service
                .save_checkpoint(&checkpoint)
                .await
                .map_err(|e| format!("保存检查点失败: {}", e))?;
        }

        // 所有分片完成,标记任务为HistoryCompleted
        let mut task = self
            .redis_service
            .load_task(task_id)
            .await
            .map_err(|e| format!("加载任务失败: {}", e))?;

        task.transition_to(crate::models::CrawlStatus::HistoryCompleted)
            .map_err(|e| format!("状态转换失败: {}", e))?;
        self.redis_service
            .save_crawl_task(&task)
            .await
            .map_err(|e| format!("保存任务失败: {}", e))?;

        // 移除活跃任务
        self.unregister_active_task(task_id).await;

        // 推送完成事件
        let duration = start_time.elapsed().as_secs();
        self.emit_completed_event(
            task_id,
            CrawlStatus::HistoryCompleted,
            task.crawled_count,
            duration,
        );

        tracing::info!(
            任务ID = %task_id,
            总爬取数 = %task.crawled_count,
            耗时秒数 = %duration,
            "历史回溯完成"
        );

        Ok(())
    }

    /// 执行向前爬取(增量更新)
    async fn execute_forward_crawl(
        &self,
        task_id: &str,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        cancel_token: CancellationToken,
    ) -> Result<(), String> {
        let crawl_start = std::time::Instant::now();

        // 创建检查点
        let mut checkpoint = CrawlCheckpoint::new_forward(task_id.to_string(), start_time);
        checkpoint.shard_end_time = end_time;

        // 爬取
        self.crawl_time_shard(task_id, &mut checkpoint, &cancel_token)
            .await?;

        // 移除活跃任务
        self.unregister_active_task(task_id).await;

        // 推送完成事件
        let task = self
            .redis_service
            .load_task(task_id)
            .await
            .map_err(|e| format!("加载任务失败: {}", e))?;
        let duration = crawl_start.elapsed().as_secs();
        self.emit_completed_event(
            task_id,
            CrawlStatus::IncrementalCrawling,
            task.crawled_count,
            duration,
        );

        tracing::info!(
            任务ID = %task_id,
            总爬取数 = %task.crawled_count,
            耗时秒数 = %duration,
            "增量更新完成"
        );

        Ok(())
    }

    /// 从检查点恢复
    async fn resume_from_checkpoint(
        &self,
        task_id: &str,
        mut checkpoint: CrawlCheckpoint,
        cancel_token: CancellationToken,
    ) -> Result<(), String> {
        // 根据方向决定恢复策略
        match checkpoint.direction {
            CrawlDirection::Backward => {
                // 获取所有待处理的时间分片
                let task = self
                    .redis_service
                    .load_task(task_id)
                    .await
                    .map_err(|e| format!("加载任务失败: {}", e))?;

                let end_time = ceil_to_hour(Utc::now());
                let start_time = floor_to_hour(task.event_start_time);

                let time_shards = self
                    .time_shard_service
                    .split_time_range_if_needed(start_time, end_time, &task.keyword)
                    .await
                    .map_err(|e| format!("时间分片失败: {}", e))?;

                // 过滤掉已完成的分片
                let remaining_shards: Vec<_> = time_shards
                    .into_iter()
                    .filter(|shard| !checkpoint.completed_shards.contains(shard))
                    .collect();

                // 继续爬取剩余分片
                self.execute_backward_crawl(task_id, remaining_shards, cancel_token)
                    .await
            }
            CrawlDirection::Forward => {
                // 继续增量更新
                self.crawl_time_shard(task_id, &mut checkpoint, &cancel_token)
                    .await?;
                self.unregister_active_task(task_id).await;
                Ok(())
            }
        }
    }

    /// 爬取单个时间分片
    async fn crawl_time_shard(
        &self,
        task_id: &str,
        checkpoint: &mut CrawlCheckpoint,
        cancel_token: &CancellationToken,
    ) -> Result<(), String> {
        let mut page = checkpoint.current_page;
        const MAX_PAGE: u32 = 50;

        loop {
            if cancel_token.is_cancelled() {
                return Ok(());
            }

            if page > MAX_PAGE {
                tracing::warn!(
                    任务ID = %task_id,
                    时间范围 = %format!("{} - {}", checkpoint.shard_start_time, checkpoint.shard_end_time),
                    "达到最大页数限制(50页)"
                );
                break;
            }

            // 爬取单页
            match self.crawl_page(task_id, checkpoint, page).await {
                Ok(has_more) => {
                    checkpoint.advance_page();
                    page += 1;

                    if !has_more {
                        tracing::info!(
                            任务ID = %task_id,
                            页码 = %page,
                            "当前时间分片已无更多结果"
                        );
                        break;
                    }
                }
                Err(e) if e.contains("CAPTCHA_DETECTED") => {
                    tracing::warn!(任务ID = %task_id, "检测到验证码,自动暂停");

                    // 推送错误事件
                    self.emit_error_event(
                        task_id,
                        "检测到验证码".to_string(),
                        ErrorCode::CaptchaDetected,
                    );

                    // 自动暂停
                    let mut task = self
                        .redis_service
                        .load_task(task_id)
                        .await
                        .map_err(|e| format!("加载任务失败: {}", e))?;
                    task.transition_to(crate::models::CrawlStatus::Paused)
                        .map_err(|e| format!("状态转换失败: {}", e))?;
                    self.redis_service
                        .save_crawl_task(&task)
                        .await
                        .map_err(|e| format!("保存任务失败: {}", e))?;

                    return Err("CAPTCHA_DETECTED".to_string());
                }
                Err(e) => {
                    return Err(e);
                }
            }

            // 随机延迟1-3秒
            self.random_delay().await;
        }

        Ok(())
    }

    /// 爬取单页
    ///
    /// 调用Playwright脚本,重试3次,返回是否还有更多结果
    async fn crawl_page(
        &self,
        task_id: &str,
        checkpoint: &CrawlCheckpoint,
        page: u32,
    ) -> Result<bool, String> {
        const MAX_RETRIES: u32 = 3;
        const RETRY_DELAYS: [u64; 3] = [1, 2, 4]; // 指数退避

        for attempt in 0..MAX_RETRIES {
            match self.crawl_page_once(task_id, checkpoint, page).await {
                Ok(has_more) => {
                    return Ok(has_more);
                }
                Err(e) if e.contains("CAPTCHA_DETECTED") => {
                    return Err(e); // 验证码错误不重试
                }
                Err(e) if attempt < MAX_RETRIES - 1 => {
                    let delay = RETRY_DELAYS[attempt as usize];
                    tracing::warn!(
                        任务ID = %task_id,
                        页码 = %page,
                        尝试次数 = %(attempt + 1),
                        延迟秒数 = %delay,
                        错误 = %e,
                        "爬取失败,即将重试"
                    );
                    tokio::time::sleep(tokio::time::Duration::from_secs(delay)).await;
                }
                Err(e) => {
                    return Err(format!("爬取失败(已重试{}次): {}", MAX_RETRIES, e));
                }
            }
        }

        unreachable!()
    }

    /// 爬取单页(单次尝试)
    async fn crawl_page_once(
        &self,
        task_id: &str,
        checkpoint: &CrawlCheckpoint,
        page: u32,
    ) -> Result<bool, String> {
        // TODO: T027实现Playwright通信
        // 这里先定义接口,返回模拟数据用于测试

        tracing::debug!(
            任务ID = %task_id,
            页码 = %page,
            时间范围 = %format!("{} - {}", checkpoint.shard_start_time, checkpoint.shard_end_time),
            "开始爬取单页"
        );

        // 模拟Playwright调用
        let result = self.call_playwright(task_id, checkpoint, page).await?;

        // 检查验证码
        if result.captcha_detected.unwrap_or(false) {
            return Err("CAPTCHA_DETECTED".to_string());
        }

        // 处理帖子数据
        let posts = self.process_posts(task_id, result.posts).await?;

        // 保存到Redis
        if !posts.is_empty() {
            self.redis_service
                .save_posts(task_id, &posts)
                .await
                .map_err(|e| format!("保存帖子失败: {}", e))?;

            // 更新任务进度
            let mut task = self
                .redis_service
                .load_task(task_id)
                .await
                .map_err(|e| format!("加载任务失败: {}", e))?;

            for post in &posts {
                task.update_progress(post.created_at, 1);
            }

            self.redis_service
                .save_crawl_task(&task)
                .await
                .map_err(|e| format!("保存任务失败: {}", e))?;

            // 推送进度事件
            let status = match checkpoint.direction {
                CrawlDirection::Backward => CrawlStatus::HistoryCrawling,
                CrawlDirection::Forward => CrawlStatus::IncrementalCrawling,
            };

            self.emit_progress_event(
                task_id,
                status,
                checkpoint.shard_start_time,
                checkpoint.shard_end_time,
                page,
                task.crawled_count,
            );
        }

        Ok(result.has_more)
    }

    /// 调用Playwright爬取
    ///
    /// 通过WebSocket与Playwright服务器通信
    async fn call_playwright(
        &self,
        task_id: &str,
        checkpoint: &CrawlCheckpoint,
        page: u32,
    ) -> Result<PlaywrightCrawlResult, String> {
        use futures_util::{SinkExt, StreamExt};
        use tokio_tungstenite::connect_async;
        use tokio_tungstenite::tungstenite::Message;

        // 获取任务和cookies
        let task = self
            .redis_service
            .load_task(task_id)
            .await
            .map_err(|e| format!("加载任务失败: {}", e))?;

        let cookies = self
            .redis_service
            .load_task_cookies(task_id)
            .await
            .map_err(|e| format!("加载任务cookies失败: {}", e))?;

        // 构建时间参数
        let start_time = Some(format_weibo_time(checkpoint.shard_start_time));
        let end_time = Some(format_weibo_time(checkpoint.shard_end_time));

        // 构建请求payload
        let request = PlaywrightCrawlRequest {
            keyword: task.keyword.clone(),
            start_time,
            end_time,
            page,
            cookies,
        };

        // 构建完整消息
        let message = serde_json::json!({
            "action": "crawl_weibo_search",
            "payload": request
        });

        tracing::debug!(
            任务ID = %task_id,
            页码 = %page,
            关键字 = %task.keyword,
            "发送Playwright请求"
        );

        // 连接到Playwright服务器 (统一使用9223端口)
        let ws_url = "ws://localhost:9223";
        let (ws_stream, _) = connect_async(ws_url)
            .await
            .map_err(|e| format!("连接Playwright服务器失败: {}", e))?;

        let (mut write, mut read) = ws_stream.split();

        // 发送请求
        let message_text =
            serde_json::to_string(&message).map_err(|e| format!("序列化请求失败: {}", e))?;

        write
            .send(Message::Text(message_text))
            .await
            .map_err(|e| format!("发送消息失败: {}", e))?;

        tracing::debug!(任务ID = %task_id, "请求已发送,等待响应");

        // 接收响应
        let response = tokio::time::timeout(tokio::time::Duration::from_secs(30), read.next())
            .await
            .map_err(|_| "等待响应超时".to_string())?
            .ok_or_else(|| "连接已关闭".to_string())?
            .map_err(|e| format!("接收响应失败: {}", e))?;

        // 解析响应
        let response_text = response
            .to_text()
            .map_err(|e| format!("解析响应文本失败: {}", e))?;

        tracing::debug!(
            任务ID = %task_id,
            响应长度 = %response_text.len(),
            "收到响应"
        );

        #[derive(serde::Deserialize)]
        struct PlaywrightResponse {
            success: bool,
            data: Option<PlaywrightCrawlResult>,
            error: Option<String>,
        }

        let response_data: PlaywrightResponse =
            serde_json::from_str(response_text).map_err(|e| format!("解析响应JSON失败: {}", e))?;

        if !response_data.success {
            let error = response_data
                .error
                .unwrap_or_else(|| "未知错误".to_string());
            return Err(format!("Playwright爬取失败: {}", error));
        }

        response_data
            .data
            .ok_or_else(|| "响应中缺少data字段".to_string())
    }

    /// 处理帖子数据
    ///
    /// 将原始数据转换为WeiboPost,去重,验证
    async fn process_posts(
        &self,
        task_id: &str,
        raw_posts: Vec<WeiboPostRaw>,
    ) -> Result<Vec<WeiboPost>, String> {
        let mut posts = Vec::new();

        for raw in raw_posts {
            // 检查是否已存在
            if self
                .redis_service
                .check_post_exists(task_id, &raw.id)
                .await
                .map_err(|e| format!("检查帖子存在性失败: {}", e))?
            {
                continue; // 去重
            }

            // 解析时间
            let created_at = crate::utils::time_utils::parse_weibo_time(&raw.created_at)
                .map_err(|e| format!("解析帖子时间失败: {}", e))?;

            // 构建WeiboPost
            let post = WeiboPost::new(
                raw.id,
                task_id.to_string(),
                raw.text,
                created_at,
                raw.author_uid,
                raw.author_screen_name,
                raw.reposts_count,
                raw.comments_count,
                raw.attitudes_count,
            );

            // 验证
            post.validate()
                .map_err(|e| format!("帖子验证失败: {}", e))?;

            posts.push(post);
        }

        Ok(posts)
    }

    /// 随机延迟1-3秒
    async fn random_delay(&self) {
        let delay_ms = rand::thread_rng().gen_range(1000..=3000);
        tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
    }

    /// 克隆用于后台任务
    fn clone_for_background(&self) -> Self {
        Self {
            redis_service: Arc::clone(&self.redis_service),
            time_shard_service: Arc::clone(&self.time_shard_service),
            active_tasks: Arc::clone(&self.active_tasks),
            app_handle: self.app_handle.clone(),
        }
    }

    /// 检查是否有活跃任务
    async fn has_active_task(&self) -> bool {
        let active_tasks = self.active_tasks.lock().await;
        !active_tasks.is_empty()
    }

    /// 注册活跃任务
    async fn register_active_task(&self, task_id: &str, token: CancellationToken) {
        let mut active_tasks = self.active_tasks.lock().await;
        active_tasks.insert(task_id.to_string(), token);
    }

    /// 注销活跃任务
    async fn unregister_active_task(&self, task_id: &str) {
        let mut active_tasks = self.active_tasks.lock().await;
        active_tasks.remove(task_id);
    }

    /// 处理爬取错误
    async fn handle_crawl_error(&self, task_id: &str, error: String) {
        // 标记任务失败
        if let Ok(mut task) = self.redis_service.load_task(task_id).await {
            task.mark_failed(error.clone());
            let _ = self.redis_service.save_crawl_task(&task).await;
        }

        // 移除活跃任务
        self.unregister_active_task(task_id).await;

        // 推送错误事件
        self.emit_error_event(task_id, error, ErrorCode::NetworkError);
    }

    // ==================== 事件推送 ====================

    /// 推送进度事件
    fn emit_progress_event(
        &self,
        task_id: &str,
        status: CrawlStatus,
        shard_start: DateTime<Utc>,
        shard_end: DateTime<Utc>,
        page: u32,
        crawled_count: u64,
    ) {
        let event = CrawlProgressEvent::new(
            task_id.to_string(),
            status,
            TimeRange::new(shard_start, shard_end),
            page,
            crawled_count,
        );

        if let Some(ref app_handle) = self.app_handle {
            let _ = app_handle.emit("crawl-progress", event);
        }
    }

    /// 推送完成事件
    fn emit_completed_event(
        &self,
        task_id: &str,
        final_status: CrawlStatus,
        total_crawled: u64,
        duration: u64,
    ) {
        let event = crate::models::CrawlCompletedEvent::new(
            task_id.to_string(),
            final_status,
            total_crawled,
            duration,
        );

        if let Some(ref app_handle) = self.app_handle {
            let _ = app_handle.emit("crawl-completed", event);
        }
    }

    /// 推送错误事件
    fn emit_error_event(&self, task_id: &str, error: String, error_code: ErrorCode) {
        let event = CrawlErrorEvent::new(task_id.to_string(), error, error_code);

        if let Some(ref app_handle) = self.app_handle {
            let _ = app_handle.emit("crawl-error", event);
        }
    }
}
