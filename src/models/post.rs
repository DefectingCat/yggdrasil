use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PostStatus {
    Draft,
    Published,
}

impl PostStatus {
    #[allow(dead_code)]
    pub fn as_str(&self) -> &'static str {
        match self {
            PostStatus::Draft => "draft",
            PostStatus::Published => "published",
        }
    }

    #[allow(dead_code)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "draft" => Some(PostStatus::Draft),
            "published" => Some(PostStatus::Published),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Post {
    pub id: i32,
    pub author_id: i32,
    pub title: String,
    pub slug: String,
    pub summary: Option<String>,
    pub content_md: String,
    pub content_html: Option<String>,
    pub status: PostStatus,
    pub published_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub tags: Vec<String>,
    pub cover_image: Option<String>,
    pub reading_time: u32,
    pub word_count: u32,
    pub toc_html: Option<String>,
    pub prev_post: Option<PostNav>,
    pub next_post: Option<PostNav>,
}

impl Post {
    pub fn formatted_date(&self) -> String {
        self.published_at
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| self.created_at.format("%Y-%m-%d").to_string())
    }

    pub fn status_label(&self) -> &'static str {
        match self.status {
            PostStatus::Published => "已发布",
            PostStatus::Draft => "草稿",
        }
    }

    pub fn status_class(&self) -> &'static str {
        match self.status {
            PostStatus::Published => "text-green-600 dark:text-green-400",
            PostStatus::Draft => "text-gray-400 dark:text-[#9b9c9d]",
        }
    }

    pub fn status_badge_class(&self) -> &'static str {
        match self.status {
            PostStatus::Published => {
                "bg-green-100 dark:bg-green-900/30 text-green-700 dark:text-green-300"
            }
            PostStatus::Draft => "bg-gray-100 dark:bg-[#333] text-gray-600 dark:text-[#9b9c9d]",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PostNav {
    pub title: String,
    pub slug: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tag {
    pub id: i32,
    pub name: String,
    pub post_count: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PostStats {
    pub total: i64,
    pub drafts: i64,
    pub published: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn sample_post() -> Post {
        Post {
            id: 1,
            author_id: 1,
            title: "Test".to_string(),
            slug: "test".to_string(),
            summary: None,
            content_md: "content".to_string(),
            content_html: None,
            status: PostStatus::Draft,
            published_at: None,
            created_at: Utc.with_ymd_and_hms(2024, 1, 15, 10, 0, 0).unwrap(),
            updated_at: Utc.with_ymd_and_hms(2024, 1, 15, 10, 0, 0).unwrap(),
            tags: vec![],
            cover_image: None,
            reading_time: 1,
            word_count: 10,
            toc_html: None,
            prev_post: None,
            next_post: None,
        }
    }

    #[test]
    fn post_status_from_str() {
        assert_eq!(PostStatus::from_str("draft"), Some(PostStatus::Draft));
        assert_eq!(
            PostStatus::from_str("published"),
            Some(PostStatus::Published)
        );
        assert_eq!(PostStatus::from_str("unknown"), None);
        assert_eq!(PostStatus::from_str(""), None);
    }

    #[test]
    fn post_status_as_str() {
        assert_eq!(PostStatus::Draft.as_str(), "draft");
        assert_eq!(PostStatus::Published.as_str(), "published");
    }

    #[test]
    fn post_status_roundtrip() {
        for status in [PostStatus::Draft, PostStatus::Published] {
            assert_eq!(PostStatus::from_str(status.as_str()), Some(status.clone()));
        }
    }

    #[test]
    fn formatted_date_uses_published_at_when_available() {
        let mut post = sample_post();
        post.published_at = Some(Utc.with_ymd_and_hms(2024, 6, 1, 12, 0, 0).unwrap());
        assert_eq!(post.formatted_date(), "2024-06-01");
    }

    #[test]
    fn formatted_date_falls_back_to_created_at() {
        let post = sample_post();
        assert_eq!(post.formatted_date(), "2024-01-15");
    }

    #[test]
    fn status_label() {
        let mut post = sample_post();
        post.status = PostStatus::Published;
        assert_eq!(post.status_label(), "已发布");
        post.status = PostStatus::Draft;
        assert_eq!(post.status_label(), "草稿");
    }

    #[test]
    fn status_class_returns_non_empty() {
        let mut post = sample_post();
        post.status = PostStatus::Published;
        assert!(!post.status_class().is_empty());
        post.status = PostStatus::Draft;
        assert!(!post.status_class().is_empty());
    }

    #[test]
    fn status_badge_class_returns_non_empty() {
        let post = sample_post();
        assert!(!post.status_badge_class().is_empty());
    }

    #[test]
    fn post_status_serde_roundtrip() {
        let json = serde_json::to_string(&PostStatus::Draft).unwrap();
        assert_eq!(
            serde_json::from_str::<PostStatus>(&json).unwrap(),
            PostStatus::Draft
        );
    }
}
