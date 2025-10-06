-- PostgreSQL 初始化脚本
-- 微博扫码登录桌面应用数据库初始化
--
-- 说明:
--   1. 此脚本在容器首次启动时自动执行
--   2. 数据库 weibo_desktop 已由环境变量 POSTGRES_DB 创建
--   3. 用户 desktop_user 已由环境变量 POSTGRES_USER 创建

-- ==========================================
-- 1. 创建扩展
-- ==========================================
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";      -- UUID生成
CREATE EXTENSION IF NOT EXISTS "pg_trgm";        -- 文本相似度搜索
CREATE EXTENSION IF NOT EXISTS "btree_gin";      -- GIN索引优化
CREATE EXTENSION IF NOT EXISTS "btree_gist";     -- GiST索引优化

-- ==========================================
-- 2. 创建基础表结构 (示例)
-- ==========================================

-- 用户账户表
CREATE TABLE IF NOT EXISTS accounts (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    weibo_uid VARCHAR(50) UNIQUE NOT NULL,
    nickname VARCHAR(255),
    avatar_url TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    last_login_at TIMESTAMP WITH TIME ZONE,
    status VARCHAR(20) DEFAULT 'active' CHECK (status IN ('active', 'inactive', 'suspended'))
);

-- 登录历史表
CREATE TABLE IF NOT EXISTS login_history (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    account_id UUID REFERENCES accounts(id) ON DELETE CASCADE,
    login_method VARCHAR(50) DEFAULT 'qrcode',
    ip_address INET,
    user_agent TEXT,
    login_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    success BOOLEAN DEFAULT TRUE
);

-- Cookies存储表 (冗余备份,主存储仍在Redis)
CREATE TABLE IF NOT EXISTS cookies_backup (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    account_id UUID REFERENCES accounts(id) ON DELETE CASCADE,
    cookies_data JSONB NOT NULL,
    expires_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    is_valid BOOLEAN DEFAULT TRUE
);

-- ==========================================
-- 3. 创建索引
-- ==========================================
CREATE INDEX IF NOT EXISTS idx_accounts_weibo_uid ON accounts(weibo_uid);
CREATE INDEX IF NOT EXISTS idx_accounts_status ON accounts(status);
CREATE INDEX IF NOT EXISTS idx_accounts_created_at ON accounts(created_at DESC);

CREATE INDEX IF NOT EXISTS idx_login_history_account_id ON login_history(account_id);
CREATE INDEX IF NOT EXISTS idx_login_history_login_at ON login_history(login_at DESC);

CREATE INDEX IF NOT EXISTS idx_cookies_backup_account_id ON cookies_backup(account_id);
CREATE INDEX IF NOT EXISTS idx_cookies_backup_expires_at ON cookies_backup(expires_at);
CREATE INDEX IF NOT EXISTS idx_cookies_backup_is_valid ON cookies_backup(is_valid);

-- ==========================================
-- 4. 创建触发器 - 自动更新 updated_at
-- ==========================================
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER update_accounts_updated_at
BEFORE UPDATE ON accounts
FOR EACH ROW
EXECUTE FUNCTION update_updated_at_column();

-- ==========================================
-- 5. 权限配置
-- ==========================================
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO desktop_user;
GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO desktop_user;
GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA public TO desktop_user;

-- ==========================================
-- 6. 初始化完成标记
-- ==========================================
CREATE TABLE IF NOT EXISTS schema_migrations (
    version VARCHAR(50) PRIMARY KEY,
    applied_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    description TEXT
);

INSERT INTO schema_migrations (version, description)
VALUES ('001', '初始化数据库结构')
ON CONFLICT (version) DO NOTHING;

-- 输出初始化信息
DO $$
BEGIN
    RAISE NOTICE '===========================================';
    RAISE NOTICE 'PostgreSQL 数据库初始化完成';
    RAISE NOTICE '数据库: weibo_desktop';
    RAISE NOTICE '用户: desktop_user';
    RAISE NOTICE '表数量: %', (SELECT count(*) FROM information_schema.tables WHERE table_schema = 'public');
    RAISE NOTICE '===========================================';
END $$;
