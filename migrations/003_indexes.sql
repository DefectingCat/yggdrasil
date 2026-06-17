-- 补充索引（已在 002_posts.sql 中创建的索引不再重复定义）

-- 用户会话查询
CREATE INDEX IF NOT EXISTS idx_sessions_user_id ON sessions(user_id);
