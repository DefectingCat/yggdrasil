use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

pub fn generate_token() -> String {
    Uuid::new_v4().to_string()
}

pub fn default_expiry() -> DateTime<Utc> {
    Utc::now() + Duration::days(30)
}

pub fn is_expired(expires_at: DateTime<Utc>) -> bool {
    Utc::now() > expires_at
}
