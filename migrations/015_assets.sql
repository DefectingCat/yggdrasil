-- 素材（图片）注册表与文章引用关联。
-- assets：uploads/ 下每张已登记图片的元数据；磁盘是字节唯一存储，本表是注册表。
-- asset_refs：哪篇文章引用了哪张素材，文章保存时由 sync_asset_refs 全量重建（sync_tags 模式）。
-- id 由应用层生成（uuid crate），SQL 侧不设默认值，避免依赖 PG13+ 的 gen_random_uuid()。

CREATE TABLE IF NOT EXISTS assets (
    id          UUID PRIMARY KEY,
    path        TEXT NOT NULL UNIQUE,        -- 相对路径 "2026/07/24/153000.<uuid>.webp"
    filename    TEXT NOT NULL,               -- 原始文件名（客户端提供，仅展示用）
    mime        TEXT NOT NULL,               -- 落盘后的实际 MIME（转码后可能变为 image/webp）
    size_bytes  BIGINT NOT NULL,
    width       INTEGER NOT NULL,
    height      INTEGER NOT NULL,
    alt         TEXT,                        -- 管理性 alt：仅作默认值/备注，不回写已有文章 HTML
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_assets_created_at ON assets (created_at DESC);

CREATE TABLE IF NOT EXISTS asset_refs (
    asset_id UUID NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
    post_id  INT  NOT NULL REFERENCES posts(id)  ON DELETE CASCADE,
    PRIMARY KEY (asset_id, post_id)
);

CREATE INDEX IF NOT EXISTS idx_asset_refs_post ON asset_refs (post_id);
