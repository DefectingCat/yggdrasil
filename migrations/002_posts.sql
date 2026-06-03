CREATE TABLE posts (
    id           SERIAL PRIMARY KEY,
    author_id    INT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,

    title        VARCHAR(200) NOT NULL,
    slug         VARCHAR(200) NOT NULL,
    summary      VARCHAR(500),

    content_md   TEXT NOT NULL,
    content_html TEXT,
    cover_image  VARCHAR(500),

    status       TEXT NOT NULL DEFAULT 'draft',
    published_at TIMESTAMPTZ,

    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at   TIMESTAMPTZ,

    CONSTRAINT posts_slug_unique UNIQUE (slug),
    CONSTRAINT posts_status_check CHECK (status IN ('draft', 'published'))
);

CREATE INDEX idx_posts_status_published ON posts(status, published_at DESC) WHERE deleted_at IS NULL;
CREATE INDEX idx_posts_slug ON posts(slug) WHERE deleted_at IS NULL;

CREATE TABLE tags (
    id   SERIAL PRIMARY KEY,
    name VARCHAR(50) UNIQUE NOT NULL
);

CREATE TABLE post_tags (
    post_id INT NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    tag_id  INT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (post_id, tag_id)
);

CREATE INDEX idx_post_tags_post ON post_tags(post_id);
CREATE INDEX idx_post_tags_tag ON post_tags(tag_id);

-- 为封面图添加索引
CREATE INDEX idx_posts_cover ON posts(cover_image) WHERE cover_image IS NOT NULL;