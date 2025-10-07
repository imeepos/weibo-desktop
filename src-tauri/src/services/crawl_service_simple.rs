//! 简化的爬取服务
//!
//! 核心职责:
//! - 简化的任务状态管理
//! - 直接的增量爬取逻辑
//! - 实时进度事件推送
//! - PostgreSQL数据存储

use crate::models::postgres::{
    CrawlTask, CrawlStatus, WeiboPost, CreateTaskRequest, ListTasksRequest, TaskProgress, ListTasksResponse
};
use crate::models::postgres::queries::{TaskQueries, PostQueries};
use crate::services::weibo_api::WeiboApiClient;
use crate::database::get_db_pool;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::Emitter;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use tracing::{info, error, warn, debug};

/// 简化的爬取事件
#[derive(Debug, Clone, serde::Serialize)]
pub struct SimpleCrawlEvent {
    pub task_id: String,
    pub event_type: CrawlEventType,
    pub timestamp: DateTime<Utc>,
    pub data: Option<serde_json::Value>,
}

/// 爬取事件类型
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type", content = "data")]
pub enum CrawlEventType {
    /// 开始爬取
    Started { keyword: String },
    /// 进度更新
    Progress {
        current_page: u32,
        crawled_count: i64,
        latest_post_time: Option<DateTime<Utc>>,
    },
    /// 帖子保存
    PostsSaved { count: usize },
    /// 爬取完成
    Completed {
        total_posts: i64,
        duration_seconds: u64,
    },
    /// 发生错误
    Error {
        error: String,
        error_code: String,
    },
}

/// Playwright爬取请求
#[derive(Debug, Clone, serde::Serialize)]
pub struct SimpleCrawlRequest {
    pub keyword: String,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub page: u32,
    pub cookies: HashMap<String, String>,
}

/// 简化的爬取服务
pub struct SimpleCrawlService {
    /// Weibo API客户端
    weibo_client: Arc<WeiboApiClient>,

    /// 活跃任务的取消令牌
    active_tasks: Arc<Mutex<HashMap<String, CancellationToken>>>,

    /// Tauri应用句柄
    app_handle: Option<tauri::AppHandle>,
}

impl SimpleCrawlService {
    /// 创建新的简化爬取服务
    pub fn new(weibo_client: Arc<WeiboApiClient>) -> Self {
        Self {
            weibo_client,
            active_tasks: Arc::new(Mutex::new(HashMap::new())),
            app_handle: None,
        }
    }

    /// 设置Tauri应用句柄
    pub fn with_app_handle(mut self, app_handle: tauri::AppHandle) -> Self {
        self.app_handle = Some(app_handle);
        self
    }

    /// 创建新任务
    pub async fn create_task(&self, request: CreateTaskRequest) -> Result<CrawlTask, String> {
        info!("创建新任务: {}", request.keyword);

        let task = TaskQueries::create_task(get_db_pool(), request)
            .await
            .map_err(|e| format!("创建任务失败: {}", e))?;

        info!("任务创建成功: {} ({})", task.keyword, task.id);
        Ok(task)
    }

    /// 获取任务详情
    pub async fn get_task(&self, task_id: &str) -> Result<Option<CrawlTask>, String> {
        let task_id = task_id.parse()
            .map_err(|e| format!("无效的任务ID: {}", e))?;

        TaskQueries::get_task_by_id(get_db_pool(), task_id)
            .await
            .map_err(|e| format!("查询任务失败: {}", e))
    }

    /// 列出任务
    pub async fn list_tasks(&self, request: ListTasksRequest) -> Result<Vec<CrawlTask>, String> {
        let response = TaskQueries::list_tasks(get_db_pool(), request)
            .await
            .map_err(|e| format!("查询任务列表失败: {}", e))?;

        // 将 CrawlTaskSummary 转换为 CrawlTask
        let mut tasks = Vec::new();
        for summary in response.tasks {
            if let Some(task) = TaskQueries::get_task_by_id(get_db_pool(), summary.id).await
                .map_err(|e| format!("获取任务详情失败: {}", e))? {
                tasks.push(task);
            }
        }

        Ok(tasks)
    }

    /// 列出任务（包含总数）
    pub async fn list_tasks_with_total(&self, request: ListTasksRequest) -> Result<ListTasksResponse, String> {
        let response = TaskQueries::list_tasks(get_db_pool(), request)
            .await
            .map_err(|e| format!("查询任务列表失败: {}", e))?;

        Ok(response)
    }

    /// 获取任务进度
    pub async fn get_task_progress(&self, task_id: &str) -> Result<Option<TaskProgress>, String> {
        let task_id = task_id.parse()
            .map_err(|e| format!("无效的任务ID: {}", e))?;

        TaskQueries::get_task_progress(get_db_pool(), task_id)
            .await
            .map_err(|e| format!("查询任务进度失败: {}", e))
    }

    /// 开始爬取任务
    pub async fn start_crawl(&self, task_id: &str) -> Result<(), String> {
        info!("开始爬取任务: {}", task_id);

        // 1. 加载任务
        let task_id_uuid = task_id.parse()
            .map_err(|e| format!("无效的任务ID: {}", e))?;

        let mut task = TaskQueries::get_task_by_id(get_db_pool(), task_id_uuid)
            .await
            .map_err(|e| format!("加载任务失败: {}", e))?
            .ok_or_else(|| format!("任务不存在: {}", task_id))?;

        // 2. 验证任务状态
        if !task.can_start_crawling() {
            return Err(format!("任务状态不允许开始爬取: {:?}", task.status));
        }

        // 3. 检查是否有其他任务正在运行
        if self.has_active_task().await {
            return Err("已有任务正在运行，请先暂停或等待完成".to_string());
        }

        // 4. 更新任务状态
        task.mark_as_crawling();
        TaskQueries::update_task_status(get_db_pool(), task_id_uuid, CrawlStatus::Crawling, None)
            .await
            .map_err(|e| format!("更新任务状态失败: {}", e))?;

        // 5. 创建取消令牌
        let cancel_token = CancellationToken::new();
        self.register_active_task(task_id, cancel_token.clone()).await;

        // 6. 发送开始事件
        self.emit_event(&SimpleCrawlEvent {
            task_id: task_id.to_string(),
            event_type: CrawlEventType::Started {
                keyword: task.keyword.clone()
            },
            timestamp: Utc::now(),
            data: None,
        }).await;

        // 7. 启动后台爬取任务
        let service_clone = self.clone_for_background();
        let task_id_owned = task_id.to_string();
        let keyword = task.keyword.clone();

        tokio::spawn(async move {
            service_clone.execute_simple_crawl(&task_id_owned, &keyword, cancel_token).await;
        });

        Ok(())
    }

    /// 暂停任务
    pub async fn pause_task(&self, task_id: &str) -> Result<(), String> {
        info!("暂停任务: {}", task_id);

        let task_id_uuid = task_id.parse()
            .map_err(|e| format!("无效的任务ID: {}", e))?;

        // 更新任务状态
        TaskQueries::update_task_status(get_db_pool(), task_id_uuid, CrawlStatus::Paused, None)
            .await
            .map_err(|e| format!("更新任务状态失败: {}", e))?;

        // 取消任务
        self.cancel_task(task_id).await;

        info!("任务已暂停: {}", task_id);
        Ok(())
    }

    /// 删除任务
    pub async fn delete_task(&self, task_id: &str) -> Result<(), String> {
        info!("删除任务: {}", task_id);

        let task_id_uuid = task_id.parse()
            .map_err(|e| format!("无效的任务ID: {}", e))?;

        // 先取消任务
        self.cancel_task(task_id).await;

        // 删除任务
        let deleted = TaskQueries::delete_task(get_db_pool(), task_id_uuid)
            .await
            .map_err(|e| format!("删除任务失败: {}", e))?;

        if deleted {
            info!("任务删除成功: {}", task_id);
            Ok(())
        } else {
            Err(format!("任务不存在: {}", task_id))
        }
    }

    /// 执行简化的爬取逻辑
    async fn execute_simple_crawl(
        &self,
        task_id: &str,
        keyword: &str,
        cancel_token: CancellationToken,
    ) {
        info!("开始执行简化爬取: 任务={}, 关键字={}", task_id, keyword);

        let start_time = Utc::now();
        let mut current_page = 1;
        let mut total_crawled = 0;
        let mut last_post_time: Option<DateTime<Utc>> = None;

        // 获取Cookies
        let cookies = match self.weibo_client.get_cookies().await {
            Ok(cookies) => cookies,
            Err(e) => {
                error!("获取Cookies失败: {}", e);
                self.handle_crawl_error(task_id, "获取Cookies失败", "COOKIES_ERROR").await;
                return;
            }
        };

        loop {
            // 检查取消请求
            if cancel_token.is_cancelled() {
                info!("任务被取消: {}", task_id);
                break;
            }

            // 构建爬取请求
            let crawl_request = SimpleCrawlRequest {
                keyword: keyword.to_string(),
                start_time: None, // 爬取最新数据
                end_time: last_post_time.map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string()),
                page: current_page,
                cookies: cookies.clone(),
            };

            debug!("发送爬取请求: 页码={}, 关键字={}", current_page, keyword);

            // 调用Weibo API爬取
            match self.weibo_client.crawl_posts(&crawl_request).await {
                Ok(posts) => {
                    if posts.is_empty() {
                        info!("没有更多数据，爬取完成: {}", task_id);
                        break;
                    }

                    // 转换并保存帖子
                    let task_id_uuid = task_id.parse().expect("Invalid task ID");
                    let weibo_posts: Vec<WeiboPost> = posts.into_iter().map(|raw_post| {
                        let post_time = Utc::now(); // 简化处理，使用当前时间
                        if last_post_time.is_none() || post_time < last_post_time.unwrap() {
                            last_post_time = Some(post_time);
                        }

                        WeiboPost::new(
                            raw_post.id,
                            task_id_uuid,
                            raw_post.text,
                            post_time,
                            raw_post.author_uid,
                            raw_post.author_screen_name,
                        )
                    }).collect();

                    // 批量保存帖子
                    if let Err(e) = PostQueries::insert_posts_batch(get_db_pool(), weibo_posts.clone()).await {
                        error!("保存帖子失败: {}", e);
                        self.handle_crawl_error(task_id, "保存数据失败", "STORAGE_ERROR").await;
                        return;
                    }

                    // 更新任务进度
                    if let Some(post_time) = last_post_time {
                        if let Err(e) = TaskQueries::update_task_progress(get_db_pool(), task_id_uuid, post_time).await {
                            error!("更新任务进度失败: {}", e);
                        }
                    }

                    total_crawled += weibo_posts.len() as i64;

                    // 发送进度事件
                    self.emit_event(&SimpleCrawlEvent {
                        task_id: task_id.to_string(),
                        event_type: CrawlEventType::Progress {
                            current_page,
                            crawled_count: total_crawled,
                            latest_post_time: last_post_time,
                        },
                        timestamp: Utc::now(),
                        data: None,
                    }).await;

                    self.emit_event(&SimpleCrawlEvent {
                        task_id: task_id.to_string(),
                        event_type: CrawlEventType::PostsSaved {
                            count: weibo_posts.len()
                        },
                        timestamp: Utc::now(),
                        data: None,
                    }).await;

                    info!("页面 {} 爬取完成，新增 {} 条帖子", current_page, weibo_posts.len());
                    current_page += 1;

                    // 随机延迟，避免被反爬
                    let delay_ms = rand::random::<u64>() % 3000 + 2000; // 2000-5000ms
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                }
                Err(e) => {
                    error!("爬取失败: {}", e);
                    self.handle_crawl_error(task_id, &format!("爬取失败: {}", e), "CRAWL_ERROR").await;
                    return;
                }
            }

            // 限制最大页数，防止无限爬取
            if current_page > 100 {
                warn!("达到最大页数限制，停止爬取: {}", task_id);
                break;
            }
        }

        // 标记任务完成
        let task_id_uuid = task_id.parse().expect("Invalid task ID");
        if let Err(e) = TaskQueries::update_task_status(get_db_pool(), task_id_uuid, CrawlStatus::Completed, None).await {
            error!("更新任务状态为完成失败: {}", e);
        }

        let duration = (Utc::now() - start_time).num_seconds() as u64;

        // 发送完成事件
        self.emit_event(&SimpleCrawlEvent {
            task_id: task_id.to_string(),
            event_type: CrawlEventType::Completed {
                total_posts: total_crawled,
                duration_seconds: duration,
            },
            timestamp: Utc::now(),
            data: None,
        }).await;

        // 清理活跃任务
        self.unregister_active_task(task_id).await;

        info!("爬取任务完成: {}, 总计 {} 条帖子，耗时 {} 秒", task_id, total_crawled, duration);
    }

    /// 检查是否有活跃任务
    async fn has_active_task(&self) -> bool {
        let active_tasks = self.active_tasks.lock().await;
        !active_tasks.is_empty()
    }

    /// 注册活跃任务
    async fn register_active_task(&self, task_id: &str, cancel_token: CancellationToken) {
        let mut active_tasks = self.active_tasks.lock().await;
        active_tasks.insert(task_id.to_string(), cancel_token);
    }

    /// 取消注册活跃任务
    async fn unregister_active_task(&self, task_id: &str) {
        let mut active_tasks = self.active_tasks.lock().await;
        active_tasks.remove(task_id);
    }

    /// 取消任务
    async fn cancel_task(&self, task_id: &str) {
        let mut active_tasks = self.active_tasks.lock().await;
        if let Some(cancel_token) = active_tasks.remove(task_id) {
            cancel_token.cancel();
            info!("任务已取消: {}", task_id);
        }
    }

    /// 处理爬取错误
    async fn handle_crawl_error(&self, task_id: &str, error_msg: &str, error_code: &str) {
        error!("爬取错误: {} - {}", task_id, error_msg);

        let task_id_uuid = task_id.parse().expect("Invalid task ID");
        if let Err(e) = TaskQueries::update_task_status(
            get_db_pool(),
            task_id_uuid,
            CrawlStatus::Failed,
            Some(error_msg.to_string())
        ).await {
            error!("更新任务状态为失败失败: {}", e);
        }

        // 发送错误事件
        self.emit_event(&SimpleCrawlEvent {
            task_id: task_id.to_string(),
            event_type: CrawlEventType::Error {
                error: error_msg.to_string(),
                error_code: error_code.to_string(),
            },
            timestamp: Utc::now(),
            data: None,
        }).await;

        // 清理活跃任务
        self.unregister_active_task(task_id).await;
    }

    /// 发送事件到前端
    async fn emit_event(&self, event: &SimpleCrawlEvent) {
        if let Some(app_handle) = &self.app_handle {
            if let Err(e) = app_handle.emit("crawl-event", event) {
                error!("发送事件失败: {}", e);
            }
        }
    }

    /// 克隆服务用于后台任务
    fn clone_for_background(&self) -> Self {
        Self {
            weibo_client: Arc::clone(&self.weibo_client),
            active_tasks: Arc::clone(&self.active_tasks),
            app_handle: self.app_handle.clone(),
        }
    }
}

// 为了兼容，添加Playwright相关的类型定义
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