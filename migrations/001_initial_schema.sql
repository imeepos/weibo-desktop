-- 微博关键字增量爬取系统 - PostgreSQL数据库架构
-- 简化版本：移除复杂的时间分片逻辑，使用直接SQL查询

-- 启用UUID扩展
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- 简化的任务表
CREATE TABLE IF NOT EXISTS tasks (
    -- 主键：使用UUID确保全局唯一性
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- 核心业务字段
    keyword VARCHAR(255) NOT NULL,
    event_start_time TIMESTAMP WITH TIME ZONE NOT NULL,

    -- 简化的状态管理（6种状态 -> 5种）
    status VARCHAR(50) NOT NULL DEFAULT 'Created' CHECK (
        status IN ('Created', 'Crawling', 'Completed', 'Paused', 'Failed')
    ),

    -- 时间跟踪字段
    min_post_time TIMESTAMP WITH TIME ZONE,
    max_post_time TIMESTAMP WITH TIME ZONE,

    -- 统计字段
    crawled_count BIGINT DEFAULT 0,

    -- 审计字段
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),

    -- 错误处理
    failure_reason TEXT,

    -- 简化的约束
    CONSTRAINT valid_event_time CHECK (event_start_time <= NOW()),
    CONSTRAINT valid_time_range CHECK (
        min_post_time IS NULL OR
        max_post_time IS NULL OR
        min_post_time <= max_post_time
    ),
    CONSTRAINT valid_failure_reason CHECK (
        status != 'Failed' OR failure_reason IS NOT NULL
    )
);

-- 高效索引
CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status);
CREATE INDEX IF NOT EXISTS idx_tasks_keyword ON tasks(keyword);
CREATE INDEX IF NOT EXISTS idx_tasks_updated_at ON tasks(updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_tasks_event_start_time ON tasks(event_start_time);

-- 自动更新updated_at字段的触发器
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_tasks_updated_at
    BEFORE UPDATE ON tasks
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- 简化的帖子表
CREATE TABLE IF NOT EXISTS posts (
    -- 微博帖子ID作为主键
    id VARCHAR(255) PRIMARY KEY,

    -- 关联任务
    task_id UUID NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,

    -- 帖子内容
    text TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,

    -- 作者信息
    author_uid VARCHAR(255) NOT NULL,
    author_screen_name VARCHAR(255) NOT NULL,

    -- 统计数据
    reposts_count BIGINT DEFAULT 0,
    comments_count BIGINT DEFAULT 0,
    attitudes_count BIGINT DEFAULT 0,

    -- 数据完整性约束
    CONSTRAINT valid_counts CHECK (
        reposts_count >= 0 AND
        comments_count >= 0 AND
        attitudes_count >= 0
    ),
    CONSTRAINT valid_post_time CHECK (created_at <= NOW())
);

-- 优化的索引策略
CREATE INDEX IF NOT EXISTS idx_posts_task_id ON posts(task_id);
CREATE INDEX IF NOT EXISTS idx_posts_created_at ON posts(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_posts_author_uid ON posts(author_uid);
CREATE INDEX IF NOT EXISTS idx_posts_task_created ON posts(task_id, created_at DESC);

-- 可选：Cookies表（如果需要PostgreSQL存储）
CREATE TABLE IF NOT EXISTS cookies (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    uid VARCHAR(255) UNIQUE NOT NULL,
    screen_name VARCHAR(255),
    cookies_data JSONB NOT NULL,
    fetched_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    validated_at TIMESTAMP WITH TIME ZONE,
    expires_at TIMESTAMP WITH TIME ZONE,

    CONSTRAINT valid_validated_time CHECK (validated_at IS NULL OR validated_at <= NOW()),
    CONSTRAINT valid_expires_time CHECK (expires_at IS NULL OR expires_at > NOW())
);

CREATE INDEX IF NOT EXISTS idx_cookies_uid ON cookies(uid);
CREATE INDEX IF NOT EXISTS idx_cookies_expires_at ON cookies(expires_at);

-- 创建用于增量爬取的视图
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
GROUP BY t.id, t.keyword, t.status, t.crawled_count, t.updated_at;

-- 插入示例数据的函数（用于测试）
CREATE OR REPLACE FUNCTION insert_sample_data()
RETURNS void AS $$
DECLARE
    sample_task_id UUID;
BEGIN
    -- 插入示例任务
    INSERT INTO tasks (keyword, event_start_time, status)
    VALUES ('测试关键字', NOW() - INTERVAL '30 days', 'Created')
    RETURNING id INTO sample_task_id;

    -- 插入示例帖子
    INSERT INTO posts (id, task_id, text, created_at, author_uid, author_screen_name)
    VALUES
        ('sample_post_1', sample_task_id, '这是一条测试微博', NOW() - INTERVAL '1 days', '123456', '测试用户1'),
        ('sample_post_2', sample_task_id, '这是另一条测试微博', NOW() - INTERVAL '2 days', '789012', '测试用户2');
END;
$$ LANGUAGE plpgsql;