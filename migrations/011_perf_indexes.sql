-- 性能审计补充索引。
-- 所有索引均为 CREATE INDEX IF NOT EXISTS，对已有数据库安全幂等。

-- 回收站查询：trash.rs 中大量 WHERE deleted_at IS NOT NULL ... ORDER BY deleted_at DESC。
-- 配合 list_deleted_posts 的 ORDER BY deleted_at DESC 分页。
CREATE INDEX IF NOT EXISTS idx_posts_deleted_at
    ON posts(deleted_at DESC) WHERE deleted_at IS NOT NULL;

-- 管理后台列表 list_posts：WHERE deleted_at IS NULL ORDER BY created_at DESC。
-- 注意 002_posts.sql 已有 idx_posts_status_published（仅 published 分页），
-- 这里补充未按 status 过滤的管理后台全量列表路径。
CREATE INDEX IF NOT EXISTS idx_posts_created_at_admin
    ON posts(created_at DESC) WHERE deleted_at IS NULL;
