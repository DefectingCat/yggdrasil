-- 会话世代号：用户角色/封禁状态变更时 bump 此列，使该用户所有已签发 session
-- 立即失效（get_user_by_token 校验世代不匹配则视为未登录）。
-- 默认 0，向后兼容。
ALTER TABLE users ADD COLUMN IF NOT EXISTS session_generation INT NOT NULL DEFAULT 0;

COMMENT ON COLUMN users.session_generation IS '会话世代号，变更时 +1 使旧 session 失效';
