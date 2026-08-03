-- 友链表：前台 /friends 卡片页数据源，后台 /admin/friends 管理。
-- 排序用整数 sort_order（越小越靠前），is_active 控制前台可见性。
CREATE TABLE IF NOT EXISTS friend_links (
    id          SERIAL PRIMARY KEY,
    name        TEXT NOT NULL,
    url         TEXT NOT NULL,
    avatar_url  TEXT,
    description TEXT NOT NULL DEFAULT '',
    sort_order  INT  NOT NULL DEFAULT 0,
    is_active   BOOLEAN NOT NULL DEFAULT TRUE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS friend_links_active_sort_idx
    ON friend_links (is_active, sort_order, id);
