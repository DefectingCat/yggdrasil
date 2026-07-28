-- MCP（Model Context Protocol）服务器的访问令牌。
-- 令牌由管理员在后台签发，绑定用户与作用域（read/write/admin），
-- 静态加密存储（token_enc = AES-GCM 密文），并以 SHA-256 哈希做每请求常量查找。
-- 与 sessions 表一致：明文不入库；但本表支持「管理员重查明文」（解密 token_enc），
-- 故不存裸明文而是可逆密文。expires_at 为 NULL 表示永不过期（签发时显式选择）。

CREATE TABLE IF NOT EXISTS mcp_tokens (
    id            UUID PRIMARY KEY,                       -- 应用层生成（uuid crate）
    user_id       INT  NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name          TEXT NOT NULL,                          -- 管理员自命名，仅展示
    scope         TEXT NOT NULL CHECK (scope IN ('read', 'write', 'admin')),
    -- AES-GCM 密文（nonce ‖ ct ‖ tag）的 hex 编码；管理员可解密重查明文。
    token_enc     TEXT NOT NULL,
    -- 明文 token 的 SHA-256 hex（64 字符），用于每请求 O(1) 常量查找。
    token_hash    CHAR(64) NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at    TIMESTAMPTZ,                            -- NULL = 永不过期（显式选择）
    last_used_at  TIMESTAMPTZ,
    revoked_at    TIMESTAMPTZ                             -- NULL = 有效；撤销后置为 NOW()
);

-- 仅在「未撤销」令牌间保证 token_hash 唯一：撤销后可回收同名 hash（理论碰撞极低）。
CREATE UNIQUE INDEX IF NOT EXISTS mcp_tokens_token_hash_active_idx
    ON mcp_tokens (token_hash) WHERE revoked_at IS NULL;
CREATE INDEX IF NOT EXISTS mcp_tokens_user_idx ON mcp_tokens (user_id);
