-- 为 posts 表添加字数与阅读时长列。
--
-- 旧数据默认 0，在管理员执行重建或编辑文章后自动回填。
-- 0 也被 row_to_post_full 用作“未计算”标记，触发基于 content_md 的回退计算。

ALTER TABLE posts
    ADD COLUMN IF NOT EXISTS word_count INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS reading_time INTEGER NOT NULL DEFAULT 0;
