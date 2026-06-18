-- 评论内容哈希索引，加速 5 分钟窗口内的重复检测查询。
-- 注意不加 UNIQUE 约束：content_hash 基于 parent_id+author+content，
-- 不同作者发相同内容（如"顶"、"+1"）是合法的，唯一约束会误杀。
CREATE INDEX IF NOT EXISTS idx_comments_content_hash
    ON comments(content_hash);
