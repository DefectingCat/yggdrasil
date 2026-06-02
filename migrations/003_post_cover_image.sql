-- 新增封面图字段
ALTER TABLE posts ADD COLUMN cover_image VARCHAR(500);

-- 为封面图添加索引
CREATE INDEX idx_posts_cover ON posts(cover_image) WHERE cover_image IS NOT NULL;
