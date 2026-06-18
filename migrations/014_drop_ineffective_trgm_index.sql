-- 删除对 ILIKE '%...%'（双侧通配符）无效的 trgm GIN 索引（L1）。
-- pg_trgm 的 GIN 索引只在前缀模式（'xxx%'）命中，双侧模糊匹配无法利用它，
-- 索引建了等于白建且误导。搜索改由 LIMIT + 限流兜底的全表扫承担；
-- 后续可升级为 tsvector 全文检索（独立大改动）。
DROP INDEX IF EXISTS idx_posts_search_trgm;
