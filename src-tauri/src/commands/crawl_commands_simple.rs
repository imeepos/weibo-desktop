/**
 * 简化的爬取命令模块
 *
 * 替代复杂的Redis时间分片架构，使用PostgreSQL的简化实现
 */

use crate::models::postgres::{
    CreateTaskRequest, ListTasksRequest, CrawlTask, TaskProgress, ListTasksResponse
};
use crate::services::SimpleCrawlService;
use crate::services::weibo_api::WeiboApiClient;
use crate::database::{init_database, get_database};
use std::sync::Arc;
use tauri::{command, AppHandle};
use tracing::{info, error};

/// 简化的创建爬取任务命令
#[command]
pub async fn create_simple_crawl_task(
    app: AppHandle,
    keyword: String,
    event_start_time: String, // ISO 8601格式
) -> Result<String, String> {
    info!("创建简化爬取任务请求: 关键字={}, 开始时间={}", keyword, event_start_time);

    // 解析时间
    let start_time = event_start_time.parse()
        .map_err(|e| format!("时间格式错误: {}", e))?;

    // 创建请求
    let request = CreateTaskRequest {
        keyword,
        event_start_time: start_time,
    };

    // 获取或创建爬取服务
    let crawl_service = get_simple_crawl_service(&app)?;

    // 创建任务
    let task = crawl_service.create_task(request).await
        .map_err(|e| format!("创建任务失败: {}", e))?;

    info!("简化爬取任务创建成功: {} ({})", task.keyword, task.id);
    Ok(task.id.to_string())
}

/// 简化的列出爬取任务命令
#[command]
pub async fn list_simple_crawl_tasks(
    app: AppHandle,
    request: ListTasksRequest,
) -> Result<ListTasksResponse, String> {
    info!("列出简化爬取任务请求: {:?}", request);

    // 获取爬取服务
    let crawl_service = get_simple_crawl_service(&app)?;

    // 获取任务列表（包含total）
    let response = crawl_service.list_tasks_with_total(request).await
        .map_err(|e| format!("获取任务列表失败: {}", e))?;

    info!("返回 {} 个简化任务 (总计: {})", response.tasks.len(), response.total);
    Ok(response)
}

/// 简化的获取任务进度命令
#[command]
pub async fn get_simple_crawl_progress(
    app: AppHandle,
    task_id: String,
) -> Result<Option<TaskProgress>, String> {
    info!("获取简化任务进度: {}", task_id);

    // 获取爬取服务
    let crawl_service = get_simple_crawl_service(&app)?;

    // 获取任务进度
    let progress = crawl_service.get_task_progress(&task_id).await
        .map_err(|e| format!("获取任务进度失败: {}", e))?;

    if let Some(ref p) = progress {
        info!("任务进度: 状态={}, 爬取数量={}, 进度={:.1}%",
              p.task.status, p.actual_post_count, p.progress_percentage);
    } else {
        info!("任务不存在: {}", task_id);
    }

    Ok(progress)
}

/// 简化的开始爬取命令
#[command]
pub async fn start_simple_crawl(
    app: AppHandle,
    task_id: String,
) -> Result<(), String> {
    info!("开始简化爬取: {}", task_id);

    // 获取爬取服务
    let crawl_service = get_simple_crawl_service(&app)?;

    // 开始爬取
    crawl_service.start_crawl(&task_id).await
        .map_err(|e| format!("开始爬取失败: {}", e))?;

    info!("简化爬取已启动: {}", task_id);
    Ok(())
}

/// 简化的暂停爬取命令
#[command]
pub async fn pause_simple_crawl(
    app: AppHandle,
    task_id: String,
) -> Result<(), String> {
    info!("暂停简化爬取: {}", task_id);

    // 获取爬取服务
    let crawl_service = get_simple_crawl_service(&app)?;

    // 暂停爬取
    crawl_service.pause_task(&task_id).await
        .map_err(|e| format!("暂停爬取失败: {}", e))?;

    info!("简化爬取已暂停: {}", task_id);
    Ok(())
}

/// 简化的删除任务命令
#[command]
pub async fn delete_simple_crawl_task(
    app: AppHandle,
    task_id: String,
) -> Result<(), String> {
    info!("删除简化任务: {}", task_id);

    // 获取爬取服务
    let crawl_service = get_simple_crawl_service(&app)?;

    // 删除任务
    crawl_service.delete_task(&task_id).await
        .map_err(|e| format!("删除任务失败: {}", e))?;

    info!("简化任务已删除: {}", task_id);
    Ok(())
}

/// 初始化简化爬取系统
#[command]
pub async fn init_simple_crawl_system(_app: AppHandle) -> Result<(), String> {
    info!("初始化简化爬取系统");

    // 初始化数据库
    if let Err(e) = init_database().await {
        error!("数据库初始化失败: {}", e);
        return Err(format!("数据库初始化失败: {}", e));
    }

    // 测试数据库连接
    if let Err(e) = get_database().health_check().await {
        error!("数据库健康检查失败: {}", e);
        return Err(format!("数据库连接失败: {}", e));
    }

    info!("简化爬取系统初始化完成");
    Ok(())
}

/// 获取简化爬取服务实例
fn get_simple_crawl_service(app: &AppHandle) -> Result<SimpleCrawlService, String> {
    // 创建新的服务实例
    let weibo_client = Arc::new(WeiboApiClient::new(
        "/home/ubuntu/worktrees/desktop/playwright/dist/validate-cookies.js".to_string()
    ));

    let crawl_service = SimpleCrawlService::new(weibo_client)
        .with_app_handle(app.clone());

    info!("创建新的简化爬取服务实例");
    Ok(crawl_service)
}