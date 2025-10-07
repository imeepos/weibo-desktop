//! 爬取任务管理命令
//!
//! 提供6个Tauri命令:
//! - create_crawl_task: 创建新的爬取任务
//! - start_crawl: 启动爬取(历史回溯或增量更新)
//! - pause_crawl: 暂停正在运行的爬取
//! - get_crawl_progress: 查询任务进度
//! - export_crawl_data: 导出爬取数据(JSON/CSV)
//! - list_crawl_tasks: 列出所有任务

use crate::models::{CrawlTask, CrawlStatus};
use crate::state::AppState;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tauri::State;

// ==================== 错误定义 ====================

/// 命令执行错误
#[derive(Debug, Serialize, Deserialize)]
pub struct CommandError {
    pub error: String,
    pub code: String,
}

impl CommandError {
    fn new(code: &str, message: String) -> Self {
        Self {
            error: message,
            code: code.to_string(),
        }
    }

    fn invalid_keyword() -> Self {
        Self::new("INVALID_KEYWORD", "关键字不能为空".to_string())
    }

    fn invalid_time(msg: &str) -> Self {
        Self::new("INVALID_TIME", msg.to_string())
    }

    fn cookies_not_found(uid: &str) -> Self {
        Self::new(
            "COOKIES_NOT_FOUND",
            format!("未找到UID {} 的Cookies,请先扫码登录", uid),
        )
    }

    fn cookies_expired(days: i64) -> Self {
        Self::new(
            "COOKIES_EXPIRED",
            format!("Cookies可能已过期(验证时间>{}天),请重新登录", days),
        )
    }

    fn task_not_found(task_id: &str) -> Self {
        Self::new(
            "TASK_NOT_FOUND",
            format!("任务 {} 不存在", task_id),
        )
    }

    fn invalid_status(current_status: &str, allowed: &str) -> Self {
        Self::new(
            "INVALID_STATUS",
            format!("任务状态 {} 无法执行此操作,仅支持{}", current_status, allowed),
        )
    }

    fn already_running() -> Self {
        Self::new(
            "ALREADY_RUNNING",
            "已有任务正在运行,请先暂停或等待完成".to_string(),
        )
    }

    fn no_data(task_id: &str) -> Self {
        Self::new(
            "NO_DATA",
            format!("任务 {} 尚无数据可导出", task_id),
        )
    }

    fn invalid_format(format: &str) -> Self {
        Self::new(
            "INVALID_FORMAT",
            format!("不支持的导出格式: {}", format),
        )
    }

    fn storage_error(details: &str) -> Self {
        Self::new("STORAGE_ERROR", details.to_string())
    }

    fn file_system_error(details: &str) -> Self {
        Self::new("FILE_SYSTEM_ERROR", format!("写入文件失败: {}", details))
    }
}

// ==================== 请求/响应结构 ====================

/// 创建任务请求
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCrawlTaskRequest {
    pub keyword: String,
    pub event_start_time: String,
    pub uid: String,
}

/// 创建任务响应
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCrawlTaskResponse {
    pub task_id: String,
    pub created_at: String,
}

/// 启动爬取请求
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartCrawlRequest {
    pub task_id: String,
}

/// 启动爬取响应
#[derive(Debug, Serialize)]
pub struct StartCrawlResponse {
    pub message: String,
    pub direction: String,
}

/// 暂停爬取请求
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PauseCrawlRequest {
    pub task_id: String,
}

/// 检查点信息
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckpointInfo {
    pub shard_start_time: String,
    pub shard_end_time: String,
    pub current_page: u32,
    pub crawled_count: u64,
}

/// 暂停爬取响应
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PauseCrawlResponse {
    pub message: String,
    pub checkpoint: CheckpointInfo,
}

/// 查询进度请求
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetCrawlProgressRequest {
    pub task_id: String,
}

/// 检查点详情
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckpointDetail {
    pub shard_start_time: String,
    pub shard_end_time: String,
    pub current_page: u32,
    pub direction: String,
    pub completed_shards: usize,
}

/// 爬取进度响应
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CrawlProgress {
    pub task_id: String,
    pub keyword: String,
    pub status: String,
    pub event_start_time: String,
    pub min_post_time: Option<String>,
    pub max_post_time: Option<String>,
    pub crawled_count: u64,
    pub created_at: String,
    pub updated_at: String,
    pub failure_reason: Option<String>,
    pub checkpoint: Option<CheckpointDetail>,
    pub estimated_progress: u32,
}

/// 导出数据请求
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportCrawlDataRequest {
    pub task_id: String,
    pub format: String,
    pub time_range: Option<TimeRangeFilter>,
}

/// 时间范围过滤
#[derive(Debug, Deserialize)]
pub struct TimeRangeFilter {
    pub start: String,
    pub end: String,
}

/// 导出数据响应
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportCrawlDataResponse {
    pub file_path: String,
    pub exported_count: usize,
    pub file_size: u64,
    pub exported_at: String,
}

/// 列出任务请求
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListCrawlTasksRequest {
    pub status: Option<String>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
}

/// 任务摘要
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CrawlTaskSummary {
    pub task_id: String,
    pub keyword: String,
    pub status: String,
    pub event_start_time: String,
    pub crawled_count: u64,
    pub created_at: String,
    pub updated_at: String,
    pub failure_reason: Option<String>,
}

/// 列出任务响应
#[derive(Debug, Serialize)]
pub struct ListCrawlTasksResponse {
    pub tasks: Vec<CrawlTaskSummary>,
    pub total: usize,
}

// ==================== Tauri Commands ====================

/// 创建爬取任务
#[tauri::command]
pub async fn create_crawl_task(
    request: CreateCrawlTaskRequest,
    state: State<'_, AppState>,
) -> Result<CreateCrawlTaskResponse, CommandError> {
    // 1. 验证关键字
    let keyword = request.keyword.trim();
    if keyword.is_empty() {
        return Err(CommandError::invalid_keyword());
    }

    // 2. 解析并验证事件开始时间
    let event_start_time = parse_iso8601(&request.event_start_time)
        .map_err(|e| CommandError::invalid_time(&format!("时间格式错误: {}", e)))?;

    if event_start_time > Utc::now() {
        return Err(CommandError::invalid_time("事件开始时间不能是未来时间"));
    }

    // 3. 获取并验证cookies
    let cookies_data = state
        .redis
        .query_cookies(&request.uid)
        .await
        .map_err(|_| CommandError::cookies_not_found(&request.uid))?;

    // 4. 检查cookies年龄
    let age_days = (Utc::now() - cookies_data.validated_at).num_days();
    if age_days > 7 {
        return Err(CommandError::cookies_expired(age_days));
    }

    // 5. 创建任务
    let task = CrawlTask::new(keyword.to_string(), event_start_time);

    // 6. 验证任务数据
    task.validate()
        .map_err(|e| CommandError::storage_error(&format!("任务验证失败: {}", e)))?;

    // 7. 保存到Redis
    state
        .redis
        .save_crawl_task(&task)
        .await
        .map_err(|e| CommandError::storage_error(&format!("保存任务失败: {}", e)))?;

    // 8. 保存cookies到单独的Redis键
    state
        .redis
        .save_task_cookies(&task.id, &cookies_data.cookies)
        .await
        .map_err(|e| CommandError::storage_error(&format!("保存任务cookies失败: {}", e)))?;

    // 9. 返回响应
    Ok(CreateCrawlTaskResponse {
        task_id: task.id.clone(),
        created_at: task.created_at.to_rfc3339(),
    })
}

/// 启动爬取
#[tauri::command]
pub async fn start_crawl(
    request: StartCrawlRequest,
    state: State<'_, AppState>,
) -> Result<StartCrawlResponse, CommandError> {
    // 1. 加载任务
    let task = state
        .redis
        .load_task(&request.task_id)
        .await
        .map_err(|_| CommandError::task_not_found(&request.task_id))?;

    // 2. 根据状态决定操作
    match task.status {
        CrawlStatus::Created => {
            // 启动历史回溯
            // TODO: 需要在AppState中添加CrawlService
            // state.crawl_service.start_history_crawl(&request.task_id).await
            // 目前返回占位响应
            tracing::warn!("CrawlService未集成到AppState,暂时返回占位响应");
            Ok(StartCrawlResponse {
                message: "任务已启动,开始历史回溯".to_string(),
                direction: "Backward".to_string(),
            })
        }
        CrawlStatus::Paused => {
            // 恢复爬取
            // TODO: state.crawl_service.resume_crawl(&request.task_id).await
            tracing::warn!("CrawlService未集成到AppState,暂时返回占位响应");
            Ok(StartCrawlResponse {
                message: "任务已恢复,从检查点继续爬取".to_string(),
                direction: "Backward".to_string(),
            })
        }
        CrawlStatus::HistoryCompleted => {
            // 启动增量更新
            // TODO: state.crawl_service.start_incremental_crawl(&request.task_id).await
            tracing::warn!("CrawlService未集成到AppState,暂时返回占位响应");
            Ok(StartCrawlResponse {
                message: "任务已启动,开始增量更新".to_string(),
                direction: "Forward".to_string(),
            })
        }
        CrawlStatus::HistoryCrawling | CrawlStatus::IncrementalCrawling => {
            // 已在运行
            Err(CommandError::already_running())
        }
        CrawlStatus::Failed => {
            Err(CommandError::invalid_status(
                task.status.as_str(),
                "Created/Paused/HistoryCompleted",
            ))
        }
    }
}

/// 暂停爬取
#[tauri::command]
pub async fn pause_crawl(
    request: PauseCrawlRequest,
    state: State<'_, AppState>,
) -> Result<PauseCrawlResponse, CommandError> {
    // 1. 加载任务
    let task = state
        .redis
        .load_task(&request.task_id)
        .await
        .map_err(|_| CommandError::task_not_found(&request.task_id))?;

    // 2. 验证状态
    match task.status {
        CrawlStatus::HistoryCrawling | CrawlStatus::IncrementalCrawling => {
            // 可以暂停
            // TODO: state.crawl_service.cancel_crawl(&request.task_id).await
            tracing::warn!("CrawlService未集成到AppState,暂时返回占位响应");

            // 加载检查点(如果存在)
            let checkpoint = state
                .redis
                .load_checkpoint(&request.task_id)
                .await
                .map_err(|e| CommandError::storage_error(&format!("加载检查点失败: {}", e)))?;

            let checkpoint_info = if let Some(cp) = checkpoint {
                CheckpointInfo {
                    shard_start_time: cp.shard_start_time.to_rfc3339(),
                    shard_end_time: cp.shard_end_time.to_rfc3339(),
                    current_page: cp.current_page,
                    crawled_count: task.crawled_count,
                }
            } else {
                // 如果没有检查点,返回默认值
                CheckpointInfo {
                    shard_start_time: Utc::now().to_rfc3339(),
                    shard_end_time: Utc::now().to_rfc3339(),
                    current_page: 1,
                    crawled_count: task.crawled_count,
                }
            };

            Ok(PauseCrawlResponse {
                message: "任务已暂停,可通过start_crawl恢复".to_string(),
                checkpoint: checkpoint_info,
            })
        }
        _ => Err(CommandError::invalid_status(
            task.status.as_str(),
            "HistoryCrawling/IncrementalCrawling",
        )),
    }
}

/// 查询进度
#[tauri::command]
pub async fn get_crawl_progress(
    request: GetCrawlProgressRequest,
    state: State<'_, AppState>,
) -> Result<CrawlProgress, CommandError> {
    // 1. 加载任务
    let task = state
        .redis
        .load_task(&request.task_id)
        .await
        .map_err(|_| CommandError::task_not_found(&request.task_id))?;

    // 2. 加载检查点(如果存在)
    let checkpoint = state
        .redis
        .load_checkpoint(&request.task_id)
        .await
        .map_err(|e| CommandError::storage_error(&format!("加载检查点失败: {}", e)))?;

    // 3. 计算预估进度
    let estimated_progress = calculate_progress(&task);

    // 4. 构建检查点详情
    let checkpoint_detail = checkpoint.map(|cp| CheckpointDetail {
        shard_start_time: cp.shard_start_time.to_rfc3339(),
        shard_end_time: cp.shard_end_time.to_rfc3339(),
        current_page: cp.current_page,
        direction: match cp.direction {
            crate::models::CrawlDirection::Backward => "Backward".to_string(),
            crate::models::CrawlDirection::Forward => "Forward".to_string(),
        },
        completed_shards: cp.completed_shards.len(),
    });

    // 5. 返回响应
    Ok(CrawlProgress {
        task_id: task.id.clone(),
        keyword: task.keyword.clone(),
        status: task.status.as_str().to_string(),
        event_start_time: task.event_start_time.to_rfc3339(),
        min_post_time: task.min_post_time.map(|t| t.to_rfc3339()),
        max_post_time: task.max_post_time.map(|t| t.to_rfc3339()),
        crawled_count: task.crawled_count,
        created_at: task.created_at.to_rfc3339(),
        updated_at: task.updated_at.to_rfc3339(),
        failure_reason: task.failure_reason.clone(),
        checkpoint: checkpoint_detail,
        estimated_progress,
    })
}

/// 导出数据
#[tauri::command]
pub async fn export_crawl_data(
    request: ExportCrawlDataRequest,
    state: State<'_, AppState>,
) -> Result<ExportCrawlDataResponse, CommandError> {
    // 1. 验证格式
    if request.format != "json" && request.format != "csv" {
        return Err(CommandError::invalid_format(&request.format));
    }

    // 2. 加载任务
    let task = state
        .redis
        .load_task(&request.task_id)
        .await
        .map_err(|_| CommandError::task_not_found(&request.task_id))?;

    // 3. 检查是否有数据
    if task.crawled_count == 0 {
        return Err(CommandError::no_data(&request.task_id));
    }

    // 4. 读取帖子数据
    let posts = if let Some(time_range) = request.time_range {
        let start = parse_iso8601(&time_range.start)
            .map_err(|e| CommandError::invalid_time(&format!("开始时间格式错误: {}", e)))?;
        let end = parse_iso8601(&time_range.end)
            .map_err(|e| CommandError::invalid_time(&format!("结束时间格式错误: {}", e)))?;

        state
            .redis
            .get_posts_by_time_range(&request.task_id, start, end)
            .await
            .map_err(|e| CommandError::storage_error(&format!("读取帖子数据失败: {}", e)))?
    } else {
        // 读取所有帖子
        let start = task.min_post_time.unwrap_or_else(|| task.event_start_time);
        let end = task.max_post_time.unwrap_or_else(Utc::now);

        state
            .redis
            .get_posts_by_time_range(&request.task_id, start, end)
            .await
            .map_err(|e| CommandError::storage_error(&format!("读取帖子数据失败: {}", e)))?
    };

    // 5. 序列化数据
    let (file_content, extension) = match request.format.as_str() {
        "json" => {
            let export_data = serde_json::json!({
                "task_id": task.id,
                "keyword": task.keyword,
                "exported_at": Utc::now().to_rfc3339(),
                "total_posts": posts.len(),
                "posts": posts,
            });
            (
                serde_json::to_string_pretty(&export_data)
                    .map_err(|e| CommandError::storage_error(&format!("JSON序列化失败: {}", e)))?,
                "json",
            )
        }
        "csv" => {
            let csv_content = serialize_to_csv(&posts)
                .map_err(|e| CommandError::storage_error(&format!("CSV序列化失败: {}", e)))?;
            (csv_content, "csv")
        }
        _ => unreachable!(),
    };

    // 6. 写入文件
    let timestamp = Utc::now().timestamp();
    let filename = format!("weibo_{}_{}.{}", task.id, timestamp, extension);

    // Tauri 2.0: 使用临时目录或当前工作目录
    let download_dir = std::env::current_dir()
        .map_err(|e| CommandError::file_system_error(&format!("无法获取当前目录: {}", e)))?;

    let file_path = download_dir.join(&filename);

    std::fs::write(&file_path, &file_content)
        .map_err(|e| CommandError::file_system_error(&e.to_string()))?;

    let file_size = file_content.len() as u64;

    // 7. 返回响应
    Ok(ExportCrawlDataResponse {
        file_path: file_path.to_string_lossy().to_string(),
        exported_count: posts.len(),
        file_size,
        exported_at: Utc::now().to_rfc3339(),
    })
}

/// 列出任务
#[tauri::command]
pub async fn list_crawl_tasks(
    request: ListCrawlTasksRequest,
    state: State<'_, AppState>,
) -> Result<ListCrawlTasksResponse, CommandError> {
    // 1. 查询所有任务
    let mut tasks = state
        .redis
        .list_all_tasks()
        .await
        .map_err(|e| CommandError::storage_error(&format!("查询任务列表失败: {}", e)))?;

    // 2. 按状态过滤
    if let Some(ref status_filter) = request.status {
        tasks.retain(|task| task.status.as_str() == status_filter);
    }

    // 3. 排序
    let sort_by = request.sort_by.as_deref().unwrap_or("createdAt");
    let sort_order = request.sort_order.as_deref().unwrap_or("desc");

    tasks.sort_by(|a, b| {
        let cmp = match sort_by {
            "createdAt" => a.created_at.cmp(&b.created_at),
            "updatedAt" => a.updated_at.cmp(&b.updated_at),
            "crawledCount" => a.crawled_count.cmp(&b.crawled_count),
            _ => a.created_at.cmp(&b.created_at),
        };

        if sort_order == "desc" {
            cmp.reverse()
        } else {
            cmp
        }
    });

    // 4. 转换为摘要
    let task_summaries: Vec<CrawlTaskSummary> = tasks
        .into_iter()
        .map(|task| CrawlTaskSummary {
            task_id: task.id,
            keyword: task.keyword,
            status: task.status.as_str().to_string(),
            event_start_time: task.event_start_time.to_rfc3339(),
            crawled_count: task.crawled_count,
            created_at: task.created_at.to_rfc3339(),
            updated_at: task.updated_at.to_rfc3339(),
            failure_reason: task.failure_reason,
        })
        .collect();

    let total = task_summaries.len();

    // 5. 返回响应
    Ok(ListCrawlTasksResponse {
        tasks: task_summaries,
        total,
    })
}

// ==================== 辅助函数 ====================

/// 解析ISO 8601时间
fn parse_iso8601(time_str: &str) -> Result<DateTime<Utc>, String> {
    DateTime::parse_from_rfc3339(time_str)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| e.to_string())
}

/// 计算预估进度
fn calculate_progress(task: &CrawlTask) -> u32 {
    match task.status {
        CrawlStatus::Created => 0,
        CrawlStatus::HistoryCrawling => {
            if let Some(min_post_time) = task.min_post_time {
                let total_duration = (Utc::now() - task.event_start_time).num_seconds() as f64;
                let crawled_duration = (Utc::now() - min_post_time).num_seconds() as f64;

                if total_duration > 0.0 {
                    ((crawled_duration / total_duration * 100.0) as u32).min(99)
                } else {
                    0
                }
            } else {
                0
            }
        }
        CrawlStatus::HistoryCompleted | CrawlStatus::IncrementalCrawling => 100,
        CrawlStatus::Paused => {
            // 暂停状态保持当前进度
            if let Some(min_post_time) = task.min_post_time {
                let total_duration = (Utc::now() - task.event_start_time).num_seconds() as f64;
                let crawled_duration = (Utc::now() - min_post_time).num_seconds() as f64;

                if total_duration > 0.0 {
                    ((crawled_duration / total_duration * 100.0) as u32).min(99)
                } else {
                    0
                }
            } else {
                0
            }
        }
        CrawlStatus::Failed => {
            // 失败状态保持失败时的进度
            if let Some(min_post_time) = task.min_post_time {
                let total_duration = (Utc::now() - task.event_start_time).num_seconds() as f64;
                let crawled_duration = (Utc::now() - min_post_time).num_seconds() as f64;

                if total_duration > 0.0 {
                    ((crawled_duration / total_duration * 100.0) as u32).min(99)
                } else {
                    0
                }
            } else {
                0
            }
        }
    }
}

/// 将帖子序列化为CSV
fn serialize_to_csv(posts: &[crate::models::WeiboPost]) -> Result<String, String> {
    use std::fmt::Write;

    let mut csv = String::new();

    // 写入表头
    writeln!(
        &mut csv,
        "post_id,text,created_at,author_uid,author_screen_name,reposts,comments,likes,crawled_at"
    )
    .map_err(|e| e.to_string())?;

    // 写入数据行
    for post in posts {
        // CSV转义: 如果文本包含逗号、引号或换行符,需要用引号包裹并转义内部引号
        let escaped_text = escape_csv_field(&post.text);
        let escaped_author = escape_csv_field(&post.author_screen_name);

        writeln!(
            &mut csv,
            "{},{},{},{},{},{},{},{},{}",
            post.id,
            escaped_text,
            post.created_at.to_rfc3339(),
            post.author_uid,
            escaped_author,
            post.reposts_count,
            post.comments_count,
            post.attitudes_count,
            post.crawled_at.to_rfc3339()
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(csv)
}

/// 转义CSV字段
fn escape_csv_field(field: &str) -> String {
    if field.contains(',') || field.contains('"') || field.contains('\n') {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}
