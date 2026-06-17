-- 补充索引（已在 002_posts.sql 中创建的索引不再重复定义）

-- 标签名称查询
CREATE INDEX IF NOT EXISTS idx_tags_name ON tags(name);

-- 文章标签关联查询（tag 方向）
CREATE INDEX IF NOT EXISTS idx_post_tags_tag_id ON post_tags(tag_id);

-- 用户会话查询
CREATE INDEX IF NOT EXISTS idx_sessions_user_id ON sessions(user_id);
