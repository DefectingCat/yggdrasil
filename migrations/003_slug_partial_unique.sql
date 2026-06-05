-- 删除全局唯一约束
ALTER TABLE posts DROP CONSTRAINT IF EXISTS posts_slug_unique;

-- 创建部分唯一索引（仅对未删除记录）
CREATE UNIQUE INDEX idx_posts_slug_unique ON posts(slug) WHERE deleted_at IS NULL;
