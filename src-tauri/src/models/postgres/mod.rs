/**
 * PostgreSQL数据模型
 *
 * 简化的数据模型，替代复杂的Redis结构
 */

use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use chrono::{DateTime, Utc};
use uuid::Uuid;

pub mod queries;

/// 简化的爬取任务状态（6种状态 -> 5种）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "varchar")]
pub enum CrawlStatus {
    /// 已创建，未开始
    #[serde(rename = "Created")]
    #[sqlx(rename = "Created")]
    Created,
    /// 爬取中（合并HistoryCrawling和IncrementalCrawling）
    #[serde(rename = "Crawling")]
    #[sqlx(rename = "Crawling")]
    Crawling,
    /// 已完成（合并HistoryCompleted）
    #[serde(rename = "Completed")]
    #[sqlx(rename = "Completed")]
    Completed,
    /// 已暂停
    #[serde(rename = "Paused")]
    #[sqlx(rename = "Paused")]
    Paused,
    /// 失败
    #[serde(rename = "Failed")]
    #[sqlx(rename = "Failed")]
    Failed,
}

impl Default for CrawlStatus {
    fn default() -> Self {
        Self::Created
    }
}

impl std::fmt::Display for CrawlStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Created => write!(f, "Created"),
            Self::Crawling => write!(f, "Crawling"),
            Self::Completed => write!(f, "Completed"),
            Self::Paused => write!(f, "Paused"),
            Self::Failed => write!(f, "Failed"),
        }
    }
}

/// 简化的爬取任务模型
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CrawlTask {
    /// 任务ID (UUID)
    pub id: Uuid,
    /// 搜索关键字
    pub keyword: String,
    /// 事件开始时间
    pub event_start_time: DateTime<Utc>,
    /// 任务状态
    pub status: CrawlStatus,
    /// 已爬取的最小帖子时间
    pub min_post_time: Option<DateTime<Utc>>,
    /// 已爬取的最大帖子时间
    pub max_post_time: Option<DateTime<Utc>>,
    /// 已爬取帖子总数
    pub crawled_count: i64,
    /// 任务创建时间
    pub created_at: DateTime<Utc>,
    /// 最后更新时间
    pub updated_at: DateTime<Utc>,
    /// 失败原因
    pub failure_reason: Option<String>,
}

impl CrawlTask {
    /// 创建新任务
    pub fn new(keyword: String, event_start_time: DateTime<Utc>) -> Self {
        Self {
            id: Uuid::new_v4(),
            keyword,
            event_start_time,
            status: CrawlStatus::Created,
            min_post_time: None,
            max_post_time: None,
            crawled_count: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            failure_reason: None,
        }
    }

    /// 标记为爬取中
    pub fn mark_as_crawling(&mut self) {
        self.status = CrawlStatus::Crawling;
        self.updated_at = Utc::now();
        self.failure_reason = None;
    }

    /// 标记为已完成
    pub fn mark_as_completed(&mut self) {
        self.status = CrawlStatus::Completed;
        self.updated_at = Utc::now();
        self.failure_reason = None;
    }

    /// 标记为暂停
    pub fn mark_as_paused(&mut self) {
        self.status = CrawlStatus::Paused;
        self.updated_at = Utc::now();
        self.failure_reason = None;
    }

    /// 标记为失败
    pub fn mark_as_failed(&mut self, reason: String) {
        self.status = CrawlStatus::Failed;
        self.updated_at = Utc::now();
        self.failure_reason = Some(reason);
    }

    /// 更新时间范围和统计
    pub fn update_time_range(&mut self, post_time: DateTime<Utc>) {
        match (&self.min_post_time, &self.max_post_time) {
            (None, None) => {
                self.min_post_time = Some(post_time);
                self.max_post_time = Some(post_time);
            }
            (Some(min), Some(max)) => {
                if post_time < *min {
                    self.min_post_time = Some(post_time);
                }
                if post_time > *max {
                    self.max_post_time = Some(post_time);
                }
            }
            _ => {
                // 这种情况不应该发生，但为了安全起见
                self.min_post_time = Some(post_time);
                self.max_post_time = Some(post_time);
            }
        }
        self.crawled_count += 1;
        self.updated_at = Utc::now();
    }

    /// 检查是否可以开始爬取
    pub fn can_start_crawling(&self) -> bool {
        matches!(self.status, CrawlStatus::Created | CrawlStatus::Paused)
    }

    /// 检查是否正在爬取
    pub fn is_crawling(&self) -> bool {
        matches!(self.status, CrawlStatus::Crawling)
    }

    /// 检查是否已完成
    pub fn is_completed(&self) -> bool {
        matches!(self.status, CrawlStatus::Completed)
    }

    /// 检查是否失败
    pub fn is_failed(&self) -> bool {
        matches!(self.status, CrawlStatus::Failed)
    }
}

/// 任务摘要（用于列表展示）
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CrawlTaskSummary {
    /// 任务ID
    pub id: Uuid,
    /// 搜索关键字
    pub keyword: String,
    /// 任务状态
    pub status: CrawlStatus,
    /// 事件开始时间
    pub event_start_time: DateTime<Utc>,
    /// 已爬取帖子总数
    pub crawled_count: i64,
    /// 任务创建时间
    pub created_at: DateTime<Utc>,
    /// 最后更新时间
    pub updated_at: DateTime<Utc>,
    /// 失败原因
    pub failure_reason: Option<String>,
}

impl From<CrawlTask> for CrawlTaskSummary {
    fn from(task: CrawlTask) -> Self {
        Self {
            id: task.id,
            keyword: task.keyword,
            status: task.status,
            event_start_time: task.event_start_time,
            crawled_count: task.crawled_count,
            created_at: task.created_at,
            updated_at: task.updated_at,
            failure_reason: task.failure_reason,
        }
    }
}

/// 简化的微博帖子模型
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct WeiboPost {
    /// 微博帖子ID
    pub id: String,
    /// 所属任务ID
    pub task_id: Uuid,
    /// 帖子内容
    pub text: String,
    /// 发布时间
    pub created_at: DateTime<Utc>,
    /// 作者UID
    pub author_uid: String,
    /// 作者昵称
    pub author_screen_name: String,
    /// 转发数
    pub reposts_count: i64,
    /// 评论数
    pub comments_count: i64,
    /// 点赞数
    pub attitudes_count: i64,
}

impl WeiboPost {
    /// 创建新帖子
    pub fn new(
        id: String,
        task_id: Uuid,
        text: String,
        created_at: DateTime<Utc>,
        author_uid: String,
        author_screen_name: String,
    ) -> Self {
        Self {
            id,
            task_id,
            text,
            created_at,
            author_uid,
            author_screen_name,
            reposts_count: 0,
            comments_count: 0,
            attitudes_count: 0,
        }
    }

    /// 设置统计数据
    pub fn set_stats(&mut self, reposts: i64, comments: i64, attitudes: i64) {
        self.reposts_count = reposts;
        self.comments_count = comments;
        self.attitudes_count = attitudes;
    }
}

/// 任务统计信息
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TaskStats {
    /// 任务ID
    pub id: Uuid,
    /// 关键字
    pub keyword: String,
    /// 状态
    pub status: CrawlStatus,
    /// 任务记录的爬取数量
    pub crawled_count: i64,
    /// 实际帖子数量
    pub actual_post_count: Option<i64>,
    /// 最早帖子时间
    pub earliest_post_time: Option<DateTime<Utc>>,
    /// 最晚帖子时间
    pub latest_post_time: Option<DateTime<Utc>>,
    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

/// 创建任务请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTaskRequest {
    /// 搜索关键字
    pub keyword: String,
    /// 事件开始时间
    pub event_start_time: DateTime<Utc>,
}

/// 列出任务请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListTasksRequest {
    /// 状态过滤
    pub status: Option<CrawlStatus>,
    /// 排序字段
    pub sort_by: Option<String>,
    /// 排序顺序
    pub sort_order: Option<String>,
    /// 限制数量
    pub limit: Option<i64>,
    /// 偏移量
    pub offset: Option<i64>,
}

/// 列出任务响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListTasksResponse {
    /// 任务列表
    pub tasks: Vec<CrawlTaskSummary>,
    /// 总数
    pub total: i64,
}

/// 任务进度信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskProgress {
    /// 任务基本信息
    pub task: CrawlTask,
    /// 实际帖子数量
    pub actual_post_count: i64,
    /// 进度百分比（基于时间范围覆盖度）
    pub progress_percentage: f64,
}

impl TaskProgress {
    /// 创建进度信息
    pub fn new(task: CrawlTask, actual_post_count: i64) -> Self {
        let progress_percentage = if actual_post_count > 0 {
            // 简化的进度计算：基于已爬取数量
            let base_progress = f64::min(95.0, (actual_post_count as f64 / 100.0) * 100.0);
            base_progress
        } else {
            0.0
        };

        Self {
            task,
            actual_post_count,
            progress_percentage,
        }
    }
}