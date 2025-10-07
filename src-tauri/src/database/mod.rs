/**
 * PostgreSQL数据库模块
 *
 * 简化的数据库架构，替代复杂的Redis时间分片系统
 */

use std::env;
use sqlx::{Pool, Postgres, postgres::PgPoolOptions};
use tracing::{info, warn};

/// 数据库连接池
pub type DbPool = Pool<Postgres>;

/// 数据库配置
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "postgresql://postgres:password@localhost:5432/weibo_crawl".to_string(),
            max_connections: 10,
            min_connections: 2,
        }
    }
}

impl DatabaseConfig {
    /// 从环境变量加载配置
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        // 尝试从 .env.db 文件加载
        if let Err(e) = dotenvy::from_filename(".env.db") {
            warn!("无法加载 .env.db 文件: {}, 使用默认配置", e);
        }

        let url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| {
                // 如果没有DATABASE_URL，则尝试组合分离的配置项
                let host = env::var("DB_HOST").unwrap_or_else(|_| "localhost".to_string());
                let port = env::var("DB_PORT").unwrap_or_else(|_| "5432".to_string());
                let name = env::var("DB_NAME").unwrap_or_else(|_| "weibo_crawl".to_string());
                let user = env::var("DB_USER").unwrap_or_else(|_| "postgres".to_string());
                let password = env::var("DB_PASSWORD").unwrap_or_else(|_| "password".to_string());

                format!("postgresql://{}:{}@{}:{}/{}", user, password, host, port, name)
            });

        let max_connections = env::var("DB_MAX_CONNECTIONS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(10);

        let min_connections = env::var("DB_MIN_CONNECTIONS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(2);

        Ok(Self {
            url,
            max_connections,
            min_connections,
        })
    }
}

/// 数据库管理器
#[derive(Debug, Clone)]
pub struct DatabaseManager {
    pool: DbPool,
}

impl DatabaseManager {
    /// 创建新的数据库管理器实例
    pub async fn new(config: DatabaseConfig) -> Result<Self, sqlx::Error> {
        info!("正在连接PostgreSQL数据库: {}",
              config.url.split('@').last().unwrap_or(&config.url));

        let pool = PgPoolOptions::new()
            .max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .connect(&config.url)
            .await?;

        info!("PostgreSQL数据库连接池创建成功，最大连接数: {}", config.max_connections);

        Ok(Self { pool })
    }

    /// 从环境变量创建数据库管理器
    pub async fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        let config = DatabaseConfig::from_env()?;
        Self::new(config).await.map_err(Into::into)
    }

    /// 获取数据库连接池
    pub fn pool(&self) -> &DbPool {
        &self.pool
    }

    /// 运行数据库迁移
    pub async fn migrate(&self) -> Result<(), sqlx::migrate::MigrateError> {
        info!("开始运行数据库迁移...");

        // SQLx 支持从 migrations 目录自动运行迁移
        // 但这里我们手动执行基本的迁移SQL
        self.create_tables_if_not_exists().await?;

        info!("数据库迁移完成");
        Ok(())
    }

    /// 创建表结构（如果不存在）
    async fn create_tables_if_not_exists(&self) -> Result<(), sqlx::Error> {
        // 启用UUID扩展
        sqlx::query("CREATE EXTENSION IF NOT EXISTS \"uuid-ossp\"")
            .execute(self.pool())
            .await?;

        // 创建tasks表
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS tasks (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                keyword VARCHAR(255) NOT NULL,
                event_start_time TIMESTAMP WITH TIME ZONE NOT NULL,
                status VARCHAR(50) NOT NULL DEFAULT 'Created' CHECK (
                    status IN ('Created', 'Crawling', 'Completed', 'Paused', 'Failed')
                ),
                min_post_time TIMESTAMP WITH TIME ZONE,
                max_post_time TIMESTAMP WITH TIME ZONE,
                crawled_count BIGINT DEFAULT 0,
                created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
                updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
                failure_reason TEXT,
                CONSTRAINT valid_event_time CHECK (event_start_time <= NOW()),
                CONSTRAINT valid_time_range CHECK (
                    min_post_time IS NULL OR
                    max_post_time IS NULL OR
                    min_post_time <= max_post_time
                ),
                CONSTRAINT valid_failure_reason CHECK (
                    status != 'Failed' OR failure_reason IS NOT NULL
                )
            )
        "#)
        .execute(self.pool())
        .await?;

        // 创建posts表
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS posts (
                id VARCHAR(255) PRIMARY KEY,
                task_id UUID NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
                text TEXT NOT NULL,
                created_at TIMESTAMP WITH TIME ZONE NOT NULL,
                author_uid VARCHAR(255) NOT NULL,
                author_screen_name VARCHAR(255) NOT NULL,
                reposts_count BIGINT DEFAULT 0,
                comments_count BIGINT DEFAULT 0,
                attitudes_count BIGINT DEFAULT 0,
                CONSTRAINT valid_counts CHECK (
                    reposts_count >= 0 AND
                    comments_count >= 0 AND
                    attitudes_count >= 0
                ),
                CONSTRAINT valid_post_time CHECK (created_at <= NOW())
            )
        "#)
        .execute(self.pool())
        .await?;

        // 创建索引
        let indexes = vec![
            "CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status)",
            "CREATE INDEX IF NOT EXISTS idx_tasks_keyword ON tasks(keyword)",
            "CREATE INDEX IF NOT EXISTS idx_tasks_updated_at ON tasks(updated_at DESC)",
            "CREATE INDEX IF NOT EXISTS idx_posts_task_id ON posts(task_id)",
            "CREATE INDEX IF NOT EXISTS idx_posts_created_at ON posts(created_at DESC)",
            "CREATE INDEX IF NOT EXISTS idx_posts_task_created ON posts(task_id, created_at DESC)",
        ];

        for index_sql in indexes {
            sqlx::query(index_sql).execute(self.pool()).await?;
        }

        // 创建触发器函数
        sqlx::query(r#"
            CREATE OR REPLACE FUNCTION update_updated_at_column()
            RETURNS TRIGGER AS $$
            BEGIN
                NEW.updated_at = NOW();
                RETURN NEW;
            END;
            $$ language 'plpgsql'
        "#)
        .execute(self.pool())
        .await?;

        // 创建触发器
        sqlx::query(r#"
            DROP TRIGGER IF EXISTS update_tasks_updated_at ON tasks;
            CREATE TRIGGER update_tasks_updated_at
                BEFORE UPDATE ON tasks
                FOR EACH ROW
                EXECUTE FUNCTION update_updated_at_column()
        "#)
        .execute(self.pool())
        .await?;

        // 创建统计视图
        sqlx::query(r#"
            CREATE OR REPLACE VIEW task_stats AS
            SELECT
                t.id,
                t.keyword,
                t.status,
                t.crawled_count,
                COUNT(p.id) as actual_post_count,
                MIN(p.created_at) as earliest_post_time,
                MAX(p.created_at) as latest_post_time,
                t.updated_at
            FROM tasks t
            LEFT JOIN posts p ON t.id = p.task_id
            GROUP BY t.id, t.keyword, t.status, t.crawled_count, t.updated_at
        "#)
        .execute(self.pool())
        .await?;

        info!("数据库表结构创建完成");
        Ok(())
    }

    /// 测试数据库连接
    pub async fn health_check(&self) -> Result<(), sqlx::Error> {
        let result: i64 = sqlx::query_scalar("SELECT 1")
            .fetch_one(self.pool())
            .await?;

        if result == 1 {
            info!("数据库连接健康检查通过");
            Ok(())
        } else {
            Err(sqlx::Error::RowNotFound)
        }
    }

    /// 关闭数据库连接池
    pub async fn close(&self) {
        info!("正在关闭数据库连接池...");
        self.pool.close().await;
        info!("数据库连接池已关闭");
    }
}

/// 全局数据库管理器实例
static mut DB_MANAGER: Option<DatabaseManager> = None;

/// 初始化全局数据库管理器
pub async fn init_database() -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        if DB_MANAGER.is_some() {
            warn!("数据库管理器已经初始化");
            return Ok(());
        }

        let manager = DatabaseManager::from_env().await?;
        manager.migrate().await?;
        manager.health_check().await?;

        DB_MANAGER = Some(manager);
        info!("全局数据库管理器初始化完成");
        Ok(())
    }
}

/// 获取全局数据库管理器
pub fn get_database() -> &'static DatabaseManager {
    unsafe {
        DB_MANAGER.as_ref().expect("数据库管理器未初始化，请先调用 init_database()")
    }
}

/// 便捷函数：获取数据库连接池
pub fn get_db_pool() -> &'static DbPool {
    get_database().pool()
}