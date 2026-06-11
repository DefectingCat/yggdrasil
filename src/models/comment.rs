use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CommentStatus {
    Pending,
    Approved,
    Spam,
    Trash,
}

impl CommentStatus {
    pub fn from_str(s: &str) -> Self {
        match s {
            "approved" => Self::Approved,
            "spam" => Self::Spam,
            "trash" => Self::Trash,
            _ => Self::Pending,
        }
    }

    #[allow(dead_code)]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Approved => "approved",
            Self::Spam => "spam",
            Self::Trash => "trash",
        }
    }
}

#[cfg(feature = "server")]
#[allow(dead_code)]
pub struct Comment {
    pub id: i64,
    pub post_id: i32,
    pub parent_id: Option<i64>,
    pub depth: i32,
    pub author_name: String,
    pub author_email: String,
    pub author_url: Option<String>,
    pub content_md: String,
    pub content_html: Option<String>,
    pub content_hash: Option<String>,
    pub status: CommentStatus,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub approved_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PublicComment {
    pub id: i64,
    pub parent_id: Option<i64>,
    pub depth: i32,
    pub author_name: String,
    pub author_url: Option<String>,
    pub avatar_url: String,
    pub content_html: Option<String>,
    pub created_at: String,
    pub created_at_iso: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AdminComment {
    pub id: i64,
    pub post_id: i32,
    pub post_title: String,
    pub post_slug: String,
    pub parent_id: Option<i64>,
    pub depth: i32,
    pub author_name: String,
    pub author_email: String,
    pub author_url: Option<String>,
    pub content_md: String,
    pub status: CommentStatus,
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comment_status_from_str() {
        assert_eq!(CommentStatus::from_str("pending"), CommentStatus::Pending);
        assert_eq!(CommentStatus::from_str("approved"), CommentStatus::Approved);
        assert_eq!(CommentStatus::from_str("spam"), CommentStatus::Spam);
        assert_eq!(CommentStatus::from_str("trash"), CommentStatus::Trash);
    }

    #[test]
    fn comment_status_from_str_unknown_defaults_to_pending() {
        assert_eq!(CommentStatus::from_str("unknown"), CommentStatus::Pending);
        assert_eq!(CommentStatus::from_str(""), CommentStatus::Pending);
    }

    #[test]
    fn comment_status_as_str() {
        assert_eq!(CommentStatus::Pending.as_str(), "pending");
        assert_eq!(CommentStatus::Approved.as_str(), "approved");
        assert_eq!(CommentStatus::Spam.as_str(), "spam");
        assert_eq!(CommentStatus::Trash.as_str(), "trash");
    }

    #[test]
    fn comment_status_serde_roundtrip() {
        let statuses = vec![
            CommentStatus::Pending,
            CommentStatus::Approved,
            CommentStatus::Spam,
            CommentStatus::Trash,
        ];
        for status in statuses {
            let json = serde_json::to_string(&status).unwrap();
            let expected = format!("\"{}\"", status.as_str());
            assert_eq!(json, expected);
            let deserialized: CommentStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, status);
        }
    }

    #[test]
    fn comment_status_deserialize_from_lowercase() {
        let pending: CommentStatus = serde_json::from_str("\"pending\"").unwrap();
        assert_eq!(pending, CommentStatus::Pending);
        let approved: CommentStatus = serde_json::from_str("\"approved\"").unwrap();
        assert_eq!(approved, CommentStatus::Approved);
    }
}
