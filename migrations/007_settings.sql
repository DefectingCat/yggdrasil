-- 回收站与站点配置键值表。
-- 采用简单键值结构而非列式配置，便于后续扩展更多设置项。
CREATE TABLE IF NOT EXISTS settings (
    key         TEXT PRIMARY KEY,
    value       TEXT NOT NULL,
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 回填回收站默认配置：自动清理默认关闭，保留期 30 天。
-- ON CONFLICT 保证重复执行迁移安全。
INSERT INTO settings (key, value) VALUES
    ('trash_auto_purge_enabled', 'false'),
    ('trash_retention_days', '30')
ON CONFLICT (key) DO NOTHING;
