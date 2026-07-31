-- session_generation 写侧补全：用户角色变更时自动 bump 世代号。
-- 读侧（get_user_by_token，src/api/auth.rs）早已校验 session_generation 不匹配即逐出缓存，
-- 但此前没有任何写路径 +1 该列，导致"降级/封禁后旧 session 立即失效"事实上从未触发。
-- 应用层无 role 变更路径，故 BEFORE UPDATE 触发器是唯一可靠兜底（手动 SQL 与未来功能都覆盖）。
-- 仅在 role 真正变化时 bump（IS DISTINCT FROM），其它列更新不误伤；
-- BEFORE 触发器内修改 NEW.session_generation 不会递归触发自身（PG 行级行为）。
CREATE OR REPLACE FUNCTION bump_session_generation_on_role_change()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.role IS DISTINCT FROM OLD.role THEN
        NEW.session_generation := OLD.session_generation + 1;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_users_session_generation ON users;
CREATE TRIGGER trg_users_session_generation
    BEFORE UPDATE ON users
    FOR EACH ROW
    EXECUTE FUNCTION bump_session_generation_on_role_change();
