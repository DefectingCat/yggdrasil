use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UserRole {
    Admin,
    Blocked,
}

impl UserRole {
    #[allow(dead_code)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "admin" => Some(UserRole::Admin),
            "blocked" => Some(UserRole::Blocked),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: i32,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub role: UserRole,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicUser {
    pub id: i32,
    pub username: String,
    pub email: String,
    pub role: UserRole,
    pub created_at: DateTime<Utc>,
}

impl From<User> for PublicUser {
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
        }
    }

    #[test]
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
}
