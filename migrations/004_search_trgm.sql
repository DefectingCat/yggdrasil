CREATE EXTENSION IF NOT EXISTS pg_trgm;

ALTER TABLE posts ADD COLUMN IF NOT EXISTS search_text TEXT
    GENERATED ALWAYS AS (
        COALESCE(title, '') || ' ' ||
        COALESCE(summary, '') || ' ' ||
        COALESCE(content_md, '')
    ) STORED;

CREATE INDEX IF NOT EXISTS idx_posts_search_trgm
    ON posts USING GIN (search_text gin_trgm_ops);
