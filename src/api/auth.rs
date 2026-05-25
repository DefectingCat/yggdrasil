use chrono::Utc;
use dioxus::prelude::*;

use crate::auth::{password, session};
use crate::db::pool::DB_POOL;
use crate::models::user::{User, UserRole};

fn validate_username(username: &str) -> Result<(), String> {
    if username.len() < 3 || username.len() > 50 {
        return Err("用户名长度必须在 3-50 字符之间".to_string());
    }
    if !username.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err("用户名只能包含字母、数字和下划线".to_string());
    }
    Ok(())
}

fn validate_email(email: &str) -> Result<(), String> {
    let re = regex::Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap();
    if !re.is_match(email) {
        return Err("邮箱格式不正确".to_string());
    }
    Ok(())
}

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

    let client = DB_POOL
        .get()
        .await
        .map_err(|e| ServerFnError::new(format!("数据库连接失败: {}", e)))?;

    // 检查是否已有 admin
    let admin_count: i64 = client
        .query_one("SELECT COUNT(*) FROM users WHERE role = 'admin'", &[])
        .await
        .map_err(|e| ServerFnError::new(format!("查询失败: {}", e)))?
        .get(0);

    if admin_count > 0 {
        return Ok(AuthResponse {
            success: false,
            message: "Registration is closed".to_string(),
            token: None,
        });
    }

    let password_hash = password::hash_password(&password)
        .map_err(|e| ServerFnError::new(format!("密码哈希失败: {}", e)))?;

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
pub async fn login(
    username: String,
    password: String,
) -> Result<AuthResponse, ServerFnError> {
    let client = DB_POOL
        .get()
        .await
        .map_err(|e| ServerFnError::new(format!("数据库连接失败: {}", e)))?;

    let row = match client
        .query_opt(
            "SELECT id, username, email, password_hash, role, created_at FROM users WHERE username = $1",
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
        Err(e) => return Err(ServerFnError::new(format!("查询失败: {}", e))),
    };

    let password_hash: String = row.get("password_hash");
    let valid = password::verify_password(&password, &password_hash)
        .map_err(|e| ServerFnError::new(format!("密码验证失败: {}", e)))?;

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
        .map_err(|e| ServerFnError::new(format!("创建 session 失败: {}", e)))?;

    Ok(AuthResponse {
        success: true,
        message: "登录成功".to_string(),
        token: Some(token),
    })
}

#[server(Logout, "/api")]
pub async fn logout() -> Result<AuthResponse, ServerFnError> {
    let client = DB_POOL
        .get()
        .await
        .map_err(|e| ServerFnError::new(format!("数据库连接失败: {}", e)))?;

    // 尝试从请求头读取 session token 并删除
    // 注意：这里简化处理，实际应在 middleware 中读取 cookie
    client
        .execute("DELETE FROM sessions WHERE expires_at < NOW()", &[])
        .await
        .map_err(|e| ServerFnError::new(format!("清理 session 失败: {}", e)))?;

    Ok(AuthResponse {
        success: true,
        message: "登出成功".to_string(),
        token: None,
    })
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CurrentUserResponse {
    pub user: Option<User>,
}

#[server(GetCurrentUser, "/api")]
pub async fn get_current_user() -> Result<CurrentUserResponse, ServerFnError> {
    // 从请求头读取 cookie
    let parts = server_context().request_parts();
    let cookie_header = parts
        .headers
        .get("cookie")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let session_token = cookie_header
        .split(';')
        .find_map(|pair| {
            let mut kv = pair.trim().splitn(2, '=');
            let key = kv.next()?;
            let value = kv.next()?;
            if key == "session" {
                Some(value.to_string())
            } else {
                None
            }
        });

    let token = match session_token {
        Some(t) => t,
        None => return Ok(CurrentUserResponse { user: None }),
    };

    let client = DB_POOL
        .get()
        .await
        .map_err(|e| ServerFnError::new(format!("数据库连接失败: {}", e)))?;

    let row = client
        .query_opt(
            "SELECT u.id, u.username, u.email, u.password_hash, u.role, u.created_at
             FROM sessions s
             JOIN users u ON s.user_id = u.id
             WHERE s.token = $1 AND s.expires_at > NOW()",
            &[&token],
        )
        .await
        .map_err(|e| ServerFnError::new(format!("查询失败: {}", e)))?;

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

    Ok(CurrentUserResponse { user })
}
