-- 为 posts 表添加字数与阅读时长列。
--
-- 设计说明：
-- - word_count / reading_time 使用 NOT NULL DEFAULT 0。
-- - 0 作为“尚未计算/回填”的哨兵值：row_to_post_full 在读到 0 时会退回到
--   基于 content_md 的实时计算，保证旧数据在列表页仍显示合理的字数与阅读时长。
-- - 本迁移同时用 PostgreSQL 可用的近似方式回填现有行，避免列表页出现大量 0。
--   回填仅按空白拆分英文词，对中文统计不精确；精确值会在文章被编辑或
--   管理员执行“重建内容”时由 Rust count_words 重新写入。

ALTER TABLE posts
    ADD COLUMN IF NOT EXISTS word_count INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS reading_time INTEGER NOT NULL DEFAULT 0;

-- 回填已存在行：将 content_md 按空白拆分为词数组并计数。
-- array_length 在空/纯空白字符串时可能返回 NULL，用 COALESCE 处理为 1。
WITH computed AS (
    SELECT
        id,
        GREATEST(1, COALESCE(array_length(regexp_split_to_array(content_md, '\s+'), 1), 1)) AS wc
    FROM posts
    WHERE word_count = 0 AND reading_time = 0
)
UPDATE posts
SET
    word_count = computed.wc,
    reading_time = GREATEST(1, computed.wc / 200)
FROM computed
WHERE posts.id = computed.id;
