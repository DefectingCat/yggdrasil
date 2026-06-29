//! 认证相关的 Dioxus server function 与辅助函数。
//!
//! 提供注册、登录、登出、获取当前用户等接口，
//! 通过 HttpOnly Cookie 维护会话，首个注册用户自动成为 admin。
//! 所有 server function 均在 `#[server(Name, "/api")]` 下注册，供客户端与服务端调用。
//! 仅在 `feature = "server"` 启用的服务端构建中执行数据库操作与 Cookie 写入。

#![allow(clippy::unused_unit, deprecated)]

use dioxus::prelude::*;
#[cfg(feature = "server")]
use http::header::{HeaderValue, SET_COOKIE};

#[cfg(feature = "server")]
use crate::api::error::AppError;
#[cfg(feature = "server")]
use crate::auth::session::get_session_from_ctx;
#[cfg(feature = "server")]
use crate::auth::{password, session};
#[cfg(feature = "server")]
use crate::db::pool::get_conn;
use crate::models::user::PublicUser;
#[cfg(feature = "server")]
use crate::models::user::{SessionUser, UserRole};

#[cfg(feature = "server")]
fn validate_username(username: &str) -> Result<(), String> {
    if username.len() < 3 || username.len() > 50 {
        return Err("用户名长度必须在 3-50 字符之间".to_string());
    }
    if !username.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err("用户名只能包含字母、数字和下划线".to_string());
    }
    Ok(())
}

#[cfg(feature = "server")]
static EMAIL_REGEX: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
    regex::Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap()
});

#[cfg(feature = "server")]
fn validate_email(email: &str) -> Result<(), String> {
    if !EMAIL_REGEX.is_match(email) {
        return Err("邮箱格式不正确".to_string());
    }
    Ok(())
}

#[cfg(feature = "server")]
fn validate_password(password: &str) -> Result<(), String> {
    if password.len() < 8 {
        return Err("密码长度至少 8 位".to_string());
    }
    Ok(())
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
/// 认证接口统一响应结构。
pub struct AuthResponse {
    /// 操作是否成功。
    pub success: bool,
    /// 提示信息。
    pub message: String,
    /// 登录成功后的会话 token（已废弃，实际通过 Cookie 传递）。
    pub token: Option<String>,
}

/// 用户注册。
///
/// 校验用户名、邮箱、密码，首个注册用户自动设为 admin；
/// 已有 admin 时返回 "Registration is closed"。
/// Dioxus server function，注册在 `/api` 路径下。
#[server(Register, "/api")]
pub async fn register(
    username: String,
    email: String,
    password: String,
) -> Result<AuthResponse, ServerFnError> {
    // 服务端构建时先进行严格限流检查。
    #[cfg(feature = "server")]
    {
        if let Some(ctx) = dioxus::fullstack::FullstackContext::current() {
            let parts = ctx.parts_mut();
            let ip = crate::api::rate_limit::get_client_ip(&parts.headers);
            if let Err(msg) = crate::api::rate_limit::check_strict_limit(&ip) {
                return Ok(AuthResponse {
                    success: false,
                    message: msg,
                    token: None,
                });
            }
        }
    }

    if let Err(e) = validate_username(&username) {
        return Ok(AuthResponse {
            success: false,
            message: e,
            token: None,
        });
    }
    if let Err(e) = validate_email(&email) {
        return Ok(AuthResponse {
            success: false,
            message: e,
            token: None,
        });
    }
    if let Err(e) = validate_password(&password) {
        return Ok(AuthResponse {
            success: false,
            message: e,
            token: None,
        });
    }

    let client = get_conn().await.map_err(AppError::db_conn)?;

    // Argon2 是 memory-hard 计算，必须在 spawn_blocking 中执行，避免阻塞 Tokio worker。
    let pw_for_hash = password.clone();
    let password_hash = tokio::task::spawn_blocking(move || password::hash_password(&pw_for_hash))
        .await
        .map_err(|_| AppError::Internal("密码处理任务失败"))?
        .map_err(|_| AppError::Internal("密码处理失败"))?;

    // 使用 INSERT ON CONFLICT 原子性地完成“首个用户成为 admin”的竞争。
    // 若已有 admin 或用户名/邮箱冲突，RETURNING 将返回空。
    let result = client
        .query_opt(
            "INSERT INTO users (username, email, password_hash, role)
             VALUES ($1, $2, $3, 'admin')
             ON CONFLICT DO NOTHING
             RETURNING id",
            &[&username, &email, &password_hash],
        )
        .await
        .map_err(AppError::query)?;

    if result.is_some() {
        return Ok(AuthResponse {
            success: true,
            message: "注册成功".to_string(),
            token: None,
        });
    }

    // 插入失败：区分是已有 admin 还是用户名/邮箱冲突。
    let admin_exists: bool = client
        .query_one(
            "SELECT EXISTS (SELECT 1 FROM users WHERE role = 'admin')",
            &[],
        )
        .await
        .map_err(AppError::query)?
        .get(0);

    let message = if admin_exists {
        "Registration is closed".to_string()
    } else {
        "用户名或邮箱已存在".to_string()
    };

    Ok(AuthResponse {
        success: false,
        message,
        token: None,
    })
}

/// 用户登录。
///
/// 验证用户名/邮箱与密码，生成会话并写入 HttpOnly Cookie；
/// 同一用户活跃会话数超过 `MAX_SESSIONS_PER_USER` 时删除最早会话。
/// Dioxus server function，注册在 `/api` 路径下。
#[server(Login, "/api")]
pub async fn login(username: String, password: String) -> Result<AuthResponse, ServerFnError> {
    // 服务端构建时先进行严格限流检查。
    #[cfg(feature = "server")]
    {
        if let Some(ctx) = dioxus::fullstack::FullstackContext::current() {
            let parts = ctx.parts_mut();
            let ip = crate::api::rate_limit::get_client_ip(&parts.headers);
            if let Err(msg) = crate::api::rate_limit::check_strict_limit(&ip) {
                return Ok(AuthResponse {
                    success: false,
                    message: msg,
                    token: None,
                });
            }
        }
    }

    let mut client = get_conn().await.map_err(AppError::db_conn)?;

    let row = match client
        .query_opt(
            "SELECT id, username, email, password_hash, role, created_at FROM users WHERE username = $1 OR email = $1",
            &[&username],
        )
        .await
    {
        Ok(Some(row)) => row,
        Ok(None) => {
            // 用户不存在时也执行一次 Argon2 verify，抹平「用户不存在」与
            // 「密码错误」的响应时序差，防止通过响应时间枚举账号（L2）。
            // 用固定合法哈希做 verify（必然失败），耗时与真实校验一致。
            const DUMMY_HASH: &str =
                "$argon2id$v=19$m=19456,t=2,p=1$j3rNaAXzdExYaL94WBWtfg$n1S75LUQKaYJwaRl5bkFF/f/N1tLfRYR/7TuQxKP94c";
            let dummy_pw = password.clone();
            let _ = tokio::task::spawn_blocking(move || {
                crate::auth::password::verify_password(&dummy_pw, DUMMY_HASH)
            })
            .await;
            return Ok(AuthResponse {
                success: false,
                message: "Invalid credentials".to_string(),
                token: None,
            });
        }
        Err(e) => {
            return Err(AppError::query(e).into());
        }
    };

    let password_hash: String = row.get("password_hash");
    // Argon2 校验同样在 spawn_blocking 中执行。
    let pw_for_verify = password.clone();
    let hash_for_verify = password_hash.clone();
    let valid = tokio::task::spawn_blocking(move || {
        password::verify_password(&pw_for_verify, &hash_for_verify)
    })
    .await
    .map_err(|_| AppError::Internal("密码处理任务失败"))?
    .map_err(|_| AppError::Internal("密码处理失败"))?;

    if !valid {
        return Ok(AuthResponse {
            success: false,
            message: "Invalid credentials".to_string(),
            token: None,
        });
    }

    let user_id: i32 = row.get("id");
    let token = session::generate_token();
    let token_hash = session::hash_token(&token);
    let expires_at = session::default_expiry();

    let max_sessions = std::env::var("MAX_SESSIONS_PER_USER")
        .ok()
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(5)
        .max(1);

    // 用事务 + 对 users 行加 FOR UPDATE 锁，串行化同一用户的并发登录，
    // 避免 COUNT→DELETE→INSERT 之间的竞态导致超出上限（M1）。
    let tx = client.transaction().await.map_err(AppError::query)?;
    // 锁住该用户行，并发登录在此排队。
    tx.execute("SELECT 1 FROM users WHERE id = $1 FOR UPDATE", &[&user_id])
        .await
        .map_err(AppError::query)?;

    let session_count: i64 = tx
        .query_one(
            "SELECT COUNT(*) FROM sessions WHERE user_id = $1 AND expires_at > NOW()",
            &[&user_id],
        )
        .await
        .map_err(AppError::query)?
        .get(0);

    if session_count >= max_sessions {
        tx.execute(
            "DELETE FROM sessions WHERE id IN (
                SELECT id FROM sessions
                WHERE user_id = $1 AND expires_at > NOW()
                ORDER BY created_at ASC
                LIMIT 1
            )",
            &[&user_id],
        )
        .await
        .map_err(AppError::query)?;
    }

    tx.execute(
        "INSERT INTO sessions (user_id, token_hash, user_agent, expires_at) VALUES ($1, $2, $3, $4)",
        &[&user_id, &token_hash, &None::<String>, &expires_at],
    )
    .await
    .map_err(AppError::query)?;

    tx.commit().await.map_err(AppError::query)?;

    let cookie = session::session_cookie(&token, 30 * 24 * 60 * 60, session::cookie_secure());
    // 通过 Dioxus FullstackContext 设置 HttpOnly Cookie 响应头。
    if let Some(ctx) = dioxus::fullstack::FullstackContext::current() {
        if let Ok(value) = HeaderValue::try_from(cookie.as_str()) {
            ctx.add_response_header(SET_COOKIE, value);
        }
    }

    Ok(AuthResponse {
        success: true,
        message: "登录成功".to_string(),
        token: None,
    })
}

/// 用户登出。
///
/// 清空客户端 session Cookie，并删除数据库中对应会话记录。
/// Dioxus server function，注册在 `/api` 路径下。
#[server(Logout, "/api")]
pub async fn logout() -> Result<AuthResponse, ServerFnError> {
    let token = get_session_from_ctx();

    let client = get_conn().await.map_err(AppError::db_conn)?;

    // 设置过期时间为 0 的 Cookie，通知浏览器清除会话。
    let cookie = session::session_cookie("", 0, session::cookie_secure());
    if let Some(ctx) = dioxus::fullstack::FullstackContext::current() {
        if let Ok(value) = HeaderValue::try_from(cookie.as_str()) {
            ctx.add_response_header(SET_COOKIE, value);
        }
    }

    if let Some(t) = token {
        let token_hash = session::hash_token(&t);
        crate::cache::invalidate_session_user(&token_hash).await;
        client
            .execute("DELETE FROM sessions WHERE token_hash = $1", &[&token_hash])
            .await
            .map_err(AppError::query)?;
    }

    Ok(AuthResponse {
        success: true,
        message: "登出成功".to_string(),
        token: None,
    })
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
/// 当前用户查询响应。
pub struct CurrentUserResponse {
    /// 当前已登录用户的公开信息；未登录时为 `None`。
    pub user: Option<PublicUser>,
}

#[cfg(feature = "server")]
/// 根据会话 token 查询对应用户（不含密码哈希，供会话缓存使用）。
///
/// 优先命中内存缓存，避免每次请求都执行 DB JOIN；未命中时回查数据库并回填缓存。
/// 缓存命中后仍回查 `users.session_generation`：若用户已被降级/封禁（generation 被
/// bump），缓存的旧 SessionUser.generation 不再匹配，此时逐出缓存并视为未登录，
/// 消除权限残留窗口（见 H2）。仅服务端内部使用，不会暴露给前端。
pub async fn get_user_by_token(token: &str) -> Result<Option<SessionUser>, ServerFnError> {
    let token_hash = session::hash_token(token);

    if let Some(cached) = crate::cache::get_session_user(&token_hash).await {
        // 缓存命中后校验世代号：bump 后该用户所有 session 应失效。
        // 查询走主键，亚毫秒级，代价可接受。
        let current_gen: Option<i32> = get_conn()
            .await
            .map_err(AppError::db_conn)?
            .query_opt(
                "SELECT session_generation FROM users WHERE id = $1",
                &[&cached.id],
            )
            .await
            .map_err(AppError::query)?
            .map(|r| r.get::<_, i32>(0));
        match current_gen {
            Some(gen) if gen == cached.session_generation => return Ok(Some(cached)),
            _ => {
                // 世代不匹配或用户已删：逐出缓存，落入下方重新查询。
                crate::cache::invalidate_session_user(&token_hash).await;
            }
        }
    }

    let client = get_conn().await.map_err(AppError::db_conn)?;

    let row = client
        .query_opt(
            "SELECT u.id, u.username, u.email, u.role, u.created_at, u.session_generation
             FROM sessions s
             JOIN users u ON s.user_id = u.id
             WHERE s.token_hash = $1 AND s.expires_at > NOW()",
            &[&token_hash],
        )
        .await
        .map_err(AppError::query)?;

    let user = match row {
        Some(row) => {
            let role_str: String = row.get("role");
            let role = UserRole::from_str(&role_str).unwrap_or(UserRole::Blocked);
            Some(SessionUser {
                id: row.get("id"),
                username: row.get("username"),
                email: row.get("email"),
                role,
                created_at: row.get("created_at"),
                session_generation: row.get("session_generation"),
            })
        }
        None => None,
    };

    if let Some(ref u) = user {
        crate::cache::set_session_user(&token_hash, u.clone()).await;
    }

    Ok(user)
}

#[cfg(feature = "server")]
/// 使指定用户的所有 session 立即失效：bump `session_generation`。
///
/// 用于角色降级、封禁、密码修改等场景。bump 后该用户所有已签发 session 在下次
/// `get_user_by_token` 时因世代不匹配被逐出缓存并视为未登录。内存缓存无需主动清，
/// 惰性逐出即可。当前仓库无运行时角色变更入口，本函数是为未来「用户管理」功能
/// 预备的基础设施，一旦引入降级/封禁的 server function，必须在 UPDATE 后调用。
#[allow(dead_code)] // 预留给未来的用户管理功能（角色变更/封禁触发全量 session 失效）
pub async fn invalidate_user_sessions(user_id: i32) -> Result<(), ServerFnError> {
    let client = get_conn().await.map_err(AppError::db_conn)?;
    client
        .execute(
            "UPDATE users SET session_generation = session_generation + 1 WHERE id = $1",
            &[&user_id],
        )
        .await
        .map_err(AppError::query)?;
    Ok(())
}

/// 获取当前登录用户的公开信息。
///
/// Dioxus server function，注册在 `/api` 路径下。
#[server(GetCurrentUser, "/api")]
pub async fn get_current_user() -> Result<CurrentUserResponse, ServerFnError> {
    let token = match get_session_from_ctx() {
        Some(t) => t,
        None => return Ok(CurrentUserResponse { user: None }),
    };

    let user = get_user_by_token(&token).await?.map(PublicUser::from);

    Ok(CurrentUserResponse { user })
}

#[cfg(feature = "server")]
/// 获取当前登录用户并要求其为 admin，否则返回 401/403。
///
/// 供其它服务端接口内部调用。
pub async fn get_current_admin_user() -> Result<SessionUser, AppError> {
    let token = get_session_from_ctx().ok_or(AppError::Unauthorized("未登录"))?;

    let session_user = get_user_by_token(&token)
        .await
        .map_err(AppError::query)?
        .ok_or(AppError::Unauthorized("会话已过期"))?;

    if session_user.role != UserRole::Admin {
        return Err(AppError::Forbidden("权限不足"));
    }

    Ok(session_user)
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;

    #[test]
    fn validate_username_valid() {
        assert!(validate_username("admin").is_ok());
        assert!(validate_username("user_123").is_ok());
        assert!(validate_username("abc").is_ok());
    }

    #[test]
    fn validate_username_too_short() {
        assert!(validate_username("ab").is_err());
    }

    #[test]
    fn validate_username_too_long() {
        assert!(validate_username(&"a".repeat(51)).is_err());
    }

    #[test]
    fn validate_username_max_length() {
        assert!(validate_username(&"a".repeat(50)).is_ok());
    }

    #[test]
    fn validate_username_special_chars() {
        assert!(validate_username("user name").is_err());
        assert!(validate_username("user@name").is_err());
        assert!(validate_username("user-name").is_err());
    }

    #[test]
    fn validate_username_unicode() {
        assert!(validate_username("用户名").is_ok());
    }

    #[test]
    fn validate_email_valid() {
        assert!(validate_email("user@example.com").is_ok());
        assert!(validate_email("a.b+c@domain.co").is_ok());
    }

    #[test]
    fn validate_email_invalid() {
        assert!(validate_email("notanemail").is_err());
        assert!(validate_email("@domain.com").is_err());
        assert!(validate_email("user@").is_err());
        assert!(validate_email("user@.com").is_err());
        assert!(validate_email("").is_err());
    }

    #[test]
    fn validate_password_valid() {
        assert!(validate_password("12345678").is_ok());
        assert!(validate_password("a very long password with spaces").is_ok());
    }

    #[test]
    fn validate_password_too_short() {
        assert!(validate_password("1234567").is_err());
    }

    #[test]
    fn validate_password_exactly_8() {
        assert!(validate_password("12345678").is_ok());
    }

    #[test]
    fn validate_password_empty() {
        assert!(validate_password("").is_err());
    }
}
