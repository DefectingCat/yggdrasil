use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

#[allow(dead_code)]
pub fn generate_token() -> String {
    Uuid::new_v4().to_string()
}

#[allow(dead_code)]
pub fn default_expiry() -> DateTime<Utc> {
    Utc::now() + Duration::days(30)
}
