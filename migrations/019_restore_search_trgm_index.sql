-- 恢复 posts.search_text 上的 trigram GIN 索引（014 以错误理由删除）。
-- 014 的理由是错的：它称"pg_trgm GIN 索引只在前缀模式 'xxx%' 命中、双侧 '%..%' 无法利用"。
-- 事实相反——gin_trgm_ops 的设计目的正是加速 LIKE/ILIKE 包含匹配（'%pat%'）：它从模式抽
-- trigram，用 GIN 反查"含这些 trigram 的行"（见 PG 文档 pg_trgm，LIKE/ILIKE 为标准用例）。
-- 因此 014 删掉了一个本可正常工作的索引，搜索此后无谓地走全表扫。这里按 004 原始定义重建。
-- search_posts (src/api/posts/search.rs) 与 search_published (src/mcp/tools/read.rs) 的
-- search_text ILIKE '%..%' 将自动走该索引。注意：trigram 索引对 <3 字符查询效果有限
-- （不足一个 trigram），可能退回顺序扫；已有 LIMIT 50 + 搜索限流兜底，可接受。
CREATE EXTENSION IF NOT EXISTS pg_trgm;
CREATE INDEX IF NOT EXISTS idx_posts_search_trgm
    ON posts USING GIN (search_text gin_trgm_ops);
