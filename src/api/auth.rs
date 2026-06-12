#![allow(clippy::unused_unit, deprecated)]

use dioxus::prelude::*;
#[cfg(feature = "server")]
use http::header::{HeaderValue, SET_COOKIE};

#[cfg(feature = "server")]
use crate::api::error::AppError;
#[cfg(feature = "server")]
use crate::auth::session::get_session_from_ctx;
use crate::auth::{password, session};
use crate::db::pool::get_conn;
use crate::models::user::{PublicUser, User, UserRole};

#[allow(dead_code)]
fn validate_username(username: &str) -> Result<(), String> {
    if username.len() < 3 || username.len() > 50 {
        return Err("用户名长度必须在 3-50 字符之间".to_string());
    }
    if !username.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err("用户名只能包含字母、数字和下划线".to_string());
    }
    Ok(())
}

#[allow(dead_code)]
fn validate_email(email: &str) -> Result<(), String> {
    let re = regex::Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap();
    if !re.is_match(email) {
        return Err("邮箱格式不正确".to_string());
    }
    Ok(())
}

#[allow(dead_code)]
fn validate_password(password: &str) -> Result<(), String> {
    if password.len() < 8 {
        return Err("密码长度至少 8 位".to_string());
    }
    Ok(())
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuthResponse {
    pub success: bool,
    pub message: String,
    pub token: Option<String>,
}

#[server(Register, "/api")]
pub async fn register(
    username: String,
    email: String,
    password: String,
) -> Result<AuthResponse, ServerFnError> {
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

    let admin_count: i64 = client
        .query_one("SELECT COUNT(*) FROM users WHERE role = 'admin'", &[])
        .await
        .map_err(AppError::query)?
        .get(0);

    if admin_count > 0 {
        return Ok(AuthResponse {
            success: false,
            message: "Registration is closed".to_string(),
            token: None,
        });
    }

    let password_hash =
        password::hash_password(&password).map_err(|_| AppError::Internal("密码处理失败"))?;

    let result = client
        .query_one(
            "INSERT INTO users (username, email, password_hash, role) VALUES ($1, $2, $3, 'admin') RETURNING id",
            &[&username, &email, &password_hash],
        )
        .await;

    match result {
        Ok(_) => Ok(AuthResponse {
            success: true,
            message: "注册成功".to_string(),
            token: None,
        }),
        Err(e) => {
            let msg = if e.to_string().contains("unique constraint") {
                "用户名或邮箱已存在".to_string()
            } else {
                format!("注册失败: {}", e)
            };
            Ok(AuthResponse {
                success: false,
                message: msg,
                token: None,
            })
        }
    }
}

#[server(Login, "/api")]
pub async fn login(username: String, password: String) -> Result<AuthResponse, ServerFnError> {
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

    let client = get_conn().await.map_err(AppError::db_conn)?;

    let row = match client
        .query_opt(
            "SELECT id, username, email, password_hash, role, created_at FROM users WHERE username = $1 OR email = $1",
            &[&username],
        )
        .await
    {
        Ok(Some(row)) => row,
        Ok(None) => {
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
    let valid = password::verify_password(&password, &password_hash)
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

    let session_count: i64 = client
        .query_one(
            "SELECT COUNT(*) FROM sessions WHERE user_id = $1 AND expires_at > NOW()",
            &[&user_id],
        )
        .await
        .map_err(AppError::query)?
        .get(0);

    if session_count >= max_sessions {
        client
            .execute(
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

    client
        .execute(
            "INSERT INTO sessions (user_id, token_hash, user_agent, expires_at) VALUES ($1, $2, $3, $4)",
            &[&user_id, &token_hash, &None::<String>, &expires_at],
        )
        .await
        .map_err(AppError::query)?;

    let cookie = session::session_cookie(&token, 30 * 24 * 60 * 60, session::cookie_secure());
    if let Some(ctx) = dioxus::fullstack::FullstackContext::current() {
        if let Ok(value) = HeaderValue::try_from(cookie.as_str()) {
            ctx.add_response_header(SET_COOKIE, value);
        }
    }

    Ok(AuthResponse {
        success: true,
        message: "登录成功".to_string(),
        token: Some(token),
    })
}

#[server(Logout, "/api")]
pub async fn logout() -> Result<AuthResponse, ServerFnError> {
    let token = get_session_from_ctx();

    let client = get_conn().await.map_err(AppError::db_conn)?;

    let cookie = session::session_cookie("", 0, session::cookie_secure());
    if let Some(ctx) = dioxus::fullstack::FullstackContext::current() {
        if let Ok(value) = HeaderValue::try_from(cookie.as_str()) {
            ctx.add_response_header(SET_COOKIE, value);
        }
    }

    if let Some(t) = token {
        let token_hash = session::hash_token(&t);
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
pub struct CurrentUserResponse {
    pub user: Option<PublicUser>,
}

#[cfg(feature = "server")]
pub async fn get_user_by_token(token: &str) -> Result<Option<User>, ServerFnError> {
    let client = get_conn().await.map_err(AppError::db_conn)?;

    let token_hash = session::hash_token(token);
    let row = client
        .query_opt(
            "SELECT u.id, u.username, u.email, u.password_hash, u.role, u.created_at
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
            Some(User {
                id: row.get("id"),
                username: row.get("username"),
                email: row.get("email"),
                password_hash: row.get("password_hash"),
                role,
                created_at: row.get("created_at"),
            })
        }
        None => None,
    };

    Ok(user)
}

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
pub async fn get_current_admin_user() -> Result<User, AppError> {
    let token = get_session_from_ctx().ok_or(AppError::Unauthorized("未登录"))?;

    let user = get_user_by_token(&token)
        .await
        .map_err(AppError::query)?
        .ok_or(AppError::Unauthorized("会话已过期"))?;

    if user.role != UserRole::Admin {
        return Err(AppError::Forbidden("权限不足"));
    }

    Ok(user)
}

#[cfg(test)]
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
