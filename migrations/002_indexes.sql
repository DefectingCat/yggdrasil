-- 按 slug 查询文章（文章详情页）
CREATE INDEX IF NOT EXISTS idx_posts_slug ON posts(slug) WHERE deleted_at IS NULL;

-- 按状态和时间查询（文章列表、归档）
CREATE INDEX IF NOT EXISTS idx_posts_status_published ON posts(status, deleted_at, published_at DESC);

-- 标签名称查询
CREATE INDEX IF NOT EXISTS idx_tags_name ON tags(name);

-- 文章标签关联查询
CREATE INDEX IF NOT EXISTS idx_post_tags_post_id ON post_tags(post_id);
CREATE INDEX IF NOT EXISTS idx_post_tags_tag_id ON post_tags(tag_id);

-- 用户会话查询
CREATE INDEX IF NOT EXISTS idx_sessions_user_id ON sessions(user_id);
