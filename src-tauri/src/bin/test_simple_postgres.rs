//! 简单的PostgreSQL连接测试（不使用SQLx宏）

use sqlx::{Row, postgres::PgPoolOptions};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 从环境变量或使用默认值
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:Postgres2025Secure@43.240.223.138:5432/vectordb".to_string());

    println!("正在连接PostgreSQL数据库...");
    println!("数据库URL: {}", database_url);

    // 创建连接池
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    println!("✓ 数据库连接池创建成功");

    // 执行简单查询测试
    let result: i64 = sqlx::query_scalar("SELECT 1")
        .fetch_one(&pool)
        .await?;

    if result == 1 {
        println!("✓ 数据库查询测试通过");
    }

    // 创建UUID扩展
    sqlx::query("CREATE EXTENSION IF NOT EXISTS \"uuid-ossp\"")
        .execute(&pool)
        .await?;
    println!("✓ UUID扩展已启用");

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
            failure_reason TEXT
        )
    "#)
    .execute(&pool)
    .await?;
    println!("✓ tasks表创建成功");

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
            attitudes_count BIGINT DEFAULT 0
        )
    "#)
    .execute(&pool)
    .await?;
    println!("✓ posts表创建成功");

    // 测试表是否存在
    let tasks_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (
           SELECT FROM information_schema.tables
           WHERE table_schema = 'public'
           AND table_name = 'tasks'
         )"
    )
    .fetch_one(&pool)
    .await?;

    let posts_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (
           SELECT FROM information_schema.tables
           WHERE table_schema = 'public'
           AND table_name = 'posts'
         )"
    )
    .fetch_one(&pool)
    .await?;

    if tasks_exists && posts_exists {
        println!("✓ 所有必需的表都已存在并创建成功");
    }

    // 测试插入一条简单的任务记录
    let task_id = uuid::Uuid::new_v4();
    let result = sqlx::query(r#"
        INSERT INTO tasks (id, keyword, event_start_time, status, crawled_count)
        VALUES ($1, $2, NOW(), 'Created', 0)
        RETURNING id
    "#)
    .bind(task_id)
    .bind("test_keyword")
    .fetch_one(&pool)
    .await?;

    println!("✓ 测试任务插入成功，ID: {:?}", result.get::<uuid::Uuid, _>("id"));

    // 关闭连接池
    pool.close().await;
    println!("✓ 数据库连接池已关闭");

    println!("PostgreSQL架构测试和初始化成功完成！");
    Ok(())
}