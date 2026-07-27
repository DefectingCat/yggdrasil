-- 上传内容去重：assets.content_hash 存原始上传字节的 SHA-256（hex）。
-- 同一内容重复上传时复用已登记素材（同一行、同一文件），不再重复落盘。
-- 唯一索引是并发去重的正确性基础：两个并发上传同内容时，后到的
-- INSERT ... ON CONFLICT (content_hash) DO NOTHING 落空，转而复用先到的行。
-- 存量行不回填（需读全量文件，代价高）：保持 NULL，不参与去重；
-- PG 唯一索引容许多个 NULL，互不冲突。

ALTER TABLE assets ADD COLUMN IF NOT EXISTS content_hash TEXT;

CREATE UNIQUE INDEX IF NOT EXISTS idx_assets_content_hash ON assets (content_hash);
