/**
 * PostgreSQL查询操作
 *
 * 简化的数据库查询，替代复杂的Redis操作
 */

use super::{CrawlTask, CrawlTaskSummary, WeiboPost, TaskProgress};
use super::{CreateTaskRequest, ListTasksRequest, ListTasksResponse};
use super::CrawlStatus;
use sqlx::{PgPool, Row};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use tracing::{info, warn};

impl std::str::FromStr for CrawlStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Created" => Ok(CrawlStatus::Created),
            "Crawling" => Ok(CrawlStatus::Crawling),
            "Completed" => Ok(CrawlStatus::Completed),
            "Paused" => Ok(CrawlStatus::Paused),
            "Failed" => Ok(CrawlStatus::Failed),
            _ => Err(format!("Unknown status: {}", s))
        }
    }
}

/// 任务查询操作
pub struct TaskQueries;

impl TaskQueries {
    /// 创建新任务
    pub async fn create_task(
        pool: &PgPool,
        request: CreateTaskRequest,
    ) -> Result<CrawlTask, sqlx::Error> {
        let task = CrawlTask::new(request.keyword, request.event_start_time);

        let row = sqlx::query!(
            r#"
            INSERT INTO tasks (id, keyword, event_start_time, status, crawled_count, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#,
            task.id,
            task.keyword,
            task.event_start_time,
            task.status as CrawlStatus,
            task.crawled_count,
            task.created_at,
            task.updated_at,
        )
        .fetch_one(pool)
        .await?;

        let created_task = CrawlTask {
            id: row.id,
            keyword: row.keyword,
            event_start_time: row.event_start_time,
            status: row.status.parse().unwrap_or_default(),
            min_post_time: row.min_post_time,
            max_post_time: row.max_post_time,
            crawled_count: row.crawled_count.unwrap_or(0),
            created_at: row.created_at.unwrap_or_else(|| Utc::now()),
            updated_at: row.updated_at.unwrap_or_else(|| Utc::now()),
            failure_reason: row.failure_reason,
        };

        info!("创建新任务成功: {} ({})", created_task.keyword, created_task.id);
        Ok(created_task)
    }

    /// 根据ID获取任务
    pub async fn get_task_by_id(pool: &PgPool, task_id: Uuid) -> Result<Option<CrawlTask>, sqlx::Error> {
        let row = sqlx::query!(
            r#"
            SELECT * FROM tasks WHERE id = $1
            "#,
            task_id
        )
        .fetch_optional(pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(CrawlTask {
                id: row.id,
                keyword: row.keyword,
                event_start_time: row.event_start_time,
                status: row.status.parse().unwrap_or_default(),
                min_post_time: row.min_post_time,
                max_post_time: row.max_post_time,
                crawled_count: row.crawled_count.unwrap_or(0),
                created_at: row.created_at.unwrap_or_else(|| Utc::now()),
                updated_at: row.updated_at.unwrap_or_else(|| Utc::now()),
                failure_reason: row.failure_reason,
            }))
        } else {
            Ok(None)
        }
    }

    /// 列出任务
    pub async fn list_tasks(
        pool: &PgPool,
        request: ListTasksRequest,
    ) -> Result<ListTasksResponse, sqlx::Error> {
        // 构建查询条件
        let mut where_clauses = Vec::new();
        let mut params = Vec::new();

        if let Some(status) = &request.status {
            where_clauses.push("status = $1");
            params.push(status.clone() as CrawlStatus);
        }

        let where_clause = if where_clauses.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", where_clauses.join(" AND "))
        };

        // 排序
        let sort_by = request.sort_by.as_deref().unwrap_or("updated_at");
        let sort_order = request.sort_order.as_deref().unwrap_or("DESC");
        let order_clause = format!("ORDER BY {} {}", sort_by, sort_order);

        // 分页
        let limit = request.limit.unwrap_or(20);
        let offset = request.offset.unwrap_or(0);
        let limit_clause = format!("LIMIT {} OFFSET {}", limit, offset);

        // 查询任务列表
        let list_query = format!(
            r#"
            SELECT id, keyword, event_start_time, status, crawled_count, created_at, updated_at, failure_reason
            FROM tasks
            {}
            {}
            {}
            "#,
            where_clause, order_clause, limit_clause
        );

        let mut query = sqlx::query(&list_query);
        for (_i, param) in params.iter().enumerate() {
            query = query.bind(param);
        }

        let rows = query.fetch_all(pool).await?;

        let tasks: Vec<CrawlTaskSummary> = rows
            .into_iter()
            .map(|row| CrawlTaskSummary {
                id: row.get("id"),
                keyword: row.get("keyword"),
                event_start_time: row.get("event_start_time"),
                status: row.get("status"),
                crawled_count: row.get("crawled_count"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                failure_reason: row.get("failure_reason"),
            })
            .collect();

        // 查询总数
        let count_query = if where_clauses.is_empty() {
            "SELECT COUNT(*) FROM tasks".to_string()
        } else {
            format!("SELECT COUNT(*) FROM tasks {}", where_clause)
        };

        let mut count_query_sqlx = sqlx::query_scalar(&count_query);
        for param in &params {
            count_query_sqlx = count_query_sqlx.bind(param);
        }

        let total: i64 = count_query_sqlx.fetch_one(pool).await?;

        info!("查询任务列表完成: {} 条任务 (总计: {})", tasks.len(), total);
        Ok(ListTasksResponse { tasks, total })
    }

    /// 更新任务状态
    pub async fn update_task_status(
        pool: &PgPool,
        task_id: Uuid,
        status: CrawlStatus,
        failure_reason: Option<String>,
    ) -> Result<Option<CrawlTask>, sqlx::Error> {
        let result = sqlx::query!(
            r#"
            UPDATE tasks
            SET status = $1, failure_reason = $2, updated_at = NOW()
            WHERE id = $3
            RETURNING *
            "#,
            status as CrawlStatus,
            failure_reason,
            task_id
        )
        .fetch_optional(pool)
        .await?;

        if let Some(row) = result {
            Ok(Some(CrawlTask {
                id: row.id,
                keyword: row.keyword,
                event_start_time: row.event_start_time,
                status: row.status.parse().unwrap_or_default(),
                min_post_time: row.min_post_time,
                max_post_time: row.max_post_time,
                crawled_count: row.crawled_count.unwrap_or(0),
                created_at: row.created_at.unwrap_or_else(|| Utc::now()),
                updated_at: row.updated_at.unwrap_or_else(|| Utc::now()),
                failure_reason: row.failure_reason,
            }))
        } else {
            Ok(None)
        }
    }

    /// 更新任务爬取进度
    pub async fn update_task_progress(
        pool: &PgPool,
        task_id: Uuid,
        new_post_time: DateTime<Utc>,
    ) -> Result<Option<CrawlTask>, sqlx::Error> {
        let result = sqlx::query!(
            r#"
            UPDATE tasks
            SET
                crawled_count = crawled_count + 1,
                min_post_time = CASE
                    WHEN min_post_time IS NULL OR $1 < min_post_time THEN $1
                    ELSE min_post_time
                END,
                max_post_time = CASE
                    WHEN max_post_time IS NULL OR $1 > max_post_time THEN $1
                    ELSE max_post_time
                END,
                updated_at = NOW()
            WHERE id = $2
            RETURNING *
            "#,
            new_post_time,
            task_id
        )
        .fetch_optional(pool)
        .await?;

        if let Some(row) = result {
            Ok(Some(CrawlTask {
                id: row.id,
                keyword: row.keyword,
                event_start_time: row.event_start_time,
                status: row.status.parse().unwrap_or_default(),
                min_post_time: row.min_post_time,
                max_post_time: row.max_post_time,
                crawled_count: row.crawled_count.unwrap_or(0),
                created_at: row.created_at.unwrap_or_else(|| Utc::now()),
                updated_at: row.updated_at.unwrap_or_else(|| Utc::now()),
                failure_reason: row.failure_reason,
            }))
        } else {
            Ok(None)
        }
    }

    /// 删除任务
    pub async fn delete_task(pool: &PgPool, task_id: Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query!(
            "DELETE FROM tasks WHERE id = $1",
            task_id
        )
        .execute(pool)
        .await?;

        let deleted = result.rows_affected() > 0;
        if deleted {
            info!("删除任务成功: {}", task_id);
        } else {
            warn!("任务不存在，无法删除: {}", task_id);
        }
        Ok(deleted)
    }

    /// 获取任务进度
    pub async fn get_task_progress(pool: &PgPool, task_id: Uuid) -> Result<Option<TaskProgress>, sqlx::Error> {
        let task = Self::get_task_by_id(pool, task_id).await?;
        if let Some(task) = task {
            let actual_post_count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM posts WHERE task_id = $1"
            )
            .bind(task_id)
            .fetch_one(pool)
            .await?;

            Ok(Some(TaskProgress::new(task, actual_post_count)))
        } else {
            Ok(None)
        }
    }
}

/// 帖子查询操作
pub struct PostQueries;

impl PostQueries {
    /// 插入帖子
    pub async fn insert_post(pool: &PgPool, post: WeiboPost) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            INSERT INTO posts (id, task_id, text, created_at, author_uid, author_screen_name, reposts_count, comments_count, attitudes_count)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            ON CONFLICT (id) DO NOTHING
            "#,
            post.id,
            post.task_id,
            post.text,
            post.created_at,
            post.author_uid,
            post.author_screen_name,
            post.reposts_count,
            post.comments_count,
            post.attitudes_count,
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    /// 批量插入帖子
    pub async fn insert_posts_batch(
        pool: &PgPool,
        posts: Vec<WeiboPost>,
    ) -> Result<(), sqlx::Error> {
        if posts.is_empty() {
            return Ok(());
        }

        let mut query_builder = sqlx::QueryBuilder::new(
            "INSERT INTO posts (id, task_id, text, created_at, author_uid, author_screen_name, reposts_count, comments_count, attitudes_count) "
        );

        query_builder.push_values(posts.iter(), |mut b, post| {
            b.push_bind(&post.id)
                .push_bind(post.task_id)
                .push_bind(&post.text)
                .push_bind(post.created_at)
                .push_bind(&post.author_uid)
                .push_bind(&post.author_screen_name)
                .push_bind(post.reposts_count)
                .push_bind(post.comments_count)
                .push_bind(post.attitudes_count);
        });

        query_builder.push(" ON CONFLICT (id) DO NOTHING");

        let query = query_builder.build();
        query.execute(pool).await?;

        info!("批量插入帖子完成: {} 条", posts.len());
        Ok(())
    }

    /// 增量查询帖子（用于增量爬取）
    pub async fn get_posts_incremental(
        pool: &PgPool,
        task_id: Uuid,
        after_time: DateTime<Utc>,
        limit: Option<i64>,
    ) -> Result<Vec<WeiboPost>, sqlx::Error> {
        let limit = limit.unwrap_or(50);

        let rows = sqlx::query!(
            r#"
            SELECT * FROM posts
            WHERE task_id = $1 AND created_at > $2
            ORDER BY created_at DESC
            LIMIT $3
            "#,
            task_id,
            after_time,
            limit
        )
        .fetch_all(pool)
        .await?;

        let posts: Vec<WeiboPost> = rows
            .into_iter()
            .map(|row| WeiboPost {
                id: row.id,
                task_id: row.task_id,
                text: row.text,
                created_at: row.created_at,
                author_uid: row.author_uid,
                author_screen_name: row.author_screen_name,
                reposts_count: row.reposts_count.unwrap_or(0),
                comments_count: row.comments_count.unwrap_or(0),
                attitudes_count: row.attitudes_count.unwrap_or(0),
            })
            .collect();

        Ok(posts)
    }

    /// 获取任务的所有帖子
    pub async fn get_posts_by_task(
        pool: &PgPool,
        task_id: Uuid,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<WeiboPost>, sqlx::Error> {
        let limit = limit.unwrap_or(100);
        let offset = offset.unwrap_or(0);

        let rows = sqlx::query!(
            r#"
            SELECT * FROM posts
            WHERE task_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
            task_id,
            limit,
            offset
        )
        .fetch_all(pool)
        .await?;

        let posts: Vec<WeiboPost> = rows
            .into_iter()
            .map(|row| WeiboPost {
                id: row.id,
                task_id: row.task_id,
                text: row.text,
                created_at: row.created_at,
                author_uid: row.author_uid,
                author_screen_name: row.author_screen_name,
                reposts_count: row.reposts_count.unwrap_or(0),
                comments_count: row.comments_count.unwrap_or(0),
                attitudes_count: row.attitudes_count.unwrap_or(0),
            })
            .collect();

        Ok(posts)
    }

    /// 获取帖子总数
    pub async fn get_post_count_by_task(pool: &PgPool, task_id: Uuid) -> Result<i64, sqlx::Error> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM posts WHERE task_id = $1"
        )
        .bind(task_id)
        .fetch_one(pool)
        .await?;

        Ok(count)
    }

    /// 获取任务的时间范围
    pub async fn get_task_time_range(
        pool: &PgPool,
        task_id: Uuid,
    ) -> Result<Option<(DateTime<Utc>, DateTime<Utc>)>, sqlx::Error> {
        let row = sqlx::query!(
            r#"
            SELECT MIN(created_at) as min_time, MAX(created_at) as max_time
            FROM posts WHERE task_id = $1
            "#,
            task_id
        )
        .fetch_optional(pool)
        .await?;

        if let Some(row) = row {
            if let (Some(min_time), Some(max_time)) = (row.min_time, row.max_time) {
                Ok(Some((min_time, max_time)))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }
}