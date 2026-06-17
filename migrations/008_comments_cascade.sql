-- 将 comments.post_id 外键从 RESTRICT 改为 CASCADE，
-- 使回收站清理/自动清理能够删除仍有评论的文章。

ALTER TABLE comments
    DROP CONSTRAINT IF EXISTS comments_post_id_fkey;

ALTER TABLE comments
    ADD CONSTRAINT comments_post_id_fkey
    FOREIGN KEY (post_id) REFERENCES posts(id) ON DELETE CASCADE;
