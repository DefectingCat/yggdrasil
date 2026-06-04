#![allow(clippy::unused_unit, deprecated, unused_imports)]

use dioxus::prelude::*;
#[cfg(feature = "server")]
use http::header::{HeaderValue, SET_COOKIE};

use crate::auth::{password, session};
#[cfg(feature = "server")]
use crate::auth::session::get_session_from_ctx;
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

    let client = get_conn().await.map_err(|e| {
        tracing::error!("Register DB connection failed: {:?}", e);
        ServerFnError::new(format!("数据库连接失败: {}", e))
    })?;

    let admin_count: i64 = client
        .query_one("SELECT COUNT(*) FROM users WHERE role = 'admin'", &[])
        .await
        .map_err(|e| {
            tracing::error!("Register admin count query failed: {:?}", e);
            ServerFnError::new(format!("查询失败: {}", e))
        })?
        .get(0);

    if admin_count > 0 {
        return Ok(AuthResponse {
            success: false,
            message: "Registration is closed".to_string(),
            token: None,
        });
    }

    let password_hash = password::hash_password(&password).map_err(|e| {
        tracing::error!("Register password hash failed: {:?}", e);
        ServerFnError::new(format!("密码哈希失败: {}", e))
    })?;

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
    let client = get_conn().await.map_err(|e| {
        tracing::error!("Login DB connection failed: {:?}", e);
        ServerFnError::new(format!("数据库连接失败: {}", e))
    })?;

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
            tracing::error!("Login user query failed: {:?}", e);
            return Err(ServerFnError::new(format!("查询失败: {}", e)));
        }
    };

    let password_hash: String = row.get("password_hash");
    let valid = password::verify_password(&password, &password_hash).map_err(|e| {
        tracing::error!("Login password verify failed: {:?}", e);
        ServerFnError::new(format!("密码验证失败: {}", e))
    })?;

    if !valid {
        return Ok(AuthResponse {
            success: false,
            message: "Invalid credentials".to_string(),
            token: None,
        });
    }

    let user_id: i32 = row.get("id");
    let token = session::generate_token();
    let expires_at = session::default_expiry();

    client
        .execute(
            "INSERT INTO sessions (user_id, token, expires_at) VALUES ($1, $2, $3)",
            &[&user_id, &token, &expires_at],
        )
        .await
        .map_err(|e| {
            tracing::error!("Login session insert failed: {:?}", e);
            ServerFnError::new(format!("创建 session 失败: {}", e))
        })?;

    let cookie = format!(
        "session={token}; HttpOnly; Path=/; Max-Age={}; SameSite=Lax",
        30 * 24 * 60 * 60
    );
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

    let client = get_conn().await.map_err(|e| {
        tracing::error!("Logout DB connection failed: {:?}", e);
        ServerFnError::new(format!("数据库连接失败: {}", e))
    })?;

    // 清除 cookie
    if let Some(ctx) = dioxus::fullstack::FullstackContext::current() {
        ctx.add_response_header(
            SET_COOKIE,
            HeaderValue::from_static("session=; HttpOnly; Path=/; Max-Age=0; SameSite=Lax"),
        );
    }

    // 删除当前 session
    if let Some(t) = token {
        client
            .execute("DELETE FROM sessions WHERE token = $1", &[&t])
            .await
            .map_err(|e| {
                tracing::error!("Logout session delete failed: {:?}", e);
                ServerFnError::new(format!("删除 session 失败: {}", e))
            })?;
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
    let client = get_conn().await.map_err(|e| {
        tracing::error!("GetCurrentUser DB connection failed: {:?}", e);
        ServerFnError::new(format!("数据库连接失败: {}", e))
    })?;

    let row = client
        .query_opt(
            "SELECT u.id, u.username, u.email, u.password_hash, u.role, u.created_at
             FROM sessions s
             JOIN users u ON s.user_id = u.id
             WHERE s.token = $1 AND s.expires_at > NOW()",
            &[&token],
        )
        .await
        .map_err(|e| {
            tracing::error!("GetCurrentUser session query failed: {:?}", e);
            ServerFnError::new(format!("查询失败: {}", e))
        })?;

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

    let user = match get_user_by_token(&token).await? {
        Some(u) => Some(PublicUser::from(u)),
        None => None,
    };

    Ok(CurrentUserResponse { user })
}
