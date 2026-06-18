//! 用户模型。
//!
//! 定义用户角色、内部用户结构体以及可暴露给前端的 PublicUser。
//! User 包含密码哈希等敏感字段，PublicUser 用于在 API 中隐藏这些字段。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 用户角色枚举。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UserRole {
    /// 管理员，拥有全部后台权限。
    Admin,
    /// 被禁用的用户，无法登录或操作。
    Blocked,
}

impl UserRole {
    /// 将数据库中的角色字符串解析为 UserRole，无法识别时返回 None。
    #[cfg(feature = "server")]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "admin" => Some(UserRole::Admin),
            "blocked" => Some(UserRole::Blocked),
            _ => None,
        }
    }
}

/// 内部使用的完整用户结构体，包含敏感字段。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// 用户主键。
    pub id: i32,
    /// 用户名，用于登录与展示。
    pub username: String,
    /// 邮箱地址。
    pub email: String,
    /// Argon2 密码哈希，不允许直接序列化返回给前端。
    pub password_hash: String,
    /// 用户角色。
    pub role: UserRole,
    /// 账户创建时间。
    pub created_at: DateTime<Utc>,
    /// 会话世代号，角色/封禁变更时 +1 使旧 session 失效。
    pub session_generation: i32,
}

/// 会话缓存使用的轻量用户结构体，不含密码哈希。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionUser {
    /// 用户主键。
    pub id: i32,
    /// 用户名。
    pub username: String,
    /// 邮箱地址。
    pub email: String,
    /// 用户角色。
    pub role: UserRole,
    /// 账户创建时间。
    pub created_at: DateTime<Utc>,
    /// 会话世代号，签发 session 时记录；与 users 表当前值不一致则 session 失效。
    pub session_generation: i32,
}

/// 可公开的用户信息，从 User 转换而来，不含密码哈希。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicUser {
    /// 用户主键。
    pub id: i32,
    /// 用户名。
    pub username: String,
    /// 邮箱地址。
    pub email: String,
    /// 用户角色。
    pub role: UserRole,
    /// 账户创建时间。
    pub created_at: DateTime<Utc>,
}

impl From<User> for SessionUser {
    /// 将 User 转换为 SessionUser，丢弃 password_hash 字段。
    fn from(u: User) -> Self {
        SessionUser {
            id: u.id,
            username: u.username,
            email: u.email,
            role: u.role,
            created_at: u.created_at,
            session_generation: u.session_generation,
        }
    }
}

impl From<SessionUser> for PublicUser {
    /// 将 SessionUser 转换为 PublicUser。
    fn from(u: SessionUser) -> Self {
        PublicUser {
            id: u.id,
            username: u.username,
            email: u.email,
            role: u.role,
            created_at: u.created_at,
        }
    }
}

impl From<User> for PublicUser {
    /// 将 User 转换为 PublicUser，丢弃 password_hash 字段。
    fn from(u: User) -> Self {
        PublicUser {
            id: u.id,
            username: u.username,
            email: u.email,
            role: u.role,
            created_at: u.created_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn sample_user() -> User {
        User {
            id: 1,
            username: "admin".to_string(),
            email: "admin@test.com".to_string(),
            password_hash: "hash".to_string(),
            role: UserRole::Admin,
            created_at: Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
            session_generation: 0,
        }
    }

    #[test]
    #[cfg(feature = "server")]
    fn user_role_from_str() {
        assert_eq!(UserRole::from_str("admin"), Some(UserRole::Admin));
        assert_eq!(UserRole::from_str("blocked"), Some(UserRole::Blocked));
        assert_eq!(UserRole::from_str("unknown"), None);
        assert_eq!(UserRole::from_str(""), None);
    }

    #[test]
    fn user_to_public_user_conversion() {
        let user = sample_user();
        let public: PublicUser = user.clone().into();
        assert_eq!(public.id, user.id);
        assert_eq!(public.username, user.username);
        assert_eq!(public.email, user.email);
        assert_eq!(public.role, user.role);
        assert_eq!(public.created_at, user.created_at);
    }

    #[test]
    fn public_user_excludes_password_hash() {
        let user = sample_user();
        let public: PublicUser = user.into();
        let json = serde_json::to_string(&public).unwrap();
        assert!(!json.contains("password_hash"));
    }

    #[test]
    fn user_role_serde_roundtrip() {
        let json = serde_json::to_string(&UserRole::Admin).unwrap();
        assert_eq!(
            serde_json::from_str::<UserRole>(&json).unwrap(),
            UserRole::Admin
        );
    }

    #[test]
    fn user_to_session_user_excludes_password_hash() {
        let user = sample_user();
        let session: SessionUser = user.clone().into();
        assert_eq!(session.id, user.id);
        assert_eq!(session.username, user.username);
        assert_eq!(session.email, user.email);
        assert_eq!(session.role, user.role);
        assert_eq!(session.created_at, user.created_at);
    }

    #[test]
    fn session_user_to_public_user_excludes_password_hash() {
        let user = sample_user();
        let session: SessionUser = user.into();
        let public: PublicUser = session.into();
        let json = serde_json::to_string(&public).unwrap();
        assert!(!json.contains("password_hash"));
    }
}
