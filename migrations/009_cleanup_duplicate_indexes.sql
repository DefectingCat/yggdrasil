-- 清理早期迁移重复创建的索引
-- 这些索引在新数据库中已不会再被创建

DROP INDEX IF EXISTS idx_posts_slug;
DROP INDEX IF EXISTS idx_tags_name;
DROP INDEX IF EXISTS idx_post_tags_tag_id;
